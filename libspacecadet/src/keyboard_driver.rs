use crate::input_keyboard::*;
use crate::output_keyboard::*;
use crate::virtual_keyboard_matrix::*;

pub struct KeyboardDriver {
    pub input: InputKeyboard,
    pub output: OutputKeyboard,
    pub matrix: VirtualKeyboardMatrix,
}

impl KeyboardDriver {
    pub fn clock_tick(&mut self) {
        for i in self.input.read_events() {
            let update = self.matrix.update(i.clone());
            let bypass = !update.within_matrix;
            if bypass {
                // Bypass the driver and forward to the output device.
                self.output.send(i);
            } else {
                println!("------------------------");
                //self.matrix.state.pretty_print();
            }
        }
    }
}






