use evdev_rs as evdev;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::keys;


pub type KeyMatrix = Vec<Vec<Option<keys::KEY>>>;
type Index2D = (usize, usize);

pub enum KeyStateChange {
    Pressed,
    Released
}

pub struct VirtualKeyboardMatrix {
    key_to_index: HashMap<evdev::enums::EV_KEY, Index2D>,
    dim: Index2D,
    state: StateMatrix
}

pub struct UpdateResult {
    pub within_matrix: bool,
    pub state_change: Option<KeyStateChange>
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
    pub fn update(&mut self, event: evdev::InputEvent) -> UpdateResult {

        // Convert the event time into a friendly representation.
        use std::time::{Duration, UNIX_EPOCH};
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
                        return UpdateResult {
                            within_matrix: true,
                            state_change: self.state.set(*index, val, now)
                        }
                    }
                    // Keys that aren't part of the matrix aren't handled.
                    None => {}
                }
            }
            // Non key event codes aren't handled.
            _ => {}
        }

        // Return a pure bypass result (i.e. key wasn't in our matrix).
        UpdateResult {
            within_matrix: false,
            state_change: None
        }
    }
}

struct StateMatrix {
    state: Vec<Vec<bool>>,
    last_pressed: Vec<Vec<SystemTime>>,
    dim: Index2D
}

impl StateMatrix {
    pub fn new(dim: Index2D) -> StateMatrix {
        // Initialize our state with every key unpressed.
        StateMatrix {
            state: vec![vec![false; dim.1]; dim.0],
            last_pressed: vec![vec![UNIX_EPOCH; dim.1]; dim.0],
            dim
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

    pub fn pretty_print(&self) {
        for r in 0..self.dim.0 {
            for c in 0..self.dim.1 {
                if self.state[r][c] {
                    print!("1");
                } else {
                    print!("0");
                }
                if c == self.dim.1 - 1 {
                    println!("");
                } else {
                    print!(" ");
                }
            }
        }
    }
}