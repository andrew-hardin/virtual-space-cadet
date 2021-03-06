use evdev_rs as evdev;
use json;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::fs::File;
use std::io::Read;
use crate::keys;
use crate::parser::ParsedKeyTree;


/// A MxN matrix of key codes. None can be used to encode matrix
/// positions without keys.
pub type KeyMatrix = Vec<Vec<Option<keys::SimpleKey>>>;

/// Simple (row, column) index.
pub type Index2D = (usize, usize);

/// A key can undergo three state changes: `Pressed`, `Released`, or `Held`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum KeyStateChange {
    Released = 0,
    Pressed = 1,
    Held = 2,
}

/// Shorthand for converting KeyStateChange into libevdev/uniput
/// compatible values.
impl Into<KeyStateChange> for i32 {
    fn into(self) -> KeyStateChange {
        match self {
            0 => KeyStateChange::Released,
            1 => KeyStateChange::Pressed,
            2 => KeyStateChange::Held,
            _ => panic!()
        }
    }
}

/// Plain-old-data struct that stores counts for each key state change
/// (e.g. 32 key presses).
#[derive(Copy, Clone)]
pub struct KeyStats {
    values: [u32; 3]
}

impl KeyStats {
    /// Create new key statistics initialized to zero.
    pub fn new() -> KeyStats {
        KeyStats {
            values: [0, 0, 0]
        }
    }
    /// Increment the number of times a particular state change was seen.
    pub fn increment(&mut self, v: KeyStateChange) {
        self.values[v as usize] += 1;
    }

    /// Get the number of times a state change was seen.
    pub fn get(&self, v: KeyStateChange) -> u32 {
        self.values[v as usize]
    }

    /// Set the number of times a state change was seen.
    pub fn set(&mut self, v: KeyStateChange, i: u32) {
        self.values[v as usize] = i;
    }
}

#[derive(Copy, Clone)]
pub struct BlockedKeyStates {
    blocked: [bool; 3]
}

impl BlockedKeyStates {
    /// Create a new block that doesn't block any key state changes.
    pub fn new() -> BlockedKeyStates {
        BlockedKeyStates {
            blocked: [false; 3]
        }
    }
    /// Create a new block for key releases and holds.
    pub fn new_block_release_and_hold() -> BlockedKeyStates {
        let mut ans = BlockedKeyStates::new();
        ans.blocked[KeyStateChange::Released as usize] = true;
        ans.blocked[KeyStateChange::Held as usize] = true;
        ans
    }

    /// Returns `true` if a key state change is blocked.
    fn check_if_blocked(&mut self, s: KeyStateChange) -> bool {
        if s == KeyStateChange::Held && self.blocked[s as usize] {
            // Multiple holds are blocked.
            true
        } else if self.blocked[s as usize] {
            // Press or releases are only blocked once
            // before all blocks are turned off.
            self.unblock();
            true
        } else {
            false
        }
    }

    /// Unblock all key state changes.
    fn unblock(&mut self) {
        self.blocked = [false; 3];
    }
}

/// Return states from after updating `VirtualKeyboardMatrix`.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MatrixUpdateResult {
    /// The given event wasn't within the matrix.
    Bypass,
    /// The event was redundant (e.g. pressing a pressed key).
    Redundant(Index2D),
    /// The event caused a state to change at the given index.
    StateChanged(Index2D, KeyStateChange),
    /// The event caused a state change, but it was blocked.
    Blocked
}

/// Transforms discrete events (e.g. `KEY_A` was pressed)
/// into a classic NxM state matrix.
///
/// This is a workhorse structure that bridges the gap between
/// physical events (e.g. the user pressing a key) and our internal
/// representation of the keyboard state -- an MxN matrix.
pub struct VirtualKeyboardMatrix {
    key_to_index: HashMap<evdev::enums::EV_KEY, Index2D>,
    dim: Index2D,
    state: StateMatrix,
    blocked: Vec<Vec<BlockedKeyStates>>,
    hold_down_threshold: Duration
}

