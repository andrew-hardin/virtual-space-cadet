use evdev_rs as evdev;
use crate::input_keyboard::InputKeyboard;
use crate::output_keyboard::{EventBuffer, OutputKeyboard};
use crate::virtual_keyboard_matrix::KeyStats;


/// A keyboard that implements both input and output keyboard traits.
/// Used for testing keys and observing side effects.
pub struct TestIOKeyboard {
    pub events_to_read: Vec<evdev::InputEvent>,
    pub output_events: Vec<evdev::InputEvent>,
    pub input_stats: KeyStats,
    pub output_stats: KeyStats,
    pub buffer: EventBuffer
}

impl TestIOKeyboard {
    pub fn new() -> TestIOKeyboard {
        TestIOKeyboard {
            events_to_read: Vec::new(),
            output_events: Vec::new(),
            input_stats: KeyStats::new(),
            output_stats: KeyStats::new(),
            buffer: EventBuffer::new(),
        }
    }
}

impl InputKeyboard for TestIOKeyboard {
    fn read_events(&mut self) -> Vec<evdev::InputEvent> {
        let ans = self.events_to_read.clone();
        for i in ans.iter() {
            self.input_stats.increment(i.value.into());
        }
        self.events_to_read = Vec::new();
        ans
    }
    fn get_stats(&self) -> KeyStats { self.input_stats }
}

impl TestIOKeyboard {
    fn send_unbuffered(&mut self, e: evdev::InputEvent) {
        self.output_stats.increment(e.value.into());
        self.output_events.push(e);
    }
}

impl OutputKeyboard for TestIOKeyboard {
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
    fn get_stats(&self) -> KeyStats { self.output_stats }
}
