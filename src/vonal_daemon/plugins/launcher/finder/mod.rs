use super::indexer::traits::AppIndex;

mod fuzzy;
mod limited_selection_sort;

#[derive(Debug)]
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
    cache: Vec<AppIndex>,
}

const MAXIMUM_NUMBER_OF_RESULTS: usize = 10;

impl Finder {
    pub fn new(indices: Vec<AppIndex>) -> Self {
        Self { cache: indices }
    }

    pub fn find(&self, query: &str) -> Vec<AppMatch<'_>> {
        let mut results: Vec<_> = self
            .cache
            .iter()
            .map(|app| {
                let generic_name = app.generic_name.clone().unwrap_or_default();
                let name_match = fuzzy::get_fuzzy_info(query, &app.name);
                let mut generic_name_match = fuzzy::get_fuzzy_info(query, &generic_name);
                let mut exec_match = fuzzy::get_fuzzy_info(query, &app.exec);

                // 1. match by name is preferred
                generic_name_match.fitness -= 10;
                exec_match.fitness -= 10;

                let mut fuzzy_info = [name_match, generic_name_match, exec_match]
                    .into_iter()
                    .max_by_key(|i| i.fitness)
                    .unwrap();

                // 2. quality correction
                let has_action = !app.actions.is_empty();
                let has_generic_name = app.generic_name.is_some();
                fuzzy_info.fitness += if has_action { 10 } else { 0 };
                fuzzy_info.fitness += if has_generic_name { 10 } else { 0 };

                AppMatch {
                    index: app,
                    fuzzy_info,
                }
            })
            .collect();

        limited_selection_sort::sort(&mut results, MAXIMUM_NUMBER_OF_RESULTS);
        results.truncate(MAXIMUM_NUMBER_OF_RESULTS);
        results
    }
}
