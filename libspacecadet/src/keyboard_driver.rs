use crate::input_keyboard::*;
use crate::output_keyboard::*;
use crate::virtual_keyboard_matrix::*;
use crate::layer::*;
use crate::KeyCode;
use std::time::Duration;

pub struct KeyboardDriver {
    pub input: InputKeyboard,
    pub output: OutputKeyboard,
    pub matrix: VirtualKeyboardMatrix,
}

pub struct LayeredKeyboardDriver {
    pub driver : KeyboardDriver,
    pub layers: Vec<Layer>,
}

impl LayeredKeyboardDriver {
    pub fn clock_tick(&mut self) {

        // Check for any keys that have been held down and oppressed by the user.
        // TODO: relocate constant to a config/params object.
        let hold_down_threshold = Duration::from_millis(250);
        let mut updates = self.driver.matrix.detect_held_keys(hold_down_threshold);

        // Handle every event coming in from the input device.
        for i in self.driver.input.read_events() {
            match self.driver.matrix.update(i.clone()) {
                // The key was within the matrix - store the update for later.
                Some(v) => { updates.push(v); }

                // If the key wasn't in the matrix, bypass the logic that handles matrix
                // state changes and forward directly to the output device.
                None => { self.driver.output.send(i); }
            }
        }

        // Loop through all updates and dispatch a matrix state changed event.
        for v in updates {
            match v.state_change {
                Some(s) => { self.matrix_state_changed(v.location, s); }
                None => {}
            }
        }
    }

    pub fn add_layer(&mut self, l: Layer) {
        self.layers.push(l);
    }

    fn matrix_state_changed(&mut self, idx: Index2D, state: KeyStateChange) {

        // Starting at the highest enabled layer, find the first key that's
        // not transparent.
        let key = self.layers.iter()
            .rev()
            .filter(|x| x.enabled)
            .map(|x| &x.codes.codes[idx.0][idx.1])
            .filter(|x| !x.is_transparent())
            .next();

        // If we reached a key, ask it to interpret the state change event.
        match key {
            Some(t) => t.handle_event(&mut self.driver, state),
            None => { println!("Reached bottom of stack without hitting a key."); }
        }
    }

}






