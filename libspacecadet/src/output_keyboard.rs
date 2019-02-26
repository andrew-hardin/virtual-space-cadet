use evdev_rs as evdev;
use std::collections::HashMap;
use std::os::raw::c_int;
use crate::KeyStats;
use crate::KeyStateChange;

pub struct OutputKeyboard {
    device: uinput::Device,
    evdev_to_uinput: EvdevToUinput,
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
            stats: KeyStats::new(),
        }
    }

    pub fn send(&mut self, e: evdev::InputEvent) {
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