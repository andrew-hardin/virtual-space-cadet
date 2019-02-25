use std::time;
use libspacecadet::*;

const EVENT_LOOP_HZ_RATE: u64 = 200;
const EVENT_LOOP_RATE: time::Duration = time::Duration::from_millis(1000 / EVENT_LOOP_HZ_RATE);

fn get_keypad_matrix() -> KeyMatrix {
    return vec![
        vec![Some(KEY::KEY_NUMLOCK), Some(KEY::KEY_KPSLASH), Some(KEY::KEY_KPASTERISK)],
        vec![Some(KEY::KEY_KP7), Some(KEY::KEY_KP8), Some(KEY::KEY_KP9)],
        vec![Some(KEY::KEY_KP4), Some(KEY::KEY_KP5), Some(KEY::KEY_KP6)],
        vec![Some(KEY::KEY_KP1), Some(KEY::KEY_KP2), Some(KEY::KEY_KP3)]
    ]
}

fn base_layer_keys() -> KeyCodeMatrix {
    let mut ans = KeyCodeMatrix::new((4,3));
    ans.codes[0][0] = Box::new(ToggleLayerKey {
        layer_name: "second".to_string()
    });
    ans.codes[0][1] = Box::new(MomentarilyEnableLayerKey {
        layer_name: "second".to_string()
    });
    ans.codes[0][2] = Box::new(HoldEnableLayerPressKey::new("second", KEY::KEY_A));
    ans.codes[3][0] = Box::new(KEY::KEY_A);
    ans.codes[3][1] = Box::new(MacroKey {
        play_macro_when: KeyStateChange::Released,
        keys: vec![KEY::KEY_H, KEY::KEY_E, KEY::KEY_L, KEY::KEY_L, KEY::KEY_O],
    });
    ans
}

fn secondary_layer_keys() -> KeyCodeMatrix {
    let mut ans = KeyCodeMatrix::new((4,3));
    ans.codes[3][0] = Box::new(KEY::KEY_2);
    ans
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

fn tick(d: &mut LayeredKeyboardDriver) {
    d.clock_tick();
}

fn main() {

    let keyboard = KeyboardDriver {
        input: InputKeyboard::open("/dev/input/event4"),
        output: OutputKeyboard::new(),
        matrix: VirtualKeyboardMatrix::new(get_keypad_matrix()),
    };

    let mut f = LayeredKeyboardDriver {
        driver: keyboard,
        layered_codes: Vec::new(),
        layer_attributes: LayerCollection::new(),
    };

    f.add_layer(LayerAttributes { name: "base".to_string(), enabled: true }, base_layer_keys());
    f.add_layer(LayerAttributes { name: "second".to_string(), enabled: false }, secondary_layer_keys());

    let mut update = || tick(&mut f);
    cyclic_executor(&mut update);
}