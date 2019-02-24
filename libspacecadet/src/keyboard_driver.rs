use crate::input_keyboard::*;
use crate::output_keyboard::*;
use crate::virtual_keyboard_matrix::*;
use crate::layer::*;
use crate::KeyCode;

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
        //TODO: move to a function that only runs once.
        //self.init();

        for i in self.driver.input.read_events() {
            let update = self.driver.matrix.update(i.clone());
            match update {
                // The key was within the matrix.
                Some(v) => {
                    // But did it experience a state change?
                    match v.state_change {
                        Some(s) => { self.matrix_state_changed(v.location, s); }
                        None => {}
                    }
                }

                // If the key wasn't in the matrix, bypass the driver and foward
                // to the output device.
                None => { self.driver.output.send(i); }
            }
        }
    }

    pub fn add_layer(&mut self, l: Layer) {
        self.layers.push(l);
    }

    pub fn init(&self) {

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

        match key {
            Some(t) => t.handle_event(&mut self.driver, state),
            None => { println!("Reached bottom of stack without hitting a key."); }
        }
    }

}






