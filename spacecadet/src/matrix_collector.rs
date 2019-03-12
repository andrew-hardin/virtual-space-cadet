use evdev_rs as evdev;

fn main() {
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
    let device_path = "/dev/input/event4";

    // Collect the matrix via user input.
    let matrix = collect_matrix(device_path);
    let row_count = matrix.len();
    let col_count = matrix.iter().map(|x| x.len()).max().unwrap();

    // Print the matrix.
    for r in 0..row_count {
        let loop_end = std::cmp::min(matrix[r].len(), col_count);
        for j in 0..loop_end {
            print!("{:?} ", matrix[r][j]);
        }
        for _j in loop_end..col_count {
            print!("____");
        }
        println!();
    }
    /*
    Two files:
        Virtual Matrix: maps keys to locations.
        Layers: maps locations to behavior.

    We create both a virtual matrix and a layers file by default.
    This provides a default behavior that can serve as a blank slate.
    */
    // FromStr() and ToStr()
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
            1 => matrix.last_mut().unwrap().push(last_key.clone().unwrap()),  // store
            2 => matrix.push(Vec::new()),  // new row
            3 => break(),  // finished
            _ => panic!()
        }
    }

    // Trim off the last empty row.
    matrix.remove(matrix.len() - 1);
    matrix
}