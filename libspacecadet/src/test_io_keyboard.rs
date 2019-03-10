use evdev_rs as evdev;
use crate::input_keyboard::InputKeyboard;
use crate::output_keyboard::{EventBuffer, OutputKeyboard};
use crate::virtual_keyboard_matrix::KeyStats;


/// A keyboard that implements the input keyboard traits.
/// Used for testing keys and observing side effects.
pub struct TestInputKeyboard {
    pub events: Vec<evdev::InputEvent>,
    pub stats: KeyStats,
}

impl TestInputKeyboard {
    pub fn new() -> TestInputKeyboard {
        TestInputKeyboard {
            events: Vec::new(),
            stats: KeyStats::new(),
        }
    }
}

impl InputKeyboard for TestInputKeyboard {
    fn read_events(&mut self) -> Vec<evdev::InputEvent> {
        let ans = self.events.clone();
        for i in ans.iter() {
            self.stats.increment(i.value.into());
        }
        self.events = Vec::new();
        ans
    }
    fn get_stats(&self) -> KeyStats { self.stats }
}

/// A keyboard that implements the output keyboard traits.
/// Used for testing keys and observing side effects.
pub struct TestOutputKeyboard {
    pub events: Vec<evdev::InputEvent>,
    pub stats: KeyStats,
    pub buffer: EventBuffer
}


impl TestOutputKeyboard {
    pub fn new() -> TestOutputKeyboard {
        TestOutputKeyboard {
            events: Vec::new(),
            stats: KeyStats::new(),
            buffer: EventBuffer::new(),
        }
    }

    fn send_unbuffered(&mut self, e: evdev::InputEvent) {
        self.stats.increment(e.value.into());
        self.events.push(e);
    }
}

impl OutputKeyboard for TestOutputKeyboard {
    fn send_override(&mut self, e: evdev::InputEvent, bypass_buffer: bool) {
        if bypass_buffer { self.send_unbuffered(e); }
        else {
            // Add the item to the buffer, then send any items that the buffer returned.
            for item in self.buffer.add(e) {
                self.send_unbuffered(item);
            }
        }
    }
    fn set_buffer(&mut self, buffer: EventBuffer) { self.buffer = buffer; }
    fn get_stats(&self) -> KeyStats { self.stats }
}
