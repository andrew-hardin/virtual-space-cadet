use evdev_rs as evdev;
use crate::layer::LayerCollection;
use crate::virtual_keyboard_matrix::{KeyStateChange, VirtualKeyboardMatrix};
use crate::virtual_keyboard_matrix::Index2D;
use crate::layer::ScheduledLayerEvent;

pub use evdev::enums::EV_KEY as SimpleKey;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::virtual_keyboard_matrix::BlockedKeyStates;
use crate::output_keyboard::{EventBuffer, OutputKeyboard};

/// The context/state surrounding a key event (e.g. press).
pub struct KeyEventContext<'a> {
    pub output_device: &'a mut OutputKeyboard,
    pub virtual_matrix: &'a mut VirtualKeyboardMatrix,
    pub layers: &'a mut LayerCollection,
    pub location: Index2D
}

/// Shorthand for a key and state change pair.
pub struct KeyState(pub SimpleKey, pub KeyStateChange);

/// Shorthand for converting a key and state change into an `evdev::InputEvent`.
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

/// Contextual constraints that ensure a key's behavior acts as expected.
pub enum KeyConstraint {
    /// The key on the other layer must be transparent.
    KeyOnOtherLayerIsTransparent(String),
    /// A layer with the given name must exist.
    LayerExists(String)
}


/// The primary interface for custom keys (e.g. macros or layer toggles).
pub trait KeyCode {

    /// React to a `KeyStateChange` event (e.g. the key was pressed).
    fn handle_event(&mut self, _ctx: &mut KeyEventContext, _state: KeyStateChange) { }

    /// Check if the key is transparent (i.e. a pass-through to the key in the next lower layer).
    fn is_transparent(&self) -> bool { false }

    /// Get any constraints the key may have.
    fn get_constraints(&self) -> Vec<KeyConstraint> {
        vec![]
    }
}


/// A key that's transparent; a pass-through to the key below it in the layer hierarchy.
pub struct TransparentKey;
impl KeyCode for TransparentKey {
    fn is_transparent(&self) -> bool { true }
}

impl KeyCode for SimpleKey {
    fn handle_event(&mut self, ctx: &mut KeyEventContext, state: KeyStateChange) {
        ctx.output_device.send(KeyState(self.clone(), state).into());
    }
}

/// A key that's the opposite of transparent; a no-op key that doesn't act on any events.
pub struct OpaqueKey;
impl KeyCode for OpaqueKey { }


/// A key that's a collection of other simple keys that are quickly pressed sequentially.
pub struct MacroKey {
    /// When to play the macro (e.g. when the key is pressed or released).
    pub play_macro_when: KeyStateChange,
    /// The collection of keys to play.
    pub keys: Vec<SimpleKey>,
}

impl KeyCode for MacroKey {
    fn handle_event(&mut self, ctx: &mut KeyEventContext, state: KeyStateChange) {
        if state == self.play_macro_when {
            for i in self.keys.iter_mut() {
                i.handle_event(ctx, KeyStateChange::Pressed);
                i.handle_event(ctx, KeyStateChange::Released);
            }
        }
    }
}


/// A key that toggles a layer on or off.
pub struct ToggleLayerKey {
    /// Name of the layer to toggle.
    pub layer_name: String
}

impl KeyCode for ToggleLayerKey {
    fn handle_event(&mut self, ctx: &mut KeyEventContext, state: KeyStateChange) {
        if state == KeyStateChange::Pressed {
            // Toggle the layer, and mask the RELEASED event that'll be processed soon.
            ctx.layers.toggle(&self.layer_name);
            ctx.virtual_matrix.set_block(BlockedKeyStates::new_block_release_and_hold(), ctx.location);
        }
    }
    fn get_constraints(&self) -> Vec<KeyConstraint> {
        vec![KeyConstraint::LayerExists(self.layer_name.clone())]
    }
}


/// A key that enables a layer when pressed, then disable it when released.
pub struct MomentarilyEnableLayerKey {
    /// Name of the layer to enable/disable.
    pub layer_name: String
}

