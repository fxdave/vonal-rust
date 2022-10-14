
#[derive(Clone)]
pub struct AppAction {
    pub name: String,
    pub command: String,
}

// TODO: remove clone
#[derive(Clone)]
pub struct AppIndex {
    pub exec: String,
    pub name: String,
    pub generic_name: Option<String>,
    pub actions: Vec<AppAction>,
}

pub trait IndexApps {
    fn index(&self) -> Vec<AppIndex>;
}
