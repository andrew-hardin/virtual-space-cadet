use evdev_rs as evdev;
use std::collections::HashMap;
use std::os::raw::c_int;
use crate::KeyStats;

/// An interface for output keyboards (e.g. sending events to OS).
pub trait OutputKeyboard {
    /// Send an event and specify the buffer policy.
    fn send_override(&mut self, e: evdev::InputEvent, bypass_buffer: bool);
    /// Set a buffer object that captures events before passing them to the OS.
    fn set_buffer(&mut self, buffer: EventBuffer);
    /// Get statistics on the type of events that have been written.
    fn get_stats(&self) -> KeyStats;
    /// Send an event that can't be buffered (i.e. immediately passed to OS).
    fn send_bypass_buffer(&mut self, e: evdev::InputEvent) {
        self.send_override(e, true);
    }
    /// Send an event that can be buffered.
    fn send(&mut self, e: evdev::InputEvent) {
        self.send_override(e, false);
    }
}

/// A wrapper around a uinput device.
pub struct UInputKeyboard {
    device: uinput::Device,
    evdev_to_uinput: EvdevToUinput,
    event_buffer: EventBuffer,
    stats: KeyStats
}

impl UInputKeyboard {
    /// Create a new uinput device with an optional name.
    pub fn new(device_name: Option<String>) -> Result<UInputKeyboard, uinput::Error> {
        let name = match device_name {
            Some(t) => t,
            None => "spacecadet".to_string()
        };
        let device = uinput::default()?
            .name(name)?
            .event(uinput::event::Keyboard::All)?
            .create()?;
        Ok(UInputKeyboard {
            device,
            evdev_to_uinput: EvdevToUinput::new(),
            event_buffer: EventBuffer::new(),
            stats: KeyStats::new(),
        })
    }

    /// Send an event to the output keyboard without buffering.
    fn send_unbuffered(&mut self, e: evdev::InputEvent) {
        // evdev event -> uinput event -> device command.
        let code = e.value;
        self.stats.increment(code.into());
        let e = self.evdev_to_uinput.convert(e).unwrap();
        println!("sending {:?} (val = {})", e, code);
        self.device.send(e, code).unwrap();
        self.device.synchronize().unwrap();
    }
}

impl OutputKeyboard for UInputKeyboard {
    fn send_override(&mut self, e: evdev::InputEvent, bypass_buffer: bool) {
        if bypass_buffer { self.send_unbuffered(e); }
        else {
            // Add the item to the buffer, then send any items that the buffer returned.
            for item in self.event_buffer.add(e) {
                self.send_unbuffered(item);
            }
        }
    }

    fn set_buffer(&mut self, buffer: EventBuffer) {
        // Concern: what if the buffer is holding keys?
        self.event_buffer = buffer;
    }

    fn get_stats(&self) -> KeyStats { self.stats }
}


/// The policy that governs when keys are sent out of the buffer.
#[derive(PartialEq, Debug)]
pub enum BufferSendKeysWhen {
    /// Send keys immediately - no buffering.
    Immediately,
    /// Send keys when the buffer is full.
    BufferFull,
}

/// The buffering policy to use after keys have been sent.
#[derive(PartialEq, Debug)]
pub enum BufferAfterSendingKeys {
    /// Stop buffering after keys are sent.
    StopBuffering
}

/// A simple buffer of `InputEvent` with policies for what to do
/// when the buffer is full and keys are sent.
pub struct EventBuffer {
    buffer: Vec<evdev::InputEvent>,
    send_keys_policy: BufferSendKeysWhen,
    post_send_policy: BufferAfterSendingKeys,
}

impl EventBuffer {
    /// Create a new event buffer with size zero (no buffering).
    pub fn new() -> EventBuffer {
        EventBuffer {
            buffer: Vec::with_capacity(0),
            send_keys_policy: BufferSendKeysWhen::Immediately,
            post_send_policy: BufferAfterSendingKeys::StopBuffering
        }
    }

    /// Create a new event buffer intended to be used in conjunction with spacecadet keys.
    pub fn new_spacecadet() -> EventBuffer {
        EventBuffer {
            buffer: Vec::with_capacity(2),
            send_keys_policy: BufferSendKeysWhen::BufferFull,
            post_send_policy: BufferAfterSendingKeys::StopBuffering
        }
    }

    /// Add an `InputEvent` to the buffer.
    pub fn add(&mut self, e: evdev::InputEvent) -> Vec<evdev::InputEvent> {
        match self.send_keys_policy {
            BufferSendKeysWhen::Immediately => { vec![e] },
            BufferSendKeysWhen::BufferFull => {
                // Add the event onto the buffer.
                assert_eq!(false, self.is_full());
                self.buffer.push(e);

                // If the buffer is full, we need to return events and
                // follow the post-send buffer policy.
                if self.is_full() {
                    let ans = self.buffer.clone();
                    match self.post_send_policy {
                        BufferAfterSendingKeys::StopBuffering => {
                            self.buffer = Vec::with_capacity(0);
                            self.send_keys_policy = BufferSendKeysWhen::Immediately;
                        }
                    }
                    ans
                } else {
                    // The buffer isn't full, so don't return any keys.
                    Vec::new()
                }
            }
        }
    }

