extern crate inotify;
use std::error::Error;

use inotify::{EventMask, Inotify, WatchMask};

use super::get_config_file_path;

pub struct Watcher {
    inotify: Inotify,
    buffer: [u8; 4096],
}

#[derive(Debug)]
pub enum ConfigEvent {
    Created,
    Deleted,
    Modified,
    None,
}

impl Watcher {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let inotify = Inotify::init().expect("Failed to initialize inotify");
        let path = get_config_file_path();
        inotify
            .watches().add(
                path,
                WatchMask::MODIFY | WatchMask::CREATE | WatchMask::DELETE,
            )
            .expect("Failed to add inotify watch");

        Ok(Self {
            inotify,
            buffer: [0; 4096],
        })
    }

    pub fn get_stream(&mut self) -> impl Iterator<Item = ConfigEvent> + '_ {
        std::iter::repeat(())
            .map(|_nothing| {
                let events = self
                    .inotify
                    .read_events_blocking(&mut self.buffer)
                    .expect("Failed to read inotify events");

                events
                    .map(|event| {
                        if event.mask.contains(EventMask::CREATE) {
                            ConfigEvent::Created
                        } else if event.mask.contains(EventMask::DELETE) {
                            ConfigEvent::Deleted
                        } else if event.mask.contains(EventMask::MODIFY) {
                            ConfigEvent::Modified
                        } else {
                            ConfigEvent::None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
    }
}
