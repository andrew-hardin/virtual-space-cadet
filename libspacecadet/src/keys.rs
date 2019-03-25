use evdev_rs as evdev;
use crate::layer::LayerCollection;
use crate::virtual_keyboard_matrix::{KeyStateChange, VirtualKeyboardMatrix};
use crate::virtual_keyboard_matrix::Index2D;
use crate::layer::ScheduledLayerEvent;

use std::str::FromStr;
use std::time::{Duration, Instant};
pub use evdev::enums::EV_KEY as SimpleKey;
use crate::virtual_keyboard_matrix::BlockedKeyStates;
use crate::output_keyboard::{EventBuffer, OutputKeyboard};
use crate::parser::*;

/// The context/state surrounding a key event (e.g. press).
pub struct KeyEventContext<'a> {
    pub output_device: &'a mut OutputKeyboard,
    pub virtual_matrix: &'a mut VirtualKeyboardMatrix,
    pub layers: &'a mut LayerCollection,
    pub location: Index2D,
    pub now: Instant,
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
    fn handle_event(&mut self, _ctx: &mut KeyEventContext, _state: KeyStateChange) {}

    /// Check if the key is transparent (i.e. a pass-through to the key in the next lower layer).
    fn is_transparent(&self) -> bool { false }

    /// Get any constraints the key may have.
    fn get_constraints(&self) -> Vec<KeyConstraint> { vec![] }
}


// The FromStr trait lets str::parse() be used to create a key code.
impl FromStr for Box<KeyCode> {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed = ParsedKeyTree::create(s)?;
        convert_tokens_to_key(&parsed)
    }
}


/// A key that's transparent; a pass-through to the key below it in the layer hierarchy.
pub struct TransparentKey {}
impl TransparentKey {
    pub fn from_tokens(item: &ParsedKeyTree) -> Result<TransparentKey, String> {
        if !item.args.is_empty() {
            Err("Transparent keys don't have arguments".to_string())
        } else if item.identifier.chars().all(|x| x == '_') {
            Ok(TransparentKey{})
        } else if item.identifier == "KC_TRANS" {
            Ok(TransparentKey{})
        } else {
            Err("Not a transparent key.".to_string())
        }
    }
}
impl KeyCode for TransparentKey {
    fn is_transparent(&self) -> bool { true }
}

/// A key that's the opposite of transparent; a no-op key that doesn't act on any events.
pub struct OpaqueKey;
impl OpaqueKey {
    pub fn from_tokens(item: &ParsedKeyTree) -> Result<OpaqueKey, String> {
        if !item.args.is_empty() {
            Err("Opaque keys don't have arguments".to_string())
        } else if item.identifier.chars().all(|x| x == 'X') {
            Ok(OpaqueKey{})
        } else if item.identifier == "KC_OPAQUE" {
            Ok(OpaqueKey{})
        } else {
            Err("Not an opaque key.".to_string())
        }
    }
}
impl KeyCode for OpaqueKey {}

#[derive(Clone)]
pub struct NormalKey {
    pub value: evdev::enums::EV_KEY
}
impl KeyCode for NormalKey {
    fn handle_event(&mut self, ctx: &mut KeyEventContext, state: KeyStateChange) {
        ctx.output_device.send(KeyState(self.value.clone(), state).into());
    }
}
impl NormalKey {
    pub fn from_tokens(item: &ParsedKeyTree) -> Result<NormalKey, String> {
        if !item.args.is_empty() {
            Err("Simple keys don't have arguments".to_string())
        } else {
            // Transform KC_Q into KEY_Q.
            let split : Vec<&str> = item.identifier.split('_').collect();
            if split.len() == 1 {
                Err("Missing an \"_\" character.".to_string())
            } else if split.len() > 2 {
                Err("Too many \"_\" characters.".to_string())
            } else {
                let trial = ("KEY_".to_string() + split[1]).to_uppercase();
                let v = evdev::enums::EventCode::from_str(&evdev::enums::EventType::EV_KEY, &trial);
                match v {
                    Some(v) => match v {
                        evdev::enums::EventCode::EV_KEY(k) => Ok(NormalKey { value : k }),
                        _ => Err("Key code was converted to an event, but it wasn't an EV_KEY.".to_string())
                    },
                    None => Err("Couldn't convert \"".to_string() + &trial + "\" to an evdev key.")
                }
            }
        }
    }
}


