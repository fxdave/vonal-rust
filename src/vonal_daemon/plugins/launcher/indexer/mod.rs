use egui::epaint::ahash::{HashMap, HashMapExt};

use self::traits::AppIndex;

pub mod desktop;
pub mod path;
pub mod traits;

#[derive(Default)]
pub struct Indexer {}

impl Indexer {
    /// index apps with multiple indexers, and deduplicate the result
    pub fn index(&self, index_path: bool) -> Vec<AppIndex> {
        let desktop_indices = desktop::index();
        if !index_path {
            return desktop_indices;
        }

        let path_indices = path::index();

        let mut final_results = HashMap::new();

        for i in path_indices {
            final_results.insert(get_exec_id(&i.exec), i);
        }

        for i in desktop_indices {
            final_results.insert(get_exec_id(&i.exec), i);
        }

        final_results.into_values().collect()
    }
}

fn get_exec_id(id: &str) -> String {
    id.rsplit('/')
        .next()
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .to_lowercase()
}
