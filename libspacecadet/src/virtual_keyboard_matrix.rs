use evdev_rs as evdev;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::keys;


pub type KeyMatrix = Vec<Vec<Option<keys::KEY>>>;
pub type Index2D = (usize, usize);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyStateChange {
    Released = 0,
    Pressed = 1,
    Held = 2,
}

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

#[derive(Copy, Clone)]
pub struct KeyStats {
    values: [u32; 3]
}

impl KeyStats {
    pub fn new() -> KeyStats {
        KeyStats {
            values: [0, 0, 0]
        }
    }
    pub fn increment(&mut self, v: KeyStateChange) {
        self.values[v as usize] += 1;
    }

    pub fn get(&self, v: KeyStateChange) -> u32 {
        self.values[v as usize]
    }
}

pub struct VirtualKeyboardMatrix {
    key_to_index: HashMap<evdev::enums::EV_KEY, Index2D>,
    dim: Index2D,
    state: StateMatrix
}

pub struct UpdateResult {
    pub state_change: Option<KeyStateChange>,
    pub location: Index2D,
}

impl VirtualKeyboardMatrix {
    pub fn new(keys: KeyMatrix) -> VirtualKeyboardMatrix {

        // Loop through the event matrix and store a map from event -> index.
        let dim = (keys.len(), keys[0].len());
        let mut hash = HashMap::new();
        for r in 0..dim.0 {
            for c in 0..dim.1 {
                match &keys[r][c] {
                    Some(t) => hash.insert(t.clone(), (r, c)),
                    None => None
                    // This (row,col) in the virtual keyboard matrix doesn't have
                    // a key assigned. It's therefore impossible for this matrix
                    // location to ever be pressed (true) or released (false).
                };
            }
        }

        VirtualKeyboardMatrix {
            key_to_index: hash,
            dim: dim,
            state: StateMatrix::new(dim)
        }
    }

    // Update the matrix state by processing a single event.
    // Returns a bool indicating if the event was in the matrix.
    pub fn update(&mut self, event: evdev::InputEvent) -> Option<UpdateResult> {

        // Convert the event time into a friendly representation.
        let now = UNIX_EPOCH +
            Duration::new(event.time.tv_sec as u64, event.time.tv_usec as u32 * 1000);

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
                        let val = match event.value {
                            0 => false,
                            _ => true
                        };
                        Some(UpdateResult {
                            state_change: self.state.set(*index, val, now),
                            location: *index,
                        })
                    }
                    // Keys that aren't part of the matrix aren't handled.
                    None => None
                }
            }
            // Non key event codes aren't handled.
            _ => None
        }
    }

    pub fn detect_held_keys(&mut self, held_key_threshold: Duration) -> Vec<UpdateResult> {
        // Loop through every cell in the matrix and detect keys that
        // have been held for longer than the given threshold.
        let mut ans = Vec::new();
        let now = SystemTime::now();
        for r in 0..self.dim.0 {
            for c in 0..self.dim.1 {
                let idx = (r, c);
                if self.state.is_held(idx, held_key_threshold, now) {
                    ans.push(UpdateResult {
                        state_change: Some(KeyStateChange::Held),
                        location: idx,
                    });
                    self.state.reset_key_press_time(idx,now);
                }
            }
        }
        ans
    }
}

struct StateMatrix {
    state: Vec<Vec<bool>>,
    last_pressed: Vec<Vec<SystemTime>>,
}

impl StateMatrix {
    pub fn new(dim: Index2D) -> StateMatrix {
        // Initialize our state with every key unpressed.
        StateMatrix {
            state: vec![vec![false; dim.1]; dim.0],
            last_pressed: vec![vec![UNIX_EPOCH; dim.1]; dim.0],
        }
    }

    pub fn set(&mut self, dim: Index2D, is_pressed: bool, now: SystemTime) -> Option<KeyStateChange> {
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

    pub fn is_held(&self, idx: Index2D, hold_threshold: Duration, now: SystemTime) -> bool {
        let is_pressed = self.state[idx.0][idx.1];
        let held_long_enough = self.last_pressed[idx.0][idx.1] + hold_threshold <= now;
        is_pressed && held_long_enough
    }

    pub fn reset_key_press_time(&mut self, idx: Index2D, when: SystemTime) {
        self.last_pressed[idx.0][idx.1] = when;
    }
}