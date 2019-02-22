extern crate evdev_rs as evdev;
extern crate uinput;
#[macro_use] extern crate enum_derive;

use libc::c_int;
use std::time;
use std::fs::File;
use evdev::enums::EventType;
use std::collections;
use std::collections::HashMap;
use uinput::event::Event::Keyboard;
use evdev::util::int_to_event_code;

const EVENT_LOOP_HZ_RATE: u64 = 100;
const EVENT_LOOP_RATE: time::Duration = time::Duration::from_millis(1000 / EVENT_LOOP_HZ_RATE);
type KeyMatrix = Vec<Vec<Option<evdev::enums::EV_KEY>>>;
type StateMatrix = Vec<Vec<bool>>;
type Index2D = (usize, usize);

struct VirtualKeyboardMatrix {
    key_locations: KeyMatrix,
    key_to_index: collections::HashMap<evdev::enums::EV_KEY, Index2D>,
    dim: Index2D,
    state: StateMatrix,
}

impl VirtualKeyboardMatrix {
    fn new(keys: KeyMatrix) -> VirtualKeyboardMatrix {

        // Loop through the event matrix and store a map from event -> index.
        let dim = (keys.len(), keys[0].len());
        let mut hash = HashMap::new();
        for r in 0..dim.0 {
            for c in 0..dim.1 {
                match &keys[r][c] {
                    Some(t) => hash.insert(t.clone(), (r, c)),
                    None => None
                    // This (row,col) in the virtual keyboard matrix doesn't have
                    // a key assigned. It's therefore impossible for this matrix
                    // location to ever be pressed (true) or released (false).
                };
            }
        }

        // Initialize the state matrix such that no key is pressed.
        let initial_state = vec![vec![false; dim.1]; dim.0];

        VirtualKeyboardMatrix {
            key_locations: keys,
            key_to_index: hash,
            dim: dim,
            state: initial_state
        }
    }

    // Update the matrix state by processing a single event.
    // Returns a bool indicating if the event was in the matrix.
    fn update(&mut self, event: evdev::InputEvent) -> bool {
        match event.event_code {
            evdev::enums::EventCode::EV_KEY(which) => {
                let location = self.key_to_index.get(&which);
                match location {
                    Some(index) => {
                        // Great, the key corresponds to an index.
                        // Code the state is either pressed or not, then return true because
                        // the key was mapped to a matrix position.
                        self.state[index.0][index.1] = match event.value {
                            0 => false,
                            _ => true
                        };
                        true
                    }
                    None => false
                }
            }
            // Any other event code isn't handled (i.e. we're only working with keys).
            _ => false
        }
    }

    fn pretty_print(&self) {
        for r in 0..self.dim.0 {
            for c in 0..self.dim.1 {
                if self.state[r][c] {
                    print!("1");
                } else {
                    print!("0");
                }
                if c == self.dim.1 - 1 {
                    println!("");
                } else {
                    print!(" ");
                }
            }
        }
    }
}

fn get_keypad_matrix() -> KeyMatrix {
    return vec![
        vec![Some(evdev::enums::EV_KEY::KEY_NUMLOCK), Some(evdev::enums::EV_KEY::KEY_KPSLASH), Some(evdev::enums::EV_KEY::KEY_KPASTERISK)],
        vec![Some(evdev::enums::EV_KEY::KEY_KP7), Some(evdev::enums::EV_KEY::KEY_KP8), Some(evdev::enums::EV_KEY::KEY_KP9)],
        vec![Some(evdev::enums::EV_KEY::KEY_KP4), Some(evdev::enums::EV_KEY::KEY_KP5), Some(evdev::enums::EV_KEY::KEY_KP6)],
        vec![Some(evdev::enums::EV_KEY::KEY_KP1), Some(evdev::enums::EV_KEY::KEY_KP2), Some(evdev::enums::EV_KEY::KEY_KP3)]
    ]
}

fn cyclic_executor<F>(action: &mut F) where F: FnMut() {
    let mut warned = false;
    loop {
        let start = time::Instant::now();
        action();
        let end = time::Instant::now();
        let elapsed = end - start;
        if elapsed < EVENT_LOOP_RATE {
            let remaining_time = EVENT_LOOP_RATE - elapsed;
            std::thread::sleep(remaining_time);
        } else if !warned {
            println!("Event loop executing slower than {}hz.", EVENT_LOOP_HZ_RATE);
            warned = false;
        }
    }
}

struct InputKeyboard {
    _file_descriptor: File,
    device: evdev::Device
}

impl InputKeyboard {

    // Open an input keyboard. Behind the scenes we're opening a non-blocking
    // file descriptor and constructing a evdev device.
    fn open(path: &str) -> InputKeyboard {
        use std::fs::OpenOptions;
        use std::os::unix::fs::OpenOptionsExt;
        let file_descriptor = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
            .unwrap();

        let mut device = evdev::Device::new_from_fd(&file_descriptor).unwrap();
        device.grab(evdev::GrabMode::Grab).unwrap();

        InputKeyboard {
            _file_descriptor : file_descriptor,
            device
        }
    }

    // Read all pending events from the device.
    // Non-blocking (i.e. returns if no events were there).
    fn read_events(&self) -> Vec<evdev::InputEvent> {
        let mut ans= Vec::new();
        loop {
            // TODO: based on the library example, there may be an
            //       edge case related to sync that's not being handled.
            let a = self.device.next_event(evdev::NORMAL);
            match a {
                Ok(k) => {
                    // We only forward on EV_KEY events.
                    match k.1.event_type {
                        EventType::EV_KEY => { ans.push(k.1); }
                        _ => ()
                    }

                }
                Err(_) => break
            }
        }
        ans
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

struct OutputKeyboard {
    device: uinput::Device,
    evdev_to_uinput: EvdevToUinput
}

impl OutputKeyboard {
    fn new() -> OutputKeyboard {
        let mut device = uinput::default().unwrap()
            .name("spacecadet").unwrap()
            .event(uinput::event::Keyboard::All).unwrap()
            .create().unwrap();

        OutputKeyboard {
            device: device,
            evdev_to_uinput: EvdevToUinput::new()
        }
    }

    fn send(&mut self, e: evdev::InputEvent) {
        // evdev event -> uinput event -> device command.
        let code = e.value;
        let e = self.evdev_to_uinput.convert(e).unwrap();
        println!("sending {:?}", e);
        self.device.send(e, code).unwrap();
        self.device.synchronize().unwrap();
    }
}

struct KeyboardDriver {
    input: InputKeyboard,
    output: OutputKeyboard,
    matrix: VirtualKeyboardMatrix
}

impl KeyboardDriver {
    fn clock_tick(&mut self) {
        for i in self.input.read_events() {
            let bypass = !self.matrix.update(i.clone());
            if bypass {
                // Bypass the driver and forward to the output device.
                self.output.send(i);
            } else {
                println!("------------------------");
                self.matrix.pretty_print();
            }
        }
    }
}


fn tick(d: &mut KeyboardDriver) {
    //println!("Tick!");
    d.clock_tick();
}



fn main() {
    println!("Running....");

    let foo = EvdevToUinput::new();

    let mut keyboard = KeyboardDriver {
        input: InputKeyboard::open("/dev/input/event4"),
        output: OutputKeyboard::new(),
        matrix: VirtualKeyboardMatrix::new(get_keypad_matrix())
    };

    let mut update = || tick(&mut keyboard);
    cyclic_executor(&mut update);
//
//    println!("... finished.");
}