impl KeyCode for MomentarilyEnableLayerKey {
    fn handle_event(&mut self, ctx: &mut KeyEventContext, state: KeyStateChange) {
        match state {
            KeyStateChange::Held =>  { }
            KeyStateChange::Released => { ctx.layers.set(&self.layer_name, false); }
            KeyStateChange::Pressed => { ctx.layers.set(&self.layer_name, true); }
        }
    }
    fn get_constraints(&self) -> Vec<KeyConstraint> {
        vec![
            KeyConstraint::LayerExists(self.layer_name.clone()),
            KeyConstraint::KeyOnOtherLayerIsTransparent(self.layer_name.clone())
        ]
    }
}


/// A key than enables a layer.
pub struct EnableLayerKey {
    /// Name of the layer to enable.
    pub layer_name: String
}

impl KeyCode for EnableLayerKey {
    fn handle_event(&mut self, ctx: &mut KeyEventContext, state: KeyStateChange) {
        match state {
            KeyStateChange::Pressed => {
                // Enable the layer.
                ctx.layers.set(&self.layer_name, true);

                // This was a PRESSED, but a RELEASED event will soon follow.
                // We don't want that event to hit the layer we just switched to.
                // Temporarily block the release + hold events for this layer position.
                ctx.virtual_matrix.set_block(BlockedKeyStates::new_block_release_and_hold(), ctx.location);
            }
            _ => {}
        }
    }
    fn get_constraints(&self) -> Vec<KeyConstraint> {
        vec![KeyConstraint::LayerExists(self.layer_name.clone())]
    }
}


/// A key than enables a layer when held, but emits a simple key when pressed + released quickly.
pub struct HoldEnableLayerPressKey {
    layer_name: String,
    key: SimpleKey,
    pressed_at: SystemTime,
    hold_threshold: Duration,
}

impl HoldEnableLayerPressKey {
    /// Create a new key by specifying the layer to change on hold and the key to emit when pressed + released.
    pub fn new(layer_name: &str, key: SimpleKey, hold_threshold: Duration) -> HoldEnableLayerPressKey {
        HoldEnableLayerPressKey {
            layer_name: layer_name.to_string(),
            key,
            pressed_at: UNIX_EPOCH,
            hold_threshold
        }
    }

    fn is_held_long_enough(&self) -> bool {
        let delta = SystemTime::now().duration_since(self.pressed_at).unwrap();
        delta > self.hold_threshold
    }
}

impl KeyCode for HoldEnableLayerPressKey {
    fn handle_event(&mut self, ctx: &mut KeyEventContext, state: KeyStateChange) {
        match state {
            KeyStateChange::Held => {
                // The keyboard driver determined the key was held - but was it held long enough?
                // If so, enable the layer.
                // Then block any future holds and the next release on the new layer. This helps
                // prevent phantom "releases" after a key switches to a different layer but
                // hasn't been released yet.
                if self.is_held_long_enough() {
                    ctx.layers.set(&self.layer_name, true);
                    ctx.virtual_matrix.set_block(BlockedKeyStates::new_block_release_and_hold(), ctx.location);
                }
            }
            KeyStateChange::Pressed => {
                self.pressed_at = SystemTime::now();
            }
            KeyStateChange::Released => {
                if self.is_held_long_enough() {
                    ctx.layers.set(&self.layer_name, true);
                } else {
                    self.key.handle_event(ctx, KeyStateChange::Pressed);
                    self.key.handle_event(ctx, KeyStateChange::Released);
                }
            }
        }
    }
    fn get_constraints(&self) -> Vec<KeyConstraint> {
        vec![KeyConstraint::LayerExists(self.layer_name.clone())]
    }
}


/// A key than enables a layer when pressed and disables the layer after the next key is pressed + released.
pub struct OneShotLayer {
    pub layer_name: String
}

impl KeyCode for OneShotLayer {
    fn handle_event(&mut self, ctx: &mut KeyEventContext, state: KeyStateChange) {
        match state {
            KeyStateChange::Held => { }
            KeyStateChange::Pressed => {
                // Enable the target layer.
                ctx.layers.set(&self.layer_name, true);

                // Inject a counter based call-back that disables the layer
                // after another key has been released (this position doesn't count).
                let t = KeyStateChange::Released;
                let e = ScheduledLayerEvent {
                    layer_name: self.layer_name.clone(),
                    event_type: t,
                    event_count: ctx.output_device.get_stats().get(t) + 1,
                    enable_layer_at_event: false,
                };
                ctx.layers.schedule_event_count_callback(e);

                // Register a block that temporarily prevents any holds or releases
                // for being registered for this key. If this block was being run under
                // a RELEASED event, then this wouldn't be necessary.
                //
                // ... but based on one man's opinion, the ergonomics are better if the layer
                // switches on PRESS (especially when typing quickly).
                ctx.virtual_matrix.set_block(BlockedKeyStates::new_block_release_and_hold(), ctx.location);

            }
            KeyStateChange::Released => { }
        }
    }
    fn get_constraints(&self) -> Vec<KeyConstraint> {
        vec![KeyConstraint::LayerExists(self.layer_name.clone())]
    }
}


