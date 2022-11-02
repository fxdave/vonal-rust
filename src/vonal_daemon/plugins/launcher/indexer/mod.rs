use self::traits::{AppIndex, IndexApps};

pub mod desktop;
pub mod path;
pub mod traits;

#[derive(Default)]
pub struct Indexer {}

impl IndexApps for Indexer {
    fn index(&self) -> Vec<AppIndex> {
        desktop::Desktop::default().index()
    }
}
