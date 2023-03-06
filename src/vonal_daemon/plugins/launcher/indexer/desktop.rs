use super::traits::{AppAction, AppIndex};
use freedesktop_desktop_entry::{default_paths, DesktopEntry, Iter};
use std::fs;

/**
 * Creates application indexes from .desktop files
 */
pub fn index() -> Vec<AppIndex> {
    Iter::new(default_paths())
        .filter_map(|path| {
            fs::read_to_string(&path)
                .ok()
                .as_ref()
                .and_then(|bytes| DesktopEntry::decode(&path, bytes).ok())
                .and_then(|entry| {
                    let locale = None;
                    Some(AppIndex {
                        name: entry.name(locale)?.to_string(),
                        exec: entry.exec()?.into(),
                        generic_name: entry.generic_name(locale).map(|s| s.to_string()),
                        actions: entry
                            .actions()
                            .map(|actions| {
                                actions
                                    .split(';')
                                    .filter_map(|action| {
                                        Some(AppAction {
                                            name: entry.action_name(action, None)?.to_string(),
                                            command: entry.action_exec(action)?.to_string(),
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default(),
                    })
                })
        })
        .collect()
}
