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
}

impl TestIOKeyboard {
    pub fn new() -> TestIOKeyboard {
        TestIOKeyboard {
            events_to_read: Vec::new(),
            output_events: Vec::new(),
            input_stats: KeyStats::new(),
            output_stats: KeyStats::new()
        }
    }
}

impl InputKeyboard for TestIOKeyboard {
    fn read_events(&mut self) -> Vec<evdev::InputEvent> {
        let ans = self.events_to_read.clone();
        self.events_to_read = Vec::new();
        ans
    }
    fn get_stats(&self) -> KeyStats { self.input_stats }
}

impl OutputKeyboard for TestIOKeyboard {
    fn send_override(&mut self, e: evdev::InputEvent, _bypass_buffer: bool) {
        self.output_events.push(e);
    }
    fn set_buffer(&mut self, _buffer: EventBuffer) { }
    fn get_stats(&self) -> KeyStats { self.output_stats }
}
