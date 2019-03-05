use std::time;
use libspacecadet::*;

fn get_keypad_matrix() -> KeyMatrix {
    return vec![
        vec![Some(KEY::KEY_ESC)],
        vec![Some(KEY::KEY_TAB)],
        vec![Some(KEY::KEY_CAPSLOCK)],
        vec![Some(KEY::KEY_LEFTSHIFT)],
        vec![Some(KEY::KEY_RIGHTSHIFT)],
    ]
}

fn base_layer_keys() -> KeyCodeMatrix {
    let left_paren = Box::new(ModifierWrappedKey {
        key: Box::new(KEY::KEY_9),
        modifier: KEY::KEY_LEFTSHIFT
    });
    let right_paren = Box::new(ModifierWrappedKey {
        key: Box::new(KEY::KEY_9),
        modifier: KEY::KEY_RIGHTSHIFT
    });

    let mut ans = KeyCodeMatrix::new((5, 1));
    ans.codes[1][0] = Box::new(KEY::KEY_CAPSLOCK);
    ans.codes[1][0] = Box::new(KEY::KEY_ESC);
    ans.codes[2][0] = Box::new(KEY::KEY_TAB);
    ans.codes[3][0] = Box::new(SpaceCadet::new(left_paren, KEY::KEY_LEFTSHIFT));
    ans.codes[4][0] = Box::new(SpaceCadet::new(right_paren, KEY::KEY_RIGHTSHIFT));
    ans
}

fn cyclic_executor<F>(action: &mut F, hz_rate: u32) where F: FnMut() {
    let event_loop_rate = time::Duration::from_millis(1000 / u64::from(hz_rate));
    let mut warned = false;
    loop {
        let start = time::Instant::now();
        action();
        let end = time::Instant::now();
        let elapsed = end - start;
        if elapsed < event_loop_rate {
            let remaining_time = event_loop_rate - elapsed;
            std::thread::sleep(remaining_time);
        } else if !warned {
            println!("Event loop executing slower than {}hz.", hz_rate);
            warned = true;
        }
    }
}

fn tick(d: &mut LayeredKeyboardDriver) {
    d.clock_tick();
}

fn main() {

    let keyboard = KeyboardDriver {
        input: InputKeyboard::open("/dev/input/event4"),
        output: OutputKeyboard::new(None),
        matrix: VirtualKeyboardMatrix::new(get_keypad_matrix()),
    };

    let mut f = LayeredKeyboardDriver {
        driver: keyboard,
        layered_codes: Vec::new(),
        layer_attributes: LayerCollection::new(),
    };

    f.add_layer(LayerAttributes { name: "base".to_string(), enabled: true }, base_layer_keys());

    let mut update = || tick(&mut f);
    cyclic_executor(&mut update, 200);
}