impl VirtualKeyboardMatrix {
    /// Create a new virtual keyboard matrix by specifying which keys are
    /// at which positions in the matrix.
    ///
    /// For example, `KEY_A` is at row 3, column 2.
    pub fn new(keys: KeyMatrix, hold_duration: Option<Duration>) -> VirtualKeyboardMatrix {

        // Loop through the event matrix and store a map from event -> index.
        let row_count = keys.len();
        let col_count = keys.iter().map(|x| x.len()).max().unwrap_or(0);
        let dim = (row_count, col_count);
        let mut hash = HashMap::new();
        for r in 0..dim.0 {
            for c in 0..keys[r].len() {
                match &keys[r][c] {
                    Some(t) => hash.insert(t.clone(), (r, c)),
                    None => None
                    // This (row,col) in the virtual keyboard matrix doesn't have
                    // a key assigned. It's therefore impossible for this matrix
                    // location to ever be pressed (true) or released (false).
                };
            }
        }

        let hold = match hold_duration {
            Some(v) => v,
            None => VirtualKeyboardMatrix::default_hold_duration()
        };

        VirtualKeyboardMatrix {
            key_to_index: hash,
            dim,
            state: StateMatrix::new(dim),
            blocked: vec![vec![BlockedKeyStates::new(); dim.1]; dim.0],
            hold_down_threshold: hold,
        }
    }

    /// Load a keyboard matrix from a file.
    pub fn load(path: &str) -> VirtualKeyboardMatrix {

        // Read the file to a string.
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        // Deserialize and print Rust data structure.
        let data = json::parse(&contents).unwrap();

        let mut mat = KeyMatrix::new();
        for rows in data["matrix"].members() {
            mat.push(Vec::new());
            for col in rows.members() {
                let key_code = col.as_str().unwrap();
                let tokenized = ParsedKeyTree::create(key_code).unwrap();

                // Try to parse it as a NormalKey.
                // If that fails, try a transparent key.
                // If both options fail then error.
                let as_normal = keys::NormalKey::from_tokens(&tokenized);
                let as_transparent = keys::TransparentKey::from_tokens(&tokenized);
                let code =
                    if as_normal.is_ok() {
                        Some(as_normal.unwrap().value)
                    } else if as_transparent.is_ok() {
                        None
                    } else {
                        panic!(format!("Couldn't convert {} into a code.", key_code));
                    };

                mat.last_mut().unwrap().push(code);
            }
        }

        VirtualKeyboardMatrix::new(mat, None)
    }

    /// Get the default duration that a key must be held to generate a HOLD event.
    pub fn default_hold_duration() -> Duration { Duration::from_millis(200) }

    /// Get the dimensions of the matrix.
    pub fn dim(&self) -> Index2D {
        self.dim
    }

    /// Block key events at the given index.
    pub fn set_block(&mut self, block: BlockedKeyStates, idx: Index2D) {
        self.blocked[idx.0][idx.1] = block;
    }

    /// Update the matrix using an input event (e.g. `KEY_A` was pressed).
    /// Events can be blocked (see `set_block`).
    pub fn update(&mut self, event: evdev::InputEvent, now: Instant) -> MatrixUpdateResult {
        // Update the matrix, then check if the event is blocked.
        let ans = self.update_without_blocking(event, now);
        self.check_blocking(ans)
    }

    /// Update the matrix state by processing a single event.
    fn update_without_blocking(&mut self, event: evdev::InputEvent, now: Instant) -> MatrixUpdateResult {

        // Filter based on the event code.
        // We only support EV_KEY events than are also in our matrix.
        // (That's why there's a nested match expression).
        match event.event_code {
            evdev::enums::EventCode::EV_KEY(which) => {
                let location = self.key_to_index.get(&which);
                match location {
                    Some(index) => {
                        // Great, the key corresponds to an index.
                        // Code the state is either pressed or not, then return true because
                        // the key was mapped to a matrix position.
                        let is_pressed = match event.value {
                            0 => false,
                            _ => true
                        };
                        match self.state.set(*index, is_pressed, now) {
                            Some(t) => MatrixUpdateResult::StateChanged(*index, t),
                            None => MatrixUpdateResult::Redundant(*index)
                        }
                    }
                    // Keys that aren't part of the matrix aren't handled.
                    None => MatrixUpdateResult::Bypass
                }
            }
            // Non key event codes aren't handled.
            _ => MatrixUpdateResult::Bypass
        }
    }

    /// Detect any keys that have been held longer than the specified duration.
    /// Holds can be blocked (see `set_block`).
    ///
    /// Returns a vector of positions where keys have been held.
    pub fn detect_held_keys(&mut self, now: Instant) -> Vec<Index2D> {
        self.detect_held_keys_without_blocking(now).iter()
            .filter(|x| {
                // Drop keys that are blocked (i.e. keep keys that aren't blocked).
                !self.blocked[x.0][x.1].check_if_blocked(KeyStateChange::Held)
            })
            .cloned()
            .collect()
    }

