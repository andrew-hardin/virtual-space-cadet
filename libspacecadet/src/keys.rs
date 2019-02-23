use evdev_rs as evdev;
use crate::layer::Layer;
use crate::virtual_keyboard_matrix::KeyStateChange;
use crate::keyboard_driver::KeyboardDriver;

pub use evdev::enums::EV_KEY as KEY;

// A key code is our primary interface for keys.
pub trait KeyCode {

    // Handle a state change event for this key.
    fn handle_event(&self, driver: &mut KeyboardDriver, state: KeyStateChange) {}

    // Is the key transparent (i.e. that when stacking layers, the key is a pass-through
    // to the key below it.
    fn is_transparent(&self) -> bool { true }
}

pub struct BlankKey { }
impl KeyCode for BlankKey { }

impl KeyCode for KEY {
    fn handle_event(&self, driver: &mut KeyboardDriver, state: KeyStateChange) {
        println!("Handling event...");
    }

    fn is_transparent(&self) -> bool { false }
}
