use evdev_rs as evdev;
use crate::layer::Layer;
use crate::virtual_keyboard_matrix::KeyStateChange;
use crate::keyboard_driver::KeyboardDriver;

pub use evdev::enums::EV_KEY as KEY;


// A key code is our primary interface for keys.
pub trait KeyCode {

    // Handle a state change event for this key.
    fn handle_event(&self, driver: &mut KeyboardDriver, state: KeyStateChange) { }

    // Is the key transparent (i.e. that when stacking layers, the key is a pass-through
    // to the key below it.
    fn is_transparent(&self) -> bool { true }
}

pub struct BlankKey { }
impl KeyCode for BlankKey { }

impl KeyCode for KEY {
    fn handle_event(&self, driver: &mut KeyboardDriver, state: KeyStateChange) {
        let v = evdev::InputEvent {
            time: evdev::TimeVal {
                tv_usec: 0,
                tv_sec: 0,
            },
            event_type : evdev::enums::EventType::EV_KEY,
            event_code : evdev::enums::EventCode::EV_KEY(self.clone()),
            value: match state {
                KeyStateChange::Pressed => 1,
                KeyStateChange::Released => 0
            }
        };
        driver.output.send(v);
    }

    fn is_transparent(&self) -> bool { false }
}
