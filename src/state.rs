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

#[derive(Clone, Data, Lens)]
pub struct VonalState {
    #[lens(name = "query_lens")]
    pub query: String,
    #[lens(name = "focused_row_id_lens")]
    pub focused_row_id: Option<u64>,
    pub results: im::Vector<AppEntry>,
}

impl VonalState {
    pub fn new() -> VonalState {
        VonalState {
            query: String::new(),
            results: im::vector![],
            focused_row_id: None,
        }
    }
}
