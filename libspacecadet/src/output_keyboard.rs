use evdev_rs as evdev;
use std::collections::HashMap;
use std::os::raw::c_int;
use crate::KeyStats;

#[derive(PartialEq, Debug)]
pub enum BufferSendKeysWhen {
    Immediately,
    BufferFull,
}

#[derive(PartialEq, Debug)]
pub enum BufferAfterSendingKeys {
    StopBuffering
}

pub struct EventBuffer {
    buffer: Vec<evdev::InputEvent>,
    send_keys_policy: BufferSendKeysWhen,
    post_send_policy: BufferAfterSendingKeys,
}

impl EventBuffer {
    pub fn new() -> EventBuffer {
        EventBuffer {
            buffer: Vec::with_capacity(0),
            send_keys_policy: BufferSendKeysWhen::Immediately,
            post_send_policy: BufferAfterSendingKeys::StopBuffering
        }
    }

    pub fn new_spacecadet() -> EventBuffer {
        EventBuffer {
            buffer: Vec::with_capacity(2),
            send_keys_policy: BufferSendKeysWhen::BufferFull,
            post_send_policy: BufferAfterSendingKeys::StopBuffering
        }
    }

    pub fn add(&mut self, e: evdev::InputEvent) -> Vec<evdev::InputEvent> {
        match self.send_keys_policy {
            BufferSendKeysWhen::Immediately => { vec![e] },
            BufferSendKeysWhen::BufferFull => {
                // Add the event onto the buffer.
                assert_eq!(false, self.is_full());
                self.buffer.push(e);

                // If the buffer is full, we need to return all the key events
                // and clear the internal buffer + stop buffering (capacity = 0).
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

pub struct OutputKeyboard {
    device: uinput::Device,
    evdev_to_uinput: EvdevToUinput,
    event_buffer: EventBuffer,
    pub stats: KeyStats
}

impl OutputKeyboard {
    pub fn new() -> OutputKeyboard {
        let device = uinput::default().unwrap()
            .name("spacecadet").unwrap()
            .event(uinput::event::Keyboard::All).unwrap()
            .create().unwrap();

        OutputKeyboard {
            device: device,
            evdev_to_uinput: EvdevToUinput::new(),
            event_buffer: EventBuffer::new(),
            stats: KeyStats::new(),
        }
    }

    pub fn set_buffer(&mut self, buffer: EventBuffer) {
        // TODO: what if the buffer already has keys?
        self.event_buffer = buffer;
    }

    pub fn send(&mut self, e: evdev::InputEvent) {
        for item in self.event_buffer.add(e) {
            self.send_unbuffered(item);
        }
    }

    pub fn send_unbuffered(&mut self, e: evdev::InputEvent) {
        // evdev event -> uinput event -> device command.
        let code = e.value;
        self.stats.increment(code.into());
        let e = self.evdev_to_uinput.convert(e).unwrap();
        println!("sending {:?} (val = {})", e, code);
        self.device.send(e, code).unwrap();
        self.device.synchronize().unwrap();
    }
}



type CEventCodesToEvents = HashMap<c_int, HashMap<c_int, uinput::event::Event>>;

struct EvdevToUinput {
    // Maps a kind -> [map a code -> uinput event]
    kind_to_code_to_event: CEventCodesToEvents
}

impl EvdevToUinput {

    fn fill_event_lookup_structure<I: std::convert::Into<uinput::Event>, T: Iterator<Item=I>>(iter: T, ans: &mut CEventCodesToEvents) {
        for j in iter {
            use uinput::event::{Kind, Code};
            let value: uinput::event::Event = j.into();
            ans.entry(value.kind())
                .or_insert(HashMap::new()).insert(value.code(), value);
        }
    }

    fn new() -> EvdevToUinput {
        let mut i = CEventCodesToEvents::new();
        EvdevToUinput::fill_event_lookup_structure(uinput::event::keyboard::Key::iter_variants(), &mut i);
        EvdevToUinput::fill_event_lookup_structure(uinput::event::keyboard::KeyPad::iter_variants(), &mut i);
        EvdevToUinput::fill_event_lookup_structure(uinput::event::keyboard::Misc::iter_variants(), &mut i);
        EvdevToUinput::fill_event_lookup_structure(uinput::event::keyboard::InputAssist::iter_variants(), &mut i);
        EvdevToUinput::fill_event_lookup_structure(uinput::event::keyboard::Function::iter_variants(), &mut i);
        EvdevToUinput::fill_event_lookup_structure(uinput::event::keyboard::Braille::iter_variants(), &mut i);
        EvdevToUinput::fill_event_lookup_structure(uinput::event::keyboard::Numeric::iter_variants(), &mut i);
        EvdevToUinput::fill_event_lookup_structure(uinput::event::keyboard::TouchPad::iter_variants(), &mut i);
        EvdevToUinput::fill_event_lookup_structure(uinput::event::keyboard::Camera::iter_variants(), &mut i);
        EvdevToUinput::fill_event_lookup_structure(uinput::event::keyboard::Attendant::iter_variants(), &mut i);

        EvdevToUinput {
            kind_to_code_to_event: i
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