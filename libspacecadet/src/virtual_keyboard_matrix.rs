use evdev_rs as evdev;
use std::collections::HashMap;
use crate::keys;


pub type KeyMatrix = Vec<Vec<Option<keys::KEY>>>;
type StateMatrix = Vec<Vec<bool>>;
type Index2D = (usize, usize);


pub struct VirtualKeyboardMatrix {
    key_to_index: HashMap<evdev::enums::EV_KEY, Index2D>,
    dim: Index2D,
    state: StateMatrix,
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

        // Initialize the state matrix such that no key is pressed.
        let initial_state = vec![vec![false; dim.1]; dim.0];

        VirtualKeyboardMatrix {
            key_to_index: hash,
            dim: dim,
            state: initial_state
        }
    }

    // Update the matrix state by processing a single event.
    // Returns a bool indicating if the event was in the matrix.
    pub fn update(&mut self, event: evdev::InputEvent) -> bool {
        match event.event_code {
            evdev::enums::EventCode::EV_KEY(which) => {
                let location = self.key_to_index.get(&which);
                match location {
                    Some(index) => {
                        // Great, the key corresponds to an index.
                        // Code the state is either pressed or not, then return true because
                        // the key was mapped to a matrix position.
                        self.state[index.0][index.1] = match event.value {
                            0 => false,
                            _ => true
                        };
                        true
                    }
                    None => false
                }
            }
            // Any other event code isn't handled (i.e. we're only working with keys).
            _ => false
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