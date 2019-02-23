use crate::input_keyboard::*;
use crate::output_keyboard::*;
use crate::virtual_keyboard_matrix::*;
use crate::layer::*;
use crate::KeyCode;

pub struct KeyboardDriver {
    pub input: InputKeyboard,
    pub output: OutputKeyboard,
    pub matrix: VirtualKeyboardMatrix,
    pub layers: Vec<Layer>
}

impl KeyboardDriver {
    pub fn clock_tick(&mut self) {
        //TODO: move to a function that only runs once.
        //self.init();

        for i in self.input.read_events() {
            let update = self.matrix.update(i.clone());
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
                None => { self.output.send(i); }
            }
        }
    }

    pub fn add_layer(&mut self, l: Layer) {
        self.layers.push(l);
    }

    pub fn init(&self) {

    }

    fn matrix_state_changed(&mut self, idx: Index2D, state: KeyStateChange) {

        let v = self.get_key_code(idx).unwrap();
        v.handle_event(self, state);

        for i in (0..self.layers.len()).rev() {
            if self.layers[i].enabled { continue; }

            let l = &self.layers[i];
            let c = &l.codes.codes[idx.0][idx.1];
            c.handle_event(self, state);

            break;
        }

//        match code {
//            Some(t) => { t.handle_event(self, state); }
//            None => println!("Fell through!")
//        }
    }

    fn get_key_code(&mut self, idx: Index2D) -> Option< Box<KeyCode>> {

        // Being new to Rust, there's probably a better way to write this
        // expression. Here's the intent:
        //   > find all non-transparent keys on enabled layers, then select the
        //     key that's closest to the top of the layer stack.
//        self.layers.iter_mut()
//            .filter(|x| x.enabled)
//            .map(|x| &mut x.codes.codes[idx.0][idx.1])
//            .filter(|x| !x.is_transparent())
//            .rev()
//            .next()
        return None;
    }
}






