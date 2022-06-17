use crate::state::{AppAction, AppEntry};

use self::indexer::traits::IndexApps;

use super::Plugin;

mod finder;
mod indexer;

pub struct ApplicationLauncherPlugin {
    finder: finder::Finder,
}

impl Plugin for ApplicationLauncherPlugin {
    fn load() -> Self {
        let indices = indexer::Indexer::default().index();
        let finder = finder::Finder::new(indices);
        Self { finder }
    }

    fn search(&self, query: &str) -> druid::im::Vector<AppEntry> {
        self.finder
            .find(query)
            .into_iter()
            .map(|result| AppEntry {
                name: result.index.name.to_owned(),
                actions: druid::im::vector![AppAction {
                    name: "Open".into(),
                    command: result.index.exec.to_owned(),
                }],
            })
            .collect()
    }
}
