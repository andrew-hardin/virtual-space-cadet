use evdev_rs as evdev;
use crate::KeyStats;


/// A wrapper around an input keyboard device (e.g. /dev/input/event4).
pub struct InputKeyboard {
    _file_descriptor: std::fs::File,
    device: evdev::Device,
    pub stats: KeyStats
}

impl InputKeyboard {

    /// Open an input keyboard.
    ///
    /// Behind the scenes we're opening a non-blocking file descriptor and
    /// constructing a evdev device.
    pub fn open(path: &str) -> InputKeyboard {
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
            device,
            stats: KeyStats::new(),
        }
    }

    /// (Non-blocking) read all pending events from the device.
    /// Immediately returns an empty vector if no events happened.
    pub fn read_events(&mut self) -> Vec<evdev::InputEvent> {
        let mut ans= Vec::new();
        loop {
            // TODO: based on the library example, there may be an
            //       edge case related to sync that's not being handled.
            let a = self.device.next_event(evdev::NORMAL);
            match a {
                Ok(k) => {
                    // We only forward on EV_KEY events.
                    match k.1.event_type {
                        evdev::enums::EventType::EV_KEY => { ans.push(k.1); }
                        _ => ()
                    }

                }
                Err(_) => break
            }
        }

        for i in ans.iter() {
            self.stats.increment(i.value.into());
        }

        ans
    }
}