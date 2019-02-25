use evdev_rs as evdev;
use crate::layer::LayerCollection;
use crate::virtual_keyboard_matrix::KeyStateChange;
use crate::keyboard_driver::KeyboardDriver;

pub use evdev::enums::EV_KEY as KEY;


// A key code is our primary interface for keys.
pub trait KeyCode {

    // Handle a state change event for this key.
    fn handle_event(&self, _: &mut KeyboardDriver, _: KeyStateChange, _: &mut LayerCollection) { }

    // Is the key transparent (i.e. that when stacking layers, the key is a pass-through
    // to the key below it.
    fn is_transparent(&self) -> bool { false }
}

pub struct BlankKey;
impl KeyCode for BlankKey {
    fn is_transparent(&self) -> bool { true }
}

impl KeyCode for KEY {
    fn handle_event(&self, driver: &mut KeyboardDriver, state: KeyStateChange, _ : &mut LayerCollection) {
        let v = evdev::InputEvent {
            time: evdev::TimeVal {
                tv_usec: 0,
                tv_sec: 0,
            },
            event_type : evdev::enums::EventType::EV_KEY,
            event_code : evdev::enums::EventCode::EV_KEY(self.clone()),
            value: match state {
                KeyStateChange::Held => 2,
                KeyStateChange::Pressed => 1,
                KeyStateChange::Released => 0,
            }
        };
        driver.output.send(v);
    }
}

pub struct MacroKey {
    pub play_macro_when: KeyStateChange,
    pub keys: Vec<KEY>,
}

impl KeyCode for MacroKey {
    fn handle_event(&self, driver: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection) {
        if state == self.play_macro_when {
            for i in self.keys.iter() {
                i.handle_event(driver, KeyStateChange::Pressed, l);
                i.handle_event(driver, KeyStateChange::Released, l);
            }
        }

    }
}

pub struct ToggleLayerKey {
    pub layer_name: String
}

impl KeyCode for ToggleLayerKey {
    fn handle_event(&self, _: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection) {
        if state == KeyStateChange::Released {
            l.toggle(&self.layer_name);
        }
    }
}

// Imitating MO.
// TODO: enforce the "enabled layer must be a transparent key" constraint.
//       this constraint is mentioned in some of the QMK documentation...
pub struct MomentarilyEnableLayerKey {
    pub layer_name: String
}

impl KeyCode for MomentarilyEnableLayerKey {
    fn handle_event(&self, _: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection) {
        match state {
            KeyStateChange::Held =>  { }
            KeyStateChange::Released => { l.set(&self.layer_name, false); }
            KeyStateChange::Pressed => { l.set(&self.layer_name, true); }
        }
    }
}

// Imitating TO
pub struct EnableLayerKey {
    pub layer_name: String
}

impl KeyCode for EnableLayerKey {
    fn handle_event(&self, _: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection) {
        match state {
            KeyStateChange::Pressed => { l.set(&self.layer_name, true); }
            _ => {}
        }
    }
}