    fn detect_held_keys_without_blocking(&mut self, now: Instant) -> Vec<Index2D> {
        // Loop through every cell in the matrix and detect keys that
        // have been held for longer than the given threshold.
        let mut ans = Vec::new();
        for r in 0..self.dim.0 {
            for c in 0..self.dim.1 {
                let idx = (r, c);
                if self.state.is_held(idx, self.hold_down_threshold, now) {
                    ans.push(idx);
                    self.state.reset_key_press_time(idx,now);
                }
            }
        }
        ans
    }

    fn check_blocking(&mut self, item: MatrixUpdateResult) -> MatrixUpdateResult {
        match item {
            MatrixUpdateResult::StateChanged(idx, state) => {
                if self.blocked[idx.0][idx.1].check_if_blocked(state) {
                    MatrixUpdateResult::Blocked
                } else {
                    item
                }
            }
            _ => item
        }
    }
}

/// A 2D matrix that records where and when key presses occurred.
struct StateMatrix {
    state: Vec<Vec<bool>>,
    last_pressed: Vec<Vec<Instant>>,
}

impl StateMatrix {
    /// Create a new state NxM state matrix with every key unpressed.
    pub fn new(dim: Index2D) -> StateMatrix {
        let pressed_ages_ago = Instant::now() - Duration::from_secs(60 * 60);
        StateMatrix {
            state: vec![vec![false; dim.1]; dim.0],
            last_pressed: vec![vec![pressed_ages_ago; dim.1]; dim.0],
        }
    }

    /// Set a key's binary state (i.e. is it pressed or not).
    ///
    /// If the key's state changed (e.g. pressed -> released), then
    /// Some() is returned.
    pub fn set(&mut self, dim: Index2D, is_pressed: bool, now: Instant) -> Option<KeyStateChange> {
        let old_state = self.state[dim.0][dim.1];
        let new_state = is_pressed;
        self.state[dim.0][dim.1] = new_state;
        if old_state != new_state {
            if old_state {
                Some(KeyStateChange::Released)
            } else {
                // Before returning, record the time that the key was pressed.
                self.last_pressed[dim.0][dim.1] = now;
                Some(KeyStateChange::Pressed)
            }
        } else {
            None
        }
    }

    /// Check if a key at the given index has been held longer than a specified duration.
    pub fn is_held(&self, idx: Index2D, hold_threshold: Duration, now: Instant) -> bool {
        let is_pressed = self.state[idx.0][idx.1];
        let held_long_enough = self.last_pressed[idx.0][idx.1] + hold_threshold <= now;
        is_pressed && held_long_enough
    }

