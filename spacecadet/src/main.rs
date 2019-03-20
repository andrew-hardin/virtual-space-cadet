use std::time;
use libspacecadet::*;

fn get_keypad_matrix() -> KeyMatrix {
    return vec![
        vec![Some(SimpleKey::KEY_ESC)],
        vec![Some(SimpleKey::KEY_TAB)],
        vec![Some(SimpleKey::KEY_CAPSLOCK)],
        vec![Some(SimpleKey::KEY_LEFTSHIFT)],
        vec![Some(SimpleKey::KEY_RIGHTSHIFT)],
    ]
}

fn base_layer_keys() -> KeyCodeMatrix {
    let mut ans = KeyCodeMatrix::new((5, 1));
    ans.codes[0][0] = "KC_CAPSLOCK".parse().unwrap();
    ans.codes[1][0] = "KC_ESC".parse().unwrap();
    ans.codes[2][0] = "KC_TAB".parse().unwrap();
    ans.codes[3][0] = "SPACECADET(WRAP(KC_LEFTSHIFT, KC_9), KC_LEFTSHIFT)".parse().unwrap();
    ans.codes[4][0] = "SPACECADET(WRAP(KC_RIGHTSHIFT, KC_0), KC_RIGHTSHIFT)".parse().unwrap();
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

fn main() {
    let input = EvdevKeyboard::open("/dev/input/event4").unwrap();
    let output = UInputKeyboard::new(None).unwrap();

    let mut f = KeyboardDriver {
        input,
        output,
        matrix: VirtualKeyboardMatrix::new(get_keypad_matrix(), Some(VirtualKeyboardMatrix::default_hold_duration())),
        layered_codes: Vec::new(),
        layer_attributes: LayerCollection::new(),
    };

    f.add_layer(LayerAttributes { name: "base".to_string(), enabled: true }, base_layer_keys());

    let mut update = || f.clock_tick(time::Instant::now());
    cyclic_executor(&mut update, 200);
}