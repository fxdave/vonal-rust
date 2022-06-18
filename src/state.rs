use druid::{im, lens, Data, Lens};

#[derive(Clone, Data, Lens)]
pub struct AppAction {
    pub name: String,
    pub command: String,
}

#[derive(Clone, Lens, Data)]
pub struct AppEntry {
    pub name: String,
    pub actions: im::Vector<AppAction>,
}

#[derive(Clone, Lens, Data)]
pub struct Focusable<T> {
    pub focusable: T,
    pub focused: bool
}

#[derive(Clone, Data, Lens)]
pub struct VonalState {
    #[lens(name = "query_lens")]
    pub query: String,
    pub results: im::Vector<Focusable<AppEntry>>,
}

impl VonalState {
    pub fn new() -> VonalState {
        VonalState {
            query: String::new(),
            results: im::vector![],
        }
    }
}
