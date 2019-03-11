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
    pub fn open(path: &str) -> Result<EvdevKeyboard, String> {
        use std::fs::OpenOptions;
        use std::os::unix::fs::OpenOptionsExt;
        let file_descriptor = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path);

        match file_descriptor {
            Ok(fd) => {
                let mut device = evdev::Device::new_from_fd(&fd).unwrap();

                // Check that the device supports keys.
                if device.has(&evdev::enums::EventType::EV_KEY) {
                    device.grab(evdev::GrabMode::Grab).unwrap();

                    Ok(EvdevKeyboard {
                        _file_descriptor: fd,
                        device,
                        stats: KeyStats::new(),
                    })
                } else {
                    Err(format!("Device isn't a keyboard: \"{}\" doesn't support EV_KEY events.", path))
                }
            }
            Err(e) => {
                Err(format!("{} on {}", e.to_string(), path))
            }
        }
    }

}

impl InputKeyboard for EvdevKeyboard {

    /// (Non-blocking) Read all pending events from the device.
    /// Immediately returns an empty vector if no events happened.
    fn read_events(&mut self) -> Vec<evdev::InputEvent> {
        let mut ans= Vec::new();
        loop {
            match self.device.next_event(evdev::NORMAL) {
                Ok(k) => {
                    match k.0 {
                        evdev::ReadStatus::Success => {
                            match k.1.event_type {
                                evdev::enums::EventType::EV_KEY => { ans.push(k.1); }
                                _ => (/* We only handle EV_KEY events */)
                            }
                        }
                        evdev::ReadStatus::Sync => {
                            panic!("Unhandled SYNC read status received.");
                        }
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
