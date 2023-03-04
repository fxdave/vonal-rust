use super::indexer::traits::AppIndex;

mod fuzzy;
mod limited_selection_sort;

struct FuzzyAppInfo {
    pub index: AppIndex,
    pub fuzzybuzz: String,
}

pub struct AppMatch<'a> {
    pub index: &'a AppIndex,
    fuzzy_info: fuzzy::FuzzyInfo,
}

impl<'a> PartialEq for AppMatch<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.fuzzy_info.fitness.eq(&other.fuzzy_info.fitness)
    }
}
impl<'a> PartialOrd for AppMatch<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.fuzzy_info
            .fitness
            .partial_cmp(&other.fuzzy_info.fitness)
    }
}

pub struct Finder {
    cache: Vec<FuzzyAppInfo>,
}

const MAXIMUM_NUMBER_OF_RESULTS: usize = 10;

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

    pub fn find(&self, query: &str) -> Vec<AppMatch<'_>> {
        let mut results: Vec<_> = self
            .cache
            .iter()
            .map(|app| AppMatch {
                index: &app.index,
                fuzzy_info: fuzzy::get_fuzzy_info(query, &app.fuzzybuzz),
            })
            .collect();

        limited_selection_sort::sort(&mut results, MAXIMUM_NUMBER_OF_RESULTS);
        results.truncate(MAXIMUM_NUMBER_OF_RESULTS);
        results
    }
}
