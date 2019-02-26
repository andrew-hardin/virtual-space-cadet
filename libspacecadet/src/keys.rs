use evdev_rs as evdev;
use crate::layer::LayerCollection;
use crate::virtual_keyboard_matrix::{KeyStateChange, KeyStats};
use crate::keyboard_driver::KeyboardDriver;
use crate::layer::ScheduledLayerEvent;

pub use evdev::enums::EV_KEY as KEY;
use std::time::{Duration, SystemTime, UNIX_EPOCH};



// A key code is our primary interface for keys.
pub trait KeyCode {

    // Handle a state change event for this key.
    fn handle_event(&mut self, _: &mut KeyboardDriver, _: KeyStateChange, _: &mut LayerCollection) { }

    // Is the key transparent (i.e. that when stacking layers, the key is a pass-through
    // to the key below it.
    fn is_transparent(&self) -> bool { false }
}

pub struct BlankKey;
impl KeyCode for BlankKey {
    fn is_transparent(&self) -> bool { true }
}

impl KeyCode for KEY {
    fn handle_event(&mut self, driver: &mut KeyboardDriver, state: KeyStateChange, _ : &mut LayerCollection) {
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
    fn handle_event(&mut self, driver: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection) {
        if state == self.play_macro_when {
            for i in self.keys.iter_mut() {
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
    fn handle_event(&mut self, _: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection) {
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
    fn handle_event(&mut self, _: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection) {
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
    fn handle_event(&mut self, _: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection) {
        match state {
            KeyStateChange::Pressed => { l.set(&self.layer_name, true); }
            _ => {}
        }
    }
}

// Imitating LT.
pub struct HoldEnableLayerPressKey {
    layer_name: String,
    key: KEY,
    pressed_at: SystemTime,
}

impl HoldEnableLayerPressKey {
    pub fn new(layer_name: &str, key: KEY) -> HoldEnableLayerPressKey {
        HoldEnableLayerPressKey {
            layer_name: layer_name.to_string(),
            key,
            pressed_at: UNIX_EPOCH
        }
    }
}

impl KeyCode for HoldEnableLayerPressKey {
    fn handle_event(&mut self, driver: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection) {
        match state {
            KeyStateChange::Held => {
                /*
                TODO: Should we toggle the layer if it's been held long enough, but it hasn't yet been released?
                      Wouldn't that cause the release event to be processed on a different layer?
                      Maybe a shadow sink would gobble up the release event from this position?
                */
            }
            KeyStateChange::Pressed => {
                self.pressed_at = SystemTime::now();
            }
            KeyStateChange::Released => {
                let delta = SystemTime::now().duration_since(self.pressed_at).unwrap();
                // TODO: extract hold duration parameter...
                let was_held = delta > Duration::from_millis(200);
                if was_held {
                    l.set(&self.layer_name, true);
                } else {
                    self.key.handle_event(driver, KeyStateChange::Pressed, l);
                    self.key.handle_event(driver, KeyStateChange::Released, l);
                }
            }
        }

    }
}

// Imitate OSL.
pub struct OneShotLayer {
    pub layer_name: String
}

// TODO: what happens to the release after we've moved to the different layer?
//       related to HoldEnableLayerPressKey
impl KeyCode for OneShotLayer {
    fn handle_event(&mut self, driver: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection) {
        match state {
            KeyStateChange::Held => { }
            KeyStateChange::Pressed => {
                // Enable the target layer.
                l.set(&self.layer_name, true);

                // Inject a counter based call-back that disables the layer
                // after another key has been released (this position doesn't count).
                let t = KeyStateChange::Released;
                let e = ScheduledLayerEvent {
                    layer_name: self.layer_name.clone(),
                    event_type: t,
                    event_count: driver.output.stats.get(t) + 1,
                    enable_layer_at_event: false,
                };
                l.schedule_event_count_callback(e);
            }
            KeyStateChange::Released => { }
        }

    }
}