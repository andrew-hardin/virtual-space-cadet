use evdev_rs as evdev;
use crate::layer::LayerCollection;
use crate::virtual_keyboard_matrix::KeyStateChange;
use crate::virtual_keyboard_matrix::Index2D;
use crate::keyboard_driver::KeyboardDriver;
use crate::layer::ScheduledLayerEvent;

pub use evdev::enums::EV_KEY as KEY;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::virtual_keyboard_matrix::BlockedKeyStates;
use crate::output_keyboard::EventBuffer;

pub struct KeyState(pub KEY, pub KeyStateChange);

// Short hand for converting a key and state change into an InputEvent.
impl Into<evdev::InputEvent> for KeyState {
    fn into(self) -> evdev::InputEvent {
        evdev::InputEvent {
            time: evdev::TimeVal {
                tv_usec: 0,
                tv_sec: 0,
            },
            event_type : evdev::enums::EventType::EV_KEY,
            event_code : evdev::enums::EventCode::EV_KEY(self.0),
            value: self.1 as i32
        }
    }
}

// A key code is our primary interface for keys.
pub trait KeyCode {

    // Handle a state change event for this key.
    fn handle_event(&mut self, _: &mut KeyboardDriver, _: KeyStateChange, _: &mut LayerCollection, _: Index2D) { }

    // Is the key transparent (i.e. that when stacking layers, the key is a pass-through
    // to the key below it.
    fn is_transparent(&self) -> bool { false }
}

pub struct BlankKey;
impl KeyCode for BlankKey {
    fn is_transparent(&self) -> bool { true }
}

impl KeyCode for KEY {
    fn handle_event(&mut self, driver: &mut KeyboardDriver, state: KeyStateChange, _ : &mut LayerCollection, _: Index2D) {
        driver.output.send(KeyState(self.clone(), state).into());
    }
}

pub struct MacroKey {
    pub play_macro_when: KeyStateChange,
    pub keys: Vec<KEY>,
}

impl KeyCode for MacroKey {
    fn handle_event(&mut self, driver: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection, idx: Index2D) {
        if state == self.play_macro_when {
            for i in self.keys.iter_mut() {
                i.handle_event(driver, KeyStateChange::Pressed, l, idx);
                i.handle_event(driver, KeyStateChange::Released, l, idx);
            }
        }

    }
}

// Imitating TG.
pub struct ToggleLayerKey {
    pub layer_name: String
}

impl KeyCode for ToggleLayerKey {
    fn handle_event(&mut self, driver: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection, idx: Index2D) {
        if state == KeyStateChange::Pressed {
            // Toggle the layer, and mask the RELEASED event that'll be processed soon.
            l.toggle(&self.layer_name);
            driver.matrix.set_block(BlockedKeyStates::new_block_release_and_hold(), idx);
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
    fn handle_event(&mut self, _: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection, _: Index2D) {
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
    fn handle_event(&mut self, driver: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection, idx: Index2D) {
        match state {
            KeyStateChange::Pressed => {
                // Enable the layer.
                l.set(&self.layer_name, true);

                // This was a PRESSED, but a RELEASED event will soon follow.
                // We don't want that event to hit the layer we just switched to.
                // Temporarily block the release + hold events for this layer position.
                driver.matrix.set_block(BlockedKeyStates::new_block_release_and_hold(), idx);
            }
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
    fn handle_event(&mut self, driver: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection, idx: Index2D) {
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
                    self.key.handle_event(driver, KeyStateChange::Pressed, l, idx);
                    self.key.handle_event(driver, KeyStateChange::Released, l, idx);
                }
            }
        }

    }
}

// Imitate OSL.
pub struct OneShotLayer {
    pub layer_name: String
}

impl KeyCode for OneShotLayer {
    fn handle_event(&mut self, driver: &mut KeyboardDriver, state: KeyStateChange, l : &mut LayerCollection, idx: Index2D) {
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

                // Register a block that temporarily prevents any holds or releases
                // for being registered for this key. If this block was being run under
                // a RELEASED event, then this wouldn't be necessary.
                //
                // ... but based on one man's opinion, the ergonomics are better if the layer
                // switches on PRESS (especially when typing quickly).
                driver.matrix.set_block(BlockedKeyStates::new_block_release_and_hold(), idx);

            }
            KeyStateChange::Released => { }
        }

    }
}

pub struct ModifierWrappedKey {
    pub key: Box<KeyCode>,
    pub modifier: KEY,
}

impl KeyCode for ModifierWrappedKey {
    fn handle_event(&mut self, driver: &mut KeyboardDriver, state: KeyStateChange, l: &mut LayerCollection, idx: Index2D) {
        match state {
            KeyStateChange::Pressed => {
                self.modifier.handle_event(driver, state, l, idx);
                self.key.handle_event(driver, state, l, idx);
            }
            KeyStateChange::Held => {
                self.key.handle_event(driver, state, l, idx);
            }
            KeyStateChange::Released => {
                self.key.handle_event(driver, state, l, idx);
                self.modifier.handle_event(driver, state, l, idx);
            }
        }
    }
}

pub struct SpaceCadet {
    key_when_tapped: Box<KeyCode>,
    modifier: KEY,
    number_of_keys_pressed: u32
}

impl SpaceCadet {
    pub fn new_from_key(when_tapped: KEY, modifier: KEY) -> SpaceCadet {
        SpaceCadet::new(Box::new(when_tapped), modifier)
    }

    pub fn new(when_tapped: Box<KeyCode>, modifier: KEY) -> SpaceCadet {
        SpaceCadet {
            key_when_tapped: when_tapped,
            modifier,
            number_of_keys_pressed: 0
        }
    }
}

// TODO: The space cadet modifier is based on the stats of real key events that are being written.
//       Special keys, like a layer change key, won't be logically recorded as a "key hit after this key".
//       I'm not sure that's a problem, but maybe it indicates a problem with our logical
//       representation of what's going on.
impl KeyCode for SpaceCadet {
    fn handle_event(&mut self, driver: &mut KeyboardDriver, state: KeyStateChange, l: &mut LayerCollection, idx: Index2D) {
        match state {
            KeyStateChange::Pressed => {
                // When the key is pressed, we don't know whether to start the modifier
                // or to emit a PRESS + RELEASE for the key. This means we need to watch
                // for new key events. Key interception is well outside our permissions, so we'll
                // rely on the driver to fill a two event buffer (modifier + some other key).
                driver.output.set_buffer(EventBuffer::new_spacecadet());
                self.modifier.handle_event(driver, KeyStateChange::Pressed, l, idx);
                self.number_of_keys_pressed = driver.output.stats.get(KeyStateChange::Pressed);
            }
            KeyStateChange::Released => {
                // Was the key pressed and released without any other keys being struck?
                let pressed_count = driver.output.stats.get(KeyStateChange::Pressed);
                let press_and_immediate_release = pressed_count == self.number_of_keys_pressed;
                if press_and_immediate_release {
                    // Reset the driver's space cadet buffer.
                    driver.output.set_buffer(EventBuffer::new());

                    // Then send a PRESS + RELEASE for the key.
                    self.key_when_tapped.handle_event(driver, KeyStateChange::Pressed, l, idx);
                    self.key_when_tapped.handle_event(driver, KeyStateChange::Released, l, idx);

                } else {
                    // Other keys were pressed since this key was pressed.
                    // We trust the driver to handle the key we placed in the spacecadet buffer.
                    // We'll send the closing release event for the modifier.
                    self.modifier.handle_event(driver, KeyStateChange::Released, l, idx);
                }
            }
            KeyStateChange::Held => {}
        }
    }
}