    fn is_full(&self) -> bool {
        return self.buffer.capacity() == self.buffer.len()
    }
}


/// A utility structure for converting from evdev to uinput events.
struct EvdevToUinput {
    // Maps a kind -> [map a code -> uinput event]
    kind_to_code_to_event: HashMap<c_int, HashMap<c_int, uinput::event::Event>>
}

impl EvdevToUinput {
    /// Create a new mapping from evdev event integers to uinput events.
    fn new() -> EvdevToUinput {
        // Create an empty structure, then fill it.
        let mut ans = EvdevToUinput {
            kind_to_code_to_event: HashMap::new()
        };
        ans.fill_events();
        ans
    }

    fn fill_events(&mut self) {
        // This is ugly - I wish there was a compile-time alternative for populating
        // the map with uinput event lookups.
        self.fill_event_lookup_structure(uinput::event::keyboard::Key::iter_variants());
        self.fill_event_lookup_structure(uinput::event::keyboard::KeyPad::iter_variants());
        self.fill_event_lookup_structure(uinput::event::keyboard::Misc::iter_variants());
        self.fill_event_lookup_structure(uinput::event::keyboard::InputAssist::iter_variants());
        self.fill_event_lookup_structure(uinput::event::keyboard::Function::iter_variants());
        self.fill_event_lookup_structure(uinput::event::keyboard::Braille::iter_variants());
        self.fill_event_lookup_structure(uinput::event::keyboard::Numeric::iter_variants());
        self.fill_event_lookup_structure(uinput::event::keyboard::TouchPad::iter_variants());
        self.fill_event_lookup_structure(uinput::event::keyboard::Camera::iter_variants());
        self.fill_event_lookup_structure(uinput::event::keyboard::Attendant::iter_variants());
    }

    fn fill_event_lookup_structure<I: std::convert::Into<uinput::Event>, T: Iterator<Item=I>>(&mut self, iter: T) {
        for j in iter {
            use uinput::event::{Kind, Code};
            let value: uinput::event::Event = j.into();
            self.kind_to_code_to_event.entry(value.kind())
                .or_insert(HashMap::new())
                .insert(value.code(), value);
        }
    }

    fn convert(&self, e: evdev::InputEvent) -> Option<uinput::event::Event> {

        // Take the InputEvent from evdev and get the two integers that represent
        // the type (EV_KEY) and the code (KEY_A).
        let codes = evdev::util::event_code_to_int(&e.event_code);
        let codes = (codes.0 as i32, codes.1 as i32);

        // The nested matches go from kind -> code -> a uinput event.
        match self.kind_to_code_to_event.get(&codes.0) {
            Some(t) => {
                match t.get(&codes.1) {
                    Some(v) => return Some(*v),
                    None => return None
                }
            }
            None => return None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use uinput::event::{Kind, Code};

    #[test]
    fn event_buffer_defaults() {
        // Check the event buffer default constructor.
        let mut item = EventBuffer::new();
        let press_1 : evdev::InputEvent = keys::KeyState(keys::SimpleKey::KEY_1, KeyStateChange::Pressed).into();
        assert_eq!(item.add(press_1).len(), 1);
    }

    #[test]
    fn event_buffer_spacecadet() {
        let mut item = EventBuffer::new_spacecadet();
        let press_1 : evdev::InputEvent = keys::KeyState(keys::SimpleKey::KEY_1, KeyStateChange::Pressed).into();
        // Send two keys, and return the full buffer on the second.
        assert_eq!(item.add(press_1.clone()).len(), 0);
        assert_eq!(item.add(press_1.clone()).len(), 2);
        // Buffer should be disabled.
        assert_eq!(item.add(press_1).len(), 1);
    }

    #[test]
    fn evdev_to_uinput_check_one() {
        // Just check that KEY_1 is converted to the correct event.
        let item = EvdevToUinput::new();
        let press_1 : evdev::InputEvent = keys::KeyState(keys::SimpleKey::KEY_1, KeyStateChange::Pressed).into();
        let converted = item.convert(press_1);
        assert!(converted.is_some());
        let converted = converted.unwrap();
        assert_eq!(converted.kind(), 1);
        assert_eq!(converted.code(), 2);
    }

    #[test]
    fn evdev_to_uniput_unsupported_value() {
        // Check that non-key values map to none.
        let key = evdev::InputEvent {
            time: evdev::TimeVal {
                tv_usec: 0,
                tv_sec: 0,
            },
            event_type : evdev::enums::EventType::EV_SYN,
            event_code : evdev::enums::EventCode::EV_SYN(evdev::enums::EV_SYN::SYN_REPORT),
            value: 0 as i32
        };
        let item = EvdevToUinput::new();
        assert!(item.convert(key).is_none());
    }
}