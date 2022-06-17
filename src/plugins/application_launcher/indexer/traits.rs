pub struct AppIndex {
    pub exec: String,
    pub name: String,
    pub genericName: Option<String>
}

pub trait IndexApps {
    fn index(&self) -> Vec<AppIndex>;
}
