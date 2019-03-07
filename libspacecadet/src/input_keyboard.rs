use evdev_rs as evdev;
use crate::KeyStats;

/// An interface for keyboards that receive input events.
pub trait InputKeyboard {
    /// Read events from the input device (non-blocking).
    fn read_events(&mut self) -> Vec<evdev::InputEvent>;
    /// Get statistics on the type of events that have been read.
    fn get_stats(&self) -> KeyStats;
}

/// A wrapper around an input keyboard device (e.g. `/dev/input/event4`).
pub struct EvdevKeyboard {
    _file_descriptor: std::fs::File,
    device: evdev::Device,
    stats: KeyStats
}

impl EvdevKeyboard {
    /// Open an input keyboard. Behind the scenes we're opening a
    /// non-blocking file descriptor and constructing an evdev device.
    pub fn open(path: &str) -> EvdevKeyboard {
        use std::fs::OpenOptions;
        use std::os::unix::fs::OpenOptionsExt;
        let file_descriptor = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
            .unwrap();

        let mut device = evdev::Device::new_from_fd(&file_descriptor).unwrap();
        device.grab(evdev::GrabMode::Grab).unwrap();

        EvdevKeyboard {
            _file_descriptor: file_descriptor,
            device,
            stats: KeyStats::new(),
        }
    }
}

impl InputKeyboard for EvdevKeyboard {

    /// (Non-blocking) Read all pending events from the device.
    /// Immediately returns an empty vector if no events happened.
    fn read_events(&mut self) -> Vec<evdev::InputEvent> {
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

    fn get_stats(&self) -> KeyStats { self.stats }
}
