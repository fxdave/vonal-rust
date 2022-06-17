use druid::im;

use crate::state::AppEntry;

pub mod application_launcher;
pub trait Plugin {
    fn load() -> Self;
    fn search(&self, query: &str) -> im::Vector<AppEntry>;
}