/// A key wrapped with a modifier. The modifier is pressed,
/// the `KeyCode` is pressed and released, then the modifier is released.
pub struct ModifierWrappedKey {
    pub key: Box<KeyCode>,
    pub modifier: SimpleKey,
}

impl KeyCode for ModifierWrappedKey {
    fn handle_event(&mut self, ctx: &mut KeyEventContext, state: KeyStateChange) {
        match state {
            KeyStateChange::Pressed => {
                self.modifier.handle_event(ctx, state);
                self.key.handle_event(ctx, state);
            }
            KeyStateChange::Held => {
                self.key.handle_event(ctx, state);
            }
            KeyStateChange::Released => {
                self.key.handle_event(ctx, state);
                self.modifier.handle_event(ctx, state);
            }
        }
    }
}


/// A key that acts like a modifier when used with another key,
/// but acts like a simple key when tapped.
///
/// The classic example is `SHIFT` when the cord contains another key, but `(` when tapped.
pub struct SpaceCadet {
    key_when_tapped: Box<KeyCode>,
    modifier: SimpleKey,
    number_of_keys_pressed: u32
}

impl SpaceCadet {
    pub fn new_from_key(when_tapped: SimpleKey, modifier: SimpleKey) -> SpaceCadet {
        SpaceCadet::new(Box::new(when_tapped), modifier)
    }

    pub fn new(when_tapped: Box<KeyCode>, modifier: SimpleKey) -> SpaceCadet {
        SpaceCadet {
            key_when_tapped: when_tapped,
            modifier,
            number_of_keys_pressed: 0
        }
    }
}

// Thought:
//   The space cadet modifier is based on the stats of real key events that are being written.
//   Special keys, like a layer change key, won't be logically recorded as a "key hit after this key".
//   I'm not sure that's a problem, but maybe it indicates a problem with our logical
//   representation of what's going on.
impl KeyCode for SpaceCadet {
    fn handle_event(&mut self, ctx: &mut KeyEventContext, state: KeyStateChange) {
        match state {
            KeyStateChange::Pressed => {
                // When the key is pressed, we don't know whether to start the modifier
                // or to emit a PRESS + RELEASE for the key. This means we need to watch
                // for new key events. Key interception is well outside our permissions, so we'll
                // rely on the driver to fill a two event buffer (modifier + some other key).
                ctx.output_device.set_buffer(EventBuffer::new_spacecadet());

                // Record how many keys have been pressed, then send a pressed event.
                // Because the buffer is active, this press won't increment any stats yet.
                self.number_of_keys_pressed = ctx.output_device.get_stats().get(KeyStateChange::Pressed);
                self.modifier.handle_event(ctx, KeyStateChange::Pressed);
            }
            KeyStateChange::Released => {
                // Was the key pressed and released without any other keys being struck?
                let pressed_count = ctx.output_device.get_stats().get(KeyStateChange::Pressed);
                let press_and_immediate_release = pressed_count == self.number_of_keys_pressed;
                if press_and_immediate_release {
                    // Reset the driver's space cadet buffer.
                    // Recall that this will clear a "modifier pressed" event.
                    ctx.output_device.set_buffer(EventBuffer::new());

                    // Then send a PRESS + RELEASE for the key.
                    self.key_when_tapped.handle_event(ctx, KeyStateChange::Pressed);
                    self.key_when_tapped.handle_event(ctx, KeyStateChange::Released);

                } else {
                    // Other keys were pressed since this key was pressed.
                    // We trust the driver to handle the key we placed in the spacecadet buffer.
                    // We'll send the closing release event for the modifier.
                    self.modifier.handle_event(ctx, KeyStateChange::Released);
                }
            }
            KeyStateChange::Held => {}
        }
    }
}
