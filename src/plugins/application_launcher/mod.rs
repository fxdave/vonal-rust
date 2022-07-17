use druid::im;

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
                actions: im::vector![AppAction {
                    name: result.index.name.clone(),
                    command: result.index.exec.clone()
                }] + result.index.actions.clone().into(),
            })
            .collect()
    }
}
