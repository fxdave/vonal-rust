use std::fs;

use freedesktop_desktop_entry::{default_paths, DesktopEntry, Iter};

use super::traits::{AppIndex, IndexApps};

/**
 * Creates application indexes from .desktop files
 */
#[derive(Default)]
pub struct DesktopIndexer {}
impl IndexApps for DesktopIndexer {
    fn index(&self) -> Vec<AppIndex> {
        Iter::new(default_paths())
            .filter_map(|path| {
                fs::read_to_string(&path)
                    .ok()
                    .as_ref()
                    .and_then(|bytes| DesktopEntry::decode(&path, &bytes).ok())
                    .and_then(|entry| {
                        let locale = None;
                        Some(AppIndex {
                            name: entry.name(locale)?.to_string(),
                            exec: entry.exec()?.to_string(),
                            genericName: entry.generic_name(locale).map(|s| s.to_string())
                        })
                    })
            })
            .collect()
    }
}
