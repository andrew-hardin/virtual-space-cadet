use crate::input_keyboard::*;
use crate::output_keyboard::*;
use crate::virtual_keyboard_matrix::*;
use crate::layer::*;
use crate::keys::*;
use std::time::{Duration, SystemTime};

/// An input keyboard, virtual matrix, and output keyboard.
pub struct KeyboardDriver {
    pub input: InputKeyboard,
    pub output: OutputKeyboard,
    pub matrix: VirtualKeyboardMatrix,
}

/// A keyboard driver with layers of keys.
pub struct LayeredKeyboardDriver {
    pub driver : KeyboardDriver,
    pub layered_codes: Vec<KeyCodeMatrix>,
    pub layer_attributes: LayerCollection
}

impl LayeredKeyboardDriver {

    /// Add a layer to the driver by specify its attributes and code matrix.
    pub fn add_layer(&mut self, attr: LayerAttributes, codes: KeyCodeMatrix) {
        self.layer_attributes.add(attr);
        self.layered_codes.push(codes);
    }

    pub fn clock_tick(&mut self) {

        // Before dispatching new events, check if any layers need to be disabled.
        self.layer_attributes.check_event_callbacks(self.driver.output.stats);

        // Check for any keys that have been held down and oppressed by the user.
        // TODO: relocate constant to a config/params object.
        let hold_down_threshold = Duration::from_millis(250);
        let now = SystemTime::now();
        for idx in self.driver.matrix.detect_held_keys(hold_down_threshold, now) {
            self.matrix_state_changed(idx, KeyStateChange::Pressed);
        }

        // Handle every event coming in from the input device.
        for i in self.driver.input.read_events() {
            match self.driver.matrix.update(i.clone()) {
                // The key was within the matrix - store the update for later.
                MatrixUpdateResult::Bypass => { self.driver.output.send(i); },
                MatrixUpdateResult::Redundant(_idx) => {},
                MatrixUpdateResult::StateChanged(idx, state) => { self.matrix_state_changed(idx, state); },
                MatrixUpdateResult::Blocked => {}
            }
        }
    }

    fn matrix_state_changed(&mut self, idx: Index2D, state: KeyStateChange) {

        // Starting at the highest enabled layer, find the first key that's
        // not transparent.
        let l = self.layered_codes.len();
        for i in (0..l).rev() {
            if self.layer_attributes.is_enabled(i) {
                let code = &mut self.layered_codes[i].codes[idx.0][idx.1];
                if !code.is_transparent() {
                    println!("Found key on layer {}", i);

                    // Capture references to the driver and layers - then ask the key to handle
                    // a state change event.
                    let mut context = KeyEventContext {
                        driver: &mut self.driver,
                        layers: &mut self.layer_attributes,
                        location: idx,
                    };
                    code.handle_event(&mut context, state);
                    return;
                }
            }
        }
        println!("Reached bottom of stack without hitting a key.");
    }
}