/// A key that's a collection of other simple keys that are quickly pressed sequentially.
pub struct MacroKey {
    /// When to play the macro (e.g. when the key is pressed or released).
    pub play_macro_when: KeyStateChange,
    /// The collection of keys to play.
    pub keys: Vec<NormalKey>,
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
impl MacroKey {
    pub fn from_tokens(item: &ParsedKeyTree) -> Result<MacroKey, String> {
        if item.identifier != "MACRO" {
            Err("Wrong identifier.".to_string())
        } else {
            let mut ans = MacroKey {
                play_macro_when: KeyStateChange::Pressed,
                keys: vec![]
            };

            for i in item.args.iter() {
                let converted = NormalKey::from_tokens(i)?;
                ans.keys.push(converted)
            }
            Ok(ans)
        }
    }
}


/// Shorthand for keys whose only argument is a layer name.
fn expecting_just_layer_arg(item: &ParsedKeyTree) -> Result<String, String> {
    if item.args.is_empty() {
        Err("Missing layer name.".to_string())
    } else if item.args.len() > 2 {
        Err("Too many arguments.".to_string())
    } else if item.args[0].args.is_empty() {
        Err("Layer name doesn't have arguments".to_string())
    } else {
        Ok(item.args[0].identifier.to_string())
    }
}


/// A key that toggles a layer on or off.
pub struct ToggleLayerKey {
    /// Name of the layer to toggle.
    pub layer_name: String
}

impl ToggleLayerKey {
    pub fn from_tokens(item: &ParsedKeyTree) -> Result<ToggleLayerKey, String> {
        if item.identifier != "TG" {
            Err("Wrong identifier.".to_string())
        } else {
            Ok(ToggleLayerKey { layer_name: expecting_just_layer_arg(item)? })
        }
    }
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

impl MomentarilyEnableLayerKey {
    pub fn from_tokens(item: &ParsedKeyTree) -> Result<MomentarilyEnableLayerKey, String> {
        if item.identifier != "MO" {
            Err("Wrong identifier.".to_string())
        } else {
            Ok(MomentarilyEnableLayerKey { layer_name: expecting_just_layer_arg(item)? })
        }
    }
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
pub struct ActivateLayerKey {
    /// Name of the layer to enable.
    pub layer_name: String
}

impl ActivateLayerKey {
    pub fn from_tokens(item: &ParsedKeyTree) -> Result<ActivateLayerKey, String> {
        if item.identifier != "AL" {
            Err("Wrong identifier.".to_string())
        } else {
            Ok(ActivateLayerKey { layer_name: expecting_just_layer_arg(item)? })
        }
    }
}

impl KeyCode for ActivateLayerKey {
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
    key: NormalKey,
    pressed_at: Instant,
    hold_threshold: Duration,
}

impl HoldEnableLayerPressKey {
    /// Create a new key by specifying the layer to change on hold and the key to emit when pressed + released.
    pub fn new(layer_name: &str, key: NormalKey, hold_threshold: Duration) -> HoldEnableLayerPressKey {
        HoldEnableLayerPressKey {
            layer_name: layer_name.to_string(),
            key,
            pressed_at: Instant::now(),
            hold_threshold
        }
    }

    fn is_held_long_enough(&self, now: Instant) -> bool {
        let delta = now.duration_since(self.pressed_at);
        delta > self.hold_threshold
    }

