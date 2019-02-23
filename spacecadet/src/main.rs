use std::time;
use libspacecadet::*;

const EVENT_LOOP_HZ_RATE: u64 = 100;
const EVENT_LOOP_RATE: time::Duration = time::Duration::from_millis(1000 / EVENT_LOOP_HZ_RATE);

fn get_keypad_matrix() -> KeyMatrix {
    return vec![
        vec![Some(KEY::KEY_NUMLOCK), Some(KEY::KEY_KPSLASH), Some(KEY::KEY_KPASTERISK)],
        vec![Some(KEY::KEY_KP7), Some(KEY::KEY_KP8), Some(KEY::KEY_KP9)],
        vec![Some(KEY::KEY_KP4), Some(KEY::KEY_KP5), Some(KEY::KEY_KP6)],
        vec![Some(KEY::KEY_KP1), Some(KEY::KEY_KP2), Some(KEY::KEY_KP3)]
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
            warned = true;
        }
    }
}

fn tick(d: &mut KeyboardDriver) {
    d.clock_tick();
}

fn main() {

    let mut keyboard = KeyboardDriver {
        input: InputKeyboard::open("/dev/input/event4"),
        output: OutputKeyboard::new(),
        matrix: VirtualKeyboardMatrix::new(get_keypad_matrix())
    };

    let mut update = || tick(&mut keyboard);
    cyclic_executor(&mut update);
}