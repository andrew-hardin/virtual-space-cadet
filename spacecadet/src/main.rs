use std::time;
use libspacecadet::*;
use clap::{Arg, App, value_t};

struct ParsedArgs {
    device_path: String,
    matrix_path: String,
    layer_path: String,
    event_hz_rate: u32,
}

impl ParsedArgs {
    fn create() -> ParsedArgs {
        let matches = App::new("Space Cadet Driver")
            .version("1.0")
            .arg(Arg::with_name("device")
                .short("d")
                .long("device")
                .value_name("DEV")
                .required(true)
                .help("The path of a keyboard device.")
                .takes_value(true   ))
            .arg(Arg::with_name("matrix")
                .short("m")
                .long("matrix")
                .value_name("FILE")
                .required(true)
                .help("The path to the matrix file.")
                .takes_value(true))
            .arg(Arg::with_name("layer")
                .short("l")
                .long("layer")
                .value_name("FILE")
                .required(true)
                .help("The path to the layer file.")
                .takes_value(true))
            .arg(Arg::with_name("hz-rate")
                .long("hz-rate")
                .value_name("U32")
                .required(false)
                .help("Frequency rate of the primary event loop.")
                .takes_value(true))
            .get_matches();
        ParsedArgs {
            device_path: matches.value_of("device").unwrap().to_string(),
            matrix_path: matches.value_of("matrix").unwrap().to_string(),
            layer_path: matches.value_of("layer").unwrap().to_string(),
            event_hz_rate: value_t!(matches, "hz-rate", u32).unwrap_or(200) // 5ms.
        }
    }
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
    let args = ParsedArgs::create();

    let mut driver = KeyboardDriver {
        input: EvdevKeyboard::open(&args.device_path).unwrap(),
        output: UInputKeyboard::new(None).unwrap(),
        matrix: VirtualKeyboardMatrix::load(&args.matrix_path),
        layered_codes: Vec::new(),
        layer_attributes: LayerCollection::new(),
    };

    driver.load_layers(&args.layer_path);

    driver.verify().unwrap();

    let mut update = || driver.clock_tick(time::Instant::now());
    cyclic_executor(&mut update, args.event_hz_rate);
}