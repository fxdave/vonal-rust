use super::indexer::traits::AppIndex;

mod fuzzy;

struct FuzzyAppInfo {
    pub index: AppIndex,
    pub fuzzybuzz: String,
}

pub struct AppMatch<'a> {
    pub index: &'a AppIndex,
    fuzzy_info: fuzzy::FuzzyInfo,
}

pub struct Finder {
    cache: Vec<FuzzyAppInfo>,
}

impl Finder {
    pub fn new<I: IntoIterator<Item = AppIndex>>(indices: I) -> Self {
        Self {
            cache: indices
                .into_iter()
                .map(|index| FuzzyAppInfo {
                    fuzzybuzz: index.name.clone()
                        + &index.exec
                        + index.generic_name.as_ref().unwrap_or(&String::new()),
                    index,
                })
                .collect(),
        }
    }

    /// TODO: This can be way faster..
    pub fn find(&self, query: &str) -> Vec<AppMatch<'_>> {
        let mut results: Vec<_> = self
            .cache
            .iter()
            .map(|app| AppMatch {
                index: &app.index,
                fuzzy_info: fuzzy::get_fuzzy_info(query, &app.fuzzybuzz),
            })
            .collect();

        results.sort_unstable_by(|a, b| b.fuzzy_info.fitness.total_cmp(&a.fuzzy_info.fitness));
        results.truncate(10);
        results
    }
}
