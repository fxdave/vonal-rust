use crate::state::AppAction;

pub struct AppIndex {
    pub exec: String,
    pub name: String,
    pub generic_name: Option<String>,
    pub actions: Vec<AppAction>,
}

pub trait IndexApps {
    fn index(&self) -> Vec<AppIndex>;
}