    /// Reset the timestamp for when a key was last pressed.
    pub fn reset_key_press_time(&mut self, idx: Index2D, when: Instant) {
        self.last_pressed[idx.0][idx.1] = when;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_matrix_set() {
        let mut m = StateMatrix::new((3, 5));

        // Check the rules that govern whether or not a set yields something.
        assert!(m.set((0, 0), false, Instant::now()).is_none());
        assert_eq!(m.set((0, 0), true, Instant::now()).unwrap(), KeyStateChange::Pressed);
        assert!(m.set((0, 0), true, Instant::now()).is_none());
        assert_eq!(m.set((0, 0), false, Instant::now()).unwrap(), KeyStateChange::Released);
    }

    #[test]
    fn state_matrix_check_held_keys() {

        let mut m = StateMatrix::new((3, 5));
        let reference = Instant::now();
        let half_hold = Duration::from_millis(100);
        let hold = Duration::from_millis(200);
        let hold2 = Duration::from_millis(200 * 2);

        // A key that was pressed + released shouldn't register as a hold.
        m.set((0,0), true, reference);
        m.set((0,0), false, reference + hold);
        assert!(!m.is_held((0, 0), hold, reference));
        assert!(!m.is_held((0, 0), hold, reference + half_hold));
        assert!(!m.is_held((0, 0), hold, reference + hold2));

        // A key that's pressed and held should register as a hold.
        let pt = (1, 1);
        m.set(pt, true, reference);
        assert!(!m.is_held(pt, hold, reference + half_hold));
        assert!(m.is_held(pt, hold, reference + hold));
        assert!(m.is_held(pt, hold, reference + hold2));

        // Release the key, and check that it no longer registers as a hold.
        m.set(pt, false, reference + hold2);
        assert!(!m.is_held(pt, hold, reference + hold2 + half_hold));
    }

    #[test]
    fn key_state_change_conversion() {
        assert_eq!(KeyStateChange::Pressed, 1.into());
        assert_eq!(KeyStateChange::Released, 0.into());
        assert_eq!(KeyStateChange::Held, 2.into());
    }

    #[test]
    fn block_key_states() {
        let mut block = BlockedKeyStates::new_block_release_and_hold();

        // Holds are blocked many times.
        assert!(block.check_if_blocked(KeyStateChange::Held));
        assert!(block.check_if_blocked(KeyStateChange::Held));

        // Presses aren't held.
        assert!(!block.check_if_blocked(KeyStateChange::Pressed));

        // Release is blocked once, then all other blocks are disabled.
        assert!(block.check_if_blocked(KeyStateChange::Released));
        assert!(!block.check_if_blocked(KeyStateChange::Released));
        assert!(!block.check_if_blocked(KeyStateChange::Held));
        assert!(!block.check_if_blocked(KeyStateChange::Pressed));
    }

    fn get_simple_matrix() -> VirtualKeyboardMatrix {
        VirtualKeyboardMatrix::new(vec![
            vec![Some(keys::SimpleKey::KEY_4), Some(keys::SimpleKey::KEY_5), None],
            vec![Some(keys::SimpleKey::KEY_1), Some(keys::SimpleKey::KEY_2), Some(keys::SimpleKey::KEY_3)]
        ], None)
    }

    // Short-hand for checking if v is a specific enum variant.
    macro_rules! is_enum_variant {
        ($v:expr, $p:pat) => (
            if let $p = $v { true } else { false }
        );
    }

    #[test]
    fn virtual_keyboard_matrix_update() {

        // Create a simple matrix, then setup a few press/release events
        // that tests will use.
        let mut mat = get_simple_matrix();
        let press_9 : evdev::InputEvent = keys::KeyState(keys::SimpleKey::KEY_9, KeyStateChange::Pressed).into();
        let press_1 : evdev::InputEvent = keys::KeyState(keys::SimpleKey::KEY_1, KeyStateChange::Pressed).into();
        let release_1 : evdev::InputEvent = keys::KeyState(keys::SimpleKey::KEY_1, KeyStateChange::Released).into();
        let t = Instant::now();

        // Update with an event that's not part of the matrix.
        assert!(is_enum_variant!(mat.update(press_9, t), MatrixUpdateResult::Bypass));

        // Send a release event, even though the key is already released.
        assert!(is_enum_variant!(mat.update(release_1.clone(), t), MatrixUpdateResult::Redundant((1, 0))));

        // Press and release.
        assert!(is_enum_variant!(mat.update(press_1.clone(), t), MatrixUpdateResult::StateChanged((1, 0), KeyStateChange::Pressed)));
        assert!(is_enum_variant!(mat.update(release_1.clone(), t), MatrixUpdateResult::StateChanged((1, 0), KeyStateChange::Released)));

        // Send a press event, then turn on the block for the key.
        // This is the classic layer-switching use case.
        assert!(is_enum_variant!(mat.update(press_1.clone(), t), MatrixUpdateResult::StateChanged((1, 0), KeyStateChange::Pressed)));
        mat.set_block(BlockedKeyStates::new_block_release_and_hold(), (1, 0));
        assert!(is_enum_variant!(mat.update(release_1, t), MatrixUpdateResult::Blocked));

        // The key shouldn't be blocked anymore.
        assert!(is_enum_variant!(mat.update(press_1, t), MatrixUpdateResult::StateChanged((1, 0), KeyStateChange::Pressed)));
    }

    #[test]
    fn virtual_keyboard_matrix_held_keys() {

        // Grab a matrix and setup press + release events.
        let mut mat = get_simple_matrix();
        let press_1 : evdev::InputEvent = keys::KeyState(keys::SimpleKey::KEY_1, KeyStateChange::Pressed).into();

        // We're going to say the press happened at the UNIX_EPOCH.
        let t = Instant::now();
        assert!(is_enum_variant!(mat.update(press_1, t), MatrixUpdateResult::StateChanged((1, 0), KeyStateChange::Pressed)));

        let hold = Duration::from_millis(200);
        let pre_hold = t + Duration::from_millis(100);
        let post_hold = t + Duration::from_millis(300);

        mat.hold_down_threshold = hold;
        assert_eq!(mat.detect_held_keys(pre_hold).len(), 0);
        assert_eq!(mat.detect_held_keys(post_hold)[0], (1, 0));

        // Check that blocked key holds don't register.
        mat.set_block(BlockedKeyStates::new_block_release_and_hold(), (1, 0));
        assert_eq!(mat.detect_held_keys(pre_hold).len(), 0);
        assert_eq!(mat.detect_held_keys(post_hold).len(), 0);
    }
}