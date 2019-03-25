use evdev_rs as evdev;
use std::io::Write;
use std::fs::File;
use clap::{Arg, App};

struct ParsedArgs {
    device_path: String,
    matrix_path: String,
    layer_path: String,
}
impl ParsedArgs {
    fn create() -> ParsedArgs {
        let matches = App::new("Matrix Collector")
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
                .help("The path to output matrix file.")
                .takes_value(true))
            .arg(Arg::with_name("layer")
                .short("l")
                .long("layer")
                .value_name("FILE")
                .help("The path to output layer file.")
                .takes_value(true))
            .get_matches();
        ParsedArgs {
            device_path: matches.value_of("device").unwrap().to_string(),
            matrix_path: matches.value_of("matrix").unwrap_or("matrix.json").to_string(),
            layer_path: matches.value_of("layer").unwrap_or("layers.json").to_string(),
        }
    }
}

fn main() {
    let args = ParsedArgs::create();

    let instructions =
        "Matrix Collector\n\
        ----------------\n\
        This application organizes the keyboard keys into an NxM matrix.\n\
        \n\
        Instructions: \n\
        \t1. Start in the top left of your keyboard, and press every key on the first row.\n\
        \t2. Tap the last key on the row twice to start the next row.\n\
        \t3. Repeat steps 1-2 until the last row.\n\
        \t4. Tap the last key on the last row three times to finish collecting the matrix.\n\
        \n\
        In short:\n\
        \tOne tap -> collect key\n\
        \tTwo taps -> goto next row\n\
        \tThree taps -> finish\n";
    println!("{}", instructions);

    // Collect the matrix via user input.
    let matrix = collect_matrix(&args.device_path);
    let col_count = matrix.iter().map(|x| x.len()).max().unwrap();


    // Create two documents:
    //  Matrix = maps keys to matrix locations.
    //  Layers = behavior of matrix locations.
    //
    // These documents could be created with a JSON library, but we'll
    // create the documents using straight strings because we want the
    // formatting to be perfect.

    // Convert the every key code to a string.
    let mut matrix : Vec<Vec<String>> = matrix.iter().map(|row| {
        row.iter().map(|column| {
            format!("\"{:?}\"", column).replacen("KEY", "KC", 1)
        }).collect()
    }).collect();

    // Find the maximum width for each column.
    let mut character_widths = vec![0; col_count];
    for row in matrix.iter() {
        for col in row.iter().enumerate() {
            character_widths[col.0] = std::cmp::max(col.1.len(), character_widths[col.0]);
        }
    }

    // Left pad each value to the max width of the column.
    for row in matrix.iter_mut() {
        for col in row.iter_mut().enumerate() {
            *col.1 = format!("{:>width$}", col.1, width = character_widths[col.0])
        }
    }

    // Some rows of the matrix may be missing keys (i.e. jagged).
    // Fill in the missing keys.
    for row in matrix.iter_mut() {
        while row.len() < col_count {
            let empty = "_".repeat(character_widths[row.len()] - 2); // don't include quotes.
            row.push(format!("\"{}\"", empty));
        }
    }

    // Combine strings.
    // This is the literal JSON value that's written to the output JSON documents.
    let left_pad_count = 4;
    let matrix = matrix.iter().map(|x| {
        // Pretty clear I'm not a rust expert...
        " ".repeat(left_pad_count) + &"[ ".to_string() + &x.join(", ") + " ]"
    }).collect::<Vec<String>>().join(",\n");

    // This is ugly, but I want the output to have a very specific format.
    let matrix_document = format!(r#"{{
  "device": "{}",
  "matrix": [
{}
  ]
}}"#, args.device_path, matrix);

    let layer_document = format!(r#"{{
  "layer_order": [ "base" ],
  "base": {{
    "enabled": true,
    "keys" : [
{}
    ]
  }}
}}"#, matrix);

    // Write the two output files.
    write_to_file(&args.matrix_path, &matrix_document);
    write_to_file(&args.layer_path, &layer_document);
}

fn write_to_file(path: &str, content: &str) {
    let mut f = File::create(&path).unwrap();
    f.write(content.as_bytes()).unwrap();
}

fn collect_matrix(name: &str) -> Vec<Vec<evdev::enums::EV_KEY>> {
    let f = std::fs::File::open(name).unwrap();
    let mut device = evdev::Device::new_from_fd(&f).unwrap();
    device.grab(evdev::GrabMode::Grab).unwrap();
    println!("Grabbed exclusive access to \"{}\"; begin pressing keys...", name);

    // Read keys until one is tapped three times.
    let mut matrix = vec![vec![]];
    let mut last_key = None;
    let mut last_key_press_count = 0;
    loop {
        // Read the next key, and ignore any events that aren't presses.
        let key = device.next_event(evdev_rs::NORMAL | evdev_rs::BLOCKING).unwrap().1;
        if key.value != 1 {
            continue;
        }

        // Get the key event code; ignore all other event types.
        let event_code = match key.event_code {
            evdev::enums::EventCode::EV_KEY(key) => Some(key),
            _ => None
        };
        if event_code.is_none() {
            continue;
        }
        let event_code = event_code.unwrap();

        // Different or same key as last time?
        if last_key.is_some() && event_code == last_key.clone().unwrap() {
            last_key_press_count += 1;
        } else {
            last_key = Some(event_code);
            last_key_press_count = 1;
        }

        match last_key_press_count {
            1 => {
                matrix.last_mut().unwrap().push(last_key.clone().unwrap());
                print!("{:?} ", last_key.clone().unwrap());
                std::io::stdout().flush().unwrap();
            },
            2 => {
                matrix.push(Vec::new());
                println!();
            },
            3 => break(),
            _ => panic!()
        }
    }

    // Trim off the last empty row.
    matrix.remove(matrix.len() - 1);
    matrix
}