    pub fn from_tokens(item: &ParsedKeyTree) -> Result<HoldEnableLayerPressKey, String> {
        if item.identifier != "LT" {
            Err("Wrong identifier.".to_string())
        } else if item.args.len() != 3 {
            Err("Wrong number of arguments.".to_string())
        } else {
            // Get a layer name.
            let layer_name = item.args[0].identifier;
            let key = NormalKey::from_tokens(&item.args[1])?;
            let duration_ms = item.args[2].identifier.parse();
            let duration_ms = duration_ms.or(Err("Couldn't convert duration to milliseconds".to_string()))?;
            Ok(HoldEnableLayerPressKey::new(layer_name, key, Duration::from_millis(duration_ms)))
        }
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
                if self.is_held_long_enough(ctx.now) {
                    ctx.layers.set(&self.layer_name, true);
                    ctx.virtual_matrix.set_block(BlockedKeyStates::new_block_release_and_hold(), ctx.location);
                }
            }
            KeyStateChange::Pressed => {
                self.pressed_at = ctx.now;
            }
            KeyStateChange::Released => {
                if self.is_held_long_enough(ctx.now) {
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

impl OneShotLayer {
    pub fn from_tokens(item: &ParsedKeyTree) -> Result<OneShotLayer, String> {
        if item.identifier != "OSL" {
            Err("Wrong identifier.".to_string())
        } else {
            Ok(OneShotLayer { layer_name: expecting_just_layer_arg(item)? })
        }
    }
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


/// A key wrapped with another key (e.g. SHIFT). The wrap key is pressed,
/// the `KeyCode` is pressed and released, then wrap is released.
pub struct WrappedKey {
    pub inside: Box<KeyCode>,
    pub outside: NormalKey,
}

impl WrappedKey {
    pub fn from_tokens(item: &ParsedKeyTree) -> Result<WrappedKey, String> {
        if item.identifier != "WRAP" {
            Err("Wrong identifier.".to_string())
        } else if item.args.len() != 2 {
            Err("Wrong number of arguments.".to_string())
        } else {
            Ok(WrappedKey {
                outside: NormalKey::from_tokens(&item.args[0])?,
                inside: convert_tokens_to_key(&item.args[1])?
            })
        }
    }
}

impl KeyCode for WrappedKey {
    fn handle_event(&mut self, ctx: &mut KeyEventContext, state: KeyStateChange) {
        match state {
            KeyStateChange::Pressed => {
                self.outside.handle_event(ctx, state);
                self.inside.handle_event(ctx, state);
            }
            KeyStateChange::Held => {
                self.inside.handle_event(ctx, state);
            }
            KeyStateChange::Released => {
                self.inside.handle_event(ctx, state);
                self.outside.handle_event(ctx, state);
            }
        }
    }
}


/// A key that acts like a modifier when used with another key,
/// but acts like a simple key when tapped.
///
/// The classic example is `SHIFT` when the cord contains another key, but `(` when tapped.
pub struct SpaceCadet {
    when_tapped: Box<KeyCode>,
    when_held: NormalKey,
    number_of_keys_pressed: u32
}

impl SpaceCadet {
    pub fn new_from_key(when_tapped: NormalKey, modifier: NormalKey) -> SpaceCadet {
        SpaceCadet::new(Box::new(when_tapped), modifier)
    }

    pub fn new(when_tapped: Box<KeyCode>, modifier: NormalKey) -> SpaceCadet {
        SpaceCadet {
            when_tapped,
            when_held: modifier,
            number_of_keys_pressed: 0
        }
    }

    pub fn from_tokens(item: &ParsedKeyTree) -> Result<SpaceCadet, String> {
        if item.identifier != "SPACECADET" {
            Err("Wrong identifier.".to_string())
        } else if item.args.len() != 2 {
            Err("Wrong number of arguments.".to_string())
        } else {
            Ok(SpaceCadet::new(
                convert_tokens_to_key(&item.args[0])?,
                NormalKey::from_tokens(&item.args[1])?
            ))
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
                self.when_held.handle_event(ctx, KeyStateChange::Pressed);
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
                    self.when_tapped.handle_event(ctx, KeyStateChange::Pressed);
                    self.when_tapped.handle_event(ctx, KeyStateChange::Released);

                } else {
                    // Other keys were pressed since this key was pressed.
                    // We trust the driver to handle the key we placed in the spacecadet buffer.
                    // We'll send the closing release event for the modifier.
                    self.when_held.handle_event(ctx, KeyStateChange::Released);
                }
            }
            KeyStateChange::Held => {}
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_io_keyboard::*;
    use crate::layer::{LayerAttributes, KeyCodeMatrix};
    use crate::keyboard_driver::*;

    type TestDriver = KeyboardDriver<TestInputKeyboard, TestOutputKeyboard>;


    /// Utility for setting up a test driver with one key.
    fn get_test_driver(key: Box<KeyCode>) -> TestDriver { get_test_driver_multilayer(vec![key]) }

    /// Utility for setting up a test driver with one key on each layer.
    fn get_test_driver_multilayer(keys: Vec<Box<KeyCode>>) -> TestDriver {
        assert!(!keys.is_empty());
        let mut layers = LayerCollection::new();
        let mut layer_codes = Vec::new();
        for i in keys.into_iter().enumerate() {
            layers.add(LayerAttributes {
                name: format!("layer_{}", i.0),
                enabled: i.0 == 0
            });

            let mut codes = KeyCodeMatrix::new((1, 2));
            codes.codes[0][0] = i.1;
            layer_codes.push(codes);
        }

        TestDriver {
            input: TestInputKeyboard::new(),
            output: TestOutputKeyboard::new(),
            layer_attributes: layers,
            layered_codes: layer_codes,
            matrix: VirtualKeyboardMatrix::new(vec![vec![Some(SimpleKey::KEY_1), Some(SimpleKey::KEY_2)]], None),
        }
    }

    #[test]
    fn normal_key() {
        let mut fx = get_test_driver(Box::new(NormalKey { value: SimpleKey::KEY_A }));

        let position: SimpleKey = SimpleKey::KEY_1;
        let press : evdev::InputEvent = KeyState(position.clone(), KeyStateChange::Pressed).into();
        let hold : evdev::InputEvent = KeyState(position.clone(), KeyStateChange::Held).into();
        let release : evdev::InputEvent = KeyState(position, KeyStateChange::Released).into();

        // Simulate a key press, hold, and release.
        // Because the hold occurs in a single clock tick, we should expect just two events.
        fx.input.events = vec![press, hold, release];
        fx.clock_tick(Instant::now());
        assert_eq!(fx.output.events.len(), 2);
    }

    fn check_noop_key(key: Box<KeyCode>) {
        let mut fx = get_test_driver(key);
        fx.input.events = vec![KeyState(SimpleKey::KEY_1, KeyStateChange::Pressed).into()];
        fx.clock_tick(Instant::now());
        assert!(fx.output.events.is_empty());
    }

    #[test]
    fn transparent_key() {
        check_noop_key(Box::new(TransparentKey{}));
    }

    #[test]
    fn opaque_key() {
        check_noop_key(Box::new(OpaqueKey{}));
    }

    #[test]
    fn macro_key() {
        // Construct a driver with a single macro key.
        let key = MacroKey {
            play_macro_when: KeyStateChange::Released,
            keys: vec![NormalKey { value: SimpleKey::KEY_H }, NormalKey { value: SimpleKey::KEY_I }]
        };
        let mut fx = get_test_driver(Box::new(key));

        // Process a press event.
        let t = Instant::now();
        fx.input.events.push(KeyState(SimpleKey::KEY_1, KeyStateChange::Pressed).into());
        fx.clock_tick(t);
        assert_eq!(fx.output.events.len(), 0);

        // Process a release event (this should trigger the macro).
        fx.input.events.push(KeyState(SimpleKey::KEY_1, KeyStateChange::Released).into());
        fx.clock_tick(t);
        assert_eq!(fx.output.events.len(), 4);

        // Check that the order is press then release.
        for i in fx.output.events.iter().enumerate() {
            if i.0 % 2 == 0 { assert_eq!(i.1.value, KeyStateChange::Pressed as i32) }
            else { assert_eq!(i.1.value, KeyStateChange::Released as i32); }
        }
    }

    #[test]
    fn toggle_layer_key() {
        // Create a driver with a toggle layer key on layer_0, and a simple key on layer_1
        let key = ToggleLayerKey { layer_name: "layer_1".to_string() };
        let plain_key = NormalKey { value: SimpleKey::KEY_A };
        let mut fx = get_test_driver_multilayer(
            vec![Box::new(key), Box::new(plain_key.clone())]
        );

        // Toggle the layer on key press.
        assert!(!fx.layer_attributes.is_enabled(1));
        fx.input.events.push(KeyState(SimpleKey::KEY_1, KeyStateChange::Pressed).into());
        fx.clock_tick(Instant::now());
        assert!(fx.layer_attributes.is_enabled(1));

        // The layer changed on key-down. The key-press event could be registered
        // on a random key- check that we're blocking this.
        assert!(fx.input.events.is_empty());
        assert!(fx.output.events.is_empty());
        fx.input.events.push(KeyState(SimpleKey::KEY_1, KeyStateChange::Released).into());
        fx.clock_tick(Instant::now());
        assert!(fx.output.events.is_empty());

        // Suppose that the user presses the key again.
        // This should register on the second layer.
        fx.input.events.push(KeyState(SimpleKey::KEY_1, KeyStateChange::Pressed).into());
        fx.clock_tick(Instant::now());
        assert_eq!(fx.output.events.len(), 1);
        assert_eq!(fx.output.events[0].event_code, evdev::enums::EventCode::EV_KEY(plain_key.value));
    }

    #[test]
    fn momentarily_enable_layer_key() {
        // Setup the test driver.
        let test_key = MomentarilyEnableLayerKey { layer_name: "layer_1".to_string() };
        let plain_key = TransparentKey{};
        let mut fx = get_test_driver_multilayer(
            vec![Box::new(test_key), Box::new(plain_key)]
        );

        let press = KeyState(SimpleKey::KEY_1, KeyStateChange::Pressed).into();
        let release = KeyState(SimpleKey::KEY_1, KeyStateChange::Released).into();

        // The test key enables a layer on press, and disables on release.
        assert!(!fx.layer_attributes.is_enabled(1));
        fx.input.events.push(press);
        fx.clock_tick(Instant::now());
        assert!(fx.layer_attributes.is_enabled(1));
        fx.input.events.push(release);
        fx.clock_tick(Instant::now());
        assert!(!fx.layer_attributes.is_enabled(1));
    }

    #[test]
    fn activate_layer_key() {
        // Setup the test driver.
        let test_key = ActivateLayerKey { layer_name: "layer_1".to_string() };
        let plain_key = NormalKey { value: SimpleKey::KEY_A };
        let mut fx = get_test_driver_multilayer(
            vec![Box::new(test_key), Box::new(plain_key.clone())]
        );

        let press = KeyState(SimpleKey::KEY_1, KeyStateChange::Pressed).into();
        let release = KeyState(SimpleKey::KEY_1, KeyStateChange::Released).into();

        // Simulate a press.
        assert!(!fx.layer_attributes.is_enabled(1));
        fx.input.events.push(press);
        fx.clock_tick(Instant::now());
        assert!(fx.layer_attributes.is_enabled(1));

        // Check that the release is blocked.
        fx.input.events.push(release);
        fx.clock_tick(Instant::now());
        assert!(fx.output.events.is_empty());
    }

    #[test]
    fn hold_enable_layer_press_key() {
        // Setup the driver with the test key at (0, 0).
        let not_hold = Duration::from_millis(10);
        let theshold = Duration::from_millis(15);
        let hold = Duration::from_millis(20);
        let long_pause = Duration::from_secs(60);
        let mut t = Instant::now();

        // Configure the driver.
        let test_key = HoldEnableLayerPressKey::new("layer_1", NormalKey { value: SimpleKey::KEY_A }, theshold);
        let plain_key = NormalKey { value: SimpleKey::KEY_B };
        let mut fx = get_test_driver_multilayer(
            vec![Box::new(test_key), Box::new(plain_key)]
        );

        let press : evdev::InputEvent = KeyState(SimpleKey::KEY_1, KeyStateChange::Pressed).into();
        let release : evdev::InputEvent = KeyState(SimpleKey::KEY_1, KeyStateChange::Released).into();

        // Test the quick press + release that emits a key
        // instead of modifying the layers states.
        fx.input.events.push(press.clone());
        fx.clock_tick(t);
        assert!(!fx.layer_attributes.is_enabled(1));
        assert!(fx.output.events.is_empty());

        // Release the key before it registers as a hold.
        // This should emit A (both press and release).
        fx.input.events.push(release.clone());
        fx.clock_tick(t + not_hold);
        assert!(!fx.layer_attributes.is_enabled(1));
        assert_eq!(fx.output.events.len(), 2);

        // Reset the output, then simulate a long pause.
        fx.output.events.clear();
        t += long_pause;

        // Test the press + hold + release that should enable a layer.
        fx.input.events.push(press);
        fx.clock_tick(t);
        assert!(!fx.layer_attributes.is_enabled(1));
        assert!(fx.output.events.is_empty());

        // Release the key - it should register as being held long enough.
        fx.input.events.push(release.clone());
        fx.clock_tick(t + hold);
        assert!(fx.layer_attributes.is_enabled(1));
        assert!(fx.output.events.is_empty());
    }

    #[test]
    fn one_shot_layer() {
        let test_key = OneShotLayer {
            layer_name: "layer_1".to_string()
        };
        let plain_key = NormalKey { value: SimpleKey::KEY_B };
        let mut fx = get_test_driver_multilayer(
            vec![Box::new(test_key), Box::new(plain_key)]
        );

        let press1 : evdev::InputEvent = KeyState(SimpleKey::KEY_1, KeyStateChange::Pressed).into();
        let release1 : evdev::InputEvent = KeyState(SimpleKey::KEY_1, KeyStateChange::Released).into();

        // The layer should be enabled on press.
        assert!(!fx.layer_attributes.is_enabled(1));
        fx.input.events.push(press1);
        fx.clock_tick(Instant::now());
        assert!(fx.layer_attributes.is_enabled(1));

        // The following release event shouldn't output any events.
        fx.input.events.push(release1);
        fx.clock_tick(Instant::now());
        assert!(fx.output.events.is_empty());

        // We should be able to press and release a key on the newly enabled layer.
        // The driver's missing a key in the second column - we'll add one for the test.
        fx.layered_codes[1].codes[0][1] = Box::new(NormalKey { value : SimpleKey::KEY_Z });
        let press2 : evdev::InputEvent = KeyState(SimpleKey::KEY_2, KeyStateChange::Pressed).into();
        let release2 : evdev::InputEvent = KeyState(SimpleKey::KEY_2, KeyStateChange::Released).into();

        // Press.
        assert!(fx.layer_attributes.is_enabled(1));
        fx.input.events.push(press2);
        fx.clock_tick(Instant::now());
        assert_eq!(fx.output.events.len(), 1);
        assert!(fx.layer_attributes.is_enabled(1));

        // Release.
        fx.input.events.push(release2);
        fx.clock_tick(Instant::now());
        assert_eq!(fx.output.events.len(), 2);
        assert!(!fx.layer_attributes.is_enabled(1));
    }

    #[test]
    fn modifier_wrapped_key() {
        let test_key = WrappedKey {
            inside: Box::new(NormalKey { value: SimpleKey::KEY_Z }),
            outside: NormalKey { value: SimpleKey::KEY_LEFTSHIFT },
        };
        let mut fx = get_test_driver(Box::new(test_key));
        let press : evdev::InputEvent = KeyState(SimpleKey::KEY_1, KeyStateChange::Pressed).into();
        let release : evdev::InputEvent = KeyState( SimpleKey::KEY_1, KeyStateChange::Released).into();

        let t0 = Instant::now();
        let t1 = t0 + Duration::from_secs(10);
        let t2 = t1 + Duration::from_millis(10);

        // Press.
        fx.input.events.push(press);
        fx.clock_tick(t0);
        assert_eq!(fx.output.events.len(), 2);

        // Hold.
        fx.clock_tick(t1);
        assert_eq!(fx.output.events.len(), 3);

        // Release.
        fx.input.events.push(release);
        fx.clock_tick(t2);
        assert_eq!(fx.output.events.len(), 5);

        // Check ordering and values.
        let values = [
            KeyStateChange::Pressed,
            KeyStateChange::Pressed,
            KeyStateChange::Held,
            KeyStateChange::Released,
            KeyStateChange::Released];
        let codes = [
            SimpleKey::KEY_LEFTSHIFT,
            SimpleKey::KEY_Z,
            SimpleKey::KEY_Z,
            SimpleKey::KEY_Z,
            SimpleKey::KEY_LEFTSHIFT];
        for i in fx.output.events.iter().enumerate() {
            assert_eq!(i.1.value, values[i.0] as i32);
            assert_eq!(i.1.event_code, evdev::enums::EventCode::EV_KEY(codes[i.0].clone()));
        }
    }

    #[test]
    fn space_cadet_taprelease() {

        // Setup the driver.
        let test_key = SpaceCadet::new_from_key(
            NormalKey { value: SimpleKey::KEY_Z },
            NormalKey { value: SimpleKey::KEY_LEFTSHIFT });
        let mut fx = get_test_driver(Box::new(test_key));

        let press : evdev::InputEvent = KeyState(SimpleKey::KEY_1, KeyStateChange::Pressed).into();
        let release : evdev::InputEvent = KeyState( SimpleKey::KEY_1, KeyStateChange::Released).into();

        // Quickly tap and release the key.
        // This should result in a Z being pressed + released.
        let t = Instant::now();
        let t1 = t + Duration::from_millis(10);
        fx.input.events.push(press);
        fx.clock_tick(t);
        assert!(fx.output.events.is_empty());
        fx.input.events.push(release);
        fx.clock_tick(t1);
        assert_eq!(fx.output.events.len(), 2);

        // Check values for equality.
        let values = [
            KeyStateChange::Pressed,
            KeyStateChange::Released];
        let codes = [
            SimpleKey::KEY_Z,
            SimpleKey::KEY_Z];
        for i in fx.output.events.iter().enumerate() {
            assert_eq!(i.1.value, values[i.0] as i32);
            assert_eq!(i.1.event_code, evdev::enums::EventCode::EV_KEY(codes[i.0].clone()));
        }
    }

    #[test]
    fn space_cadet_hold() {

        // Setup the driver.
        let test_key = SpaceCadet::new_from_key(
            NormalKey { value: SimpleKey::KEY_Z },
            NormalKey { value: SimpleKey::KEY_LEFTSHIFT });
        let mut fx = get_test_driver(Box::new(test_key));
        fx.layered_codes[0].codes[0][1] = Box::new(NormalKey { value: SimpleKey::KEY_Y });

        let press1 : evdev::InputEvent = KeyState(SimpleKey::KEY_1, KeyStateChange::Pressed).into();
        let release1 : evdev::InputEvent = KeyState( SimpleKey::KEY_1, KeyStateChange::Released).into();
        let press2 : evdev::InputEvent = KeyState(SimpleKey::KEY_2, KeyStateChange::Pressed).into();
        let release2 : evdev::InputEvent = KeyState( SimpleKey::KEY_2, KeyStateChange::Released).into();

        // Press the spacecadet key, then press a second key while the
        // spacecadet is still being held.
        let t = Instant::now();
        let event_sequence = [press1, press2.clone(), release2.clone(), press2, release2, release1];
        for i in event_sequence.iter().enumerate() {
            fx.input.events.push(i.1.clone());
            fx.clock_tick(t + (i.0 as u32) * Duration::from_millis(10));
        }

        assert_eq!(fx.output.events.len(), 6);

        // Check the sequence of output events.
        let values = [
            KeyStateChange::Pressed,
            KeyStateChange::Pressed,
            KeyStateChange::Released,
            KeyStateChange::Pressed,
            KeyStateChange::Released,
            KeyStateChange::Released];
        let codes = [
            SimpleKey::KEY_LEFTSHIFT,
            SimpleKey::KEY_Y,
            SimpleKey::KEY_Y,
            SimpleKey::KEY_Y,
            SimpleKey::KEY_Y,
            SimpleKey::KEY_LEFTSHIFT];
        for i in fx.output.events.iter().enumerate() {
            assert_eq!(i.1.value, values[i.0] as i32);
            assert_eq!(i.1.event_code, evdev::enums::EventCode::EV_KEY(codes[i.0].clone()));
        }
    }
}