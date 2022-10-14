use self::traits::{AppIndex, IndexApps};

pub mod desktop_indexer;
pub mod path_indexer;
pub mod traits;

#[derive(Default)]
pub struct Indexer {}

impl IndexApps for Indexer {
    fn index(&self) -> Vec<AppIndex> {
        desktop_indexer::DesktopIndexer::default().index()
    }
}
