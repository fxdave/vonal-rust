pub struct AppIndex {
    pub exec: String,
    pub name: String,
    pub generic_name: Option<String>
}

pub trait IndexApps {
    fn index(&self) -> Vec<AppIndex>;
}
