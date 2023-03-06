use std::cmp::min;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

struct InCaseSensitiveChar(char);

impl Ord for InCaseSensitiveChar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .to_ascii_lowercase()
            .cmp(&other.0.to_ascii_lowercase())
    }
}

impl PartialOrd for InCaseSensitiveChar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            self.0
                .to_ascii_lowercase()
                .cmp(&other.0.to_ascii_lowercase()),
        )
    }
}

impl PartialEq for InCaseSensitiveChar {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_ascii_lowercase() == other.0.to_ascii_lowercase()
    }
}

impl Hash for InCaseSensitiveChar {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_ascii_lowercase().hash(state);
    }
}

impl Eq for InCaseSensitiveChar {}

/// In order to reach the best fuzzy match, this function makes a new query string
/// without those chars that cannot be found in the name.
///
/// For e.g: We want to match `clomium` with `chromium`,
/// but this would give us only one character long match: the `["c"]`.
/// Instead of this, we will find `comium` so we will get `["c", "omium"]`.
/// So in this example, this function makes `comium` from `clomium`.
#[allow(clippy::redundant_closure)]
fn get_without_uncommon_chars(query: &str, name: &str) -> (String, usize) {
    let a: HashSet<InCaseSensitiveChar> = query.chars().map(|x| InCaseSensitiveChar(x)).collect();
    let b: HashSet<InCaseSensitiveChar> = name.chars().map(|x| InCaseSensitiveChar(x)).collect();

    let difference: HashSet<&InCaseSensitiveChar> = a.difference(&b).collect();

    let mut query_without_uncommon_chars = query.to_string();
    query_without_uncommon_chars.retain(|c| !difference.contains(&InCaseSensitiveChar(c)));

    let number_of_deleted_chars = difference.len();
    (query_without_uncommon_chars, number_of_deleted_chars)
}

/// This struct represents the found segments matching to the query,
/// and a position number for marking where was the first match.
#[derive(Clone)]
struct SegmentsInfo {
    first_match: Option<usize>,
    missmatch_count: usize,
    segments: Vec<String>,
}

impl SegmentsInfo {
    fn get_fitness(&self) -> i32 {
        // 1. bigger cardinality wins
        // 2. larger groups wins
        let mut fitness = self
            .segments
            .iter()
            .map(|s| {
                let len = s.len() as i32;
                len + (len - 1)
            })
            .sum::<i32>()
            * 1000;

        // 3. punish late match
        fitness -= self.first_match.unwrap_or_default() as i32 * 30;

        // 4. smaller distances wins
        fitness -= self.missmatch_count as i32 * 10;

        return fitness;
    }
}

/// Match query with the name and return the `SegmentsInfo`
fn get_matching_segments_from_index(query: &str, name: &str, from: usize) -> SegmentsInfo {
    let mut segments: Vec<String> = vec![];
    let mut first_match_pos = None;
    let mut visited_name_chars = 0;
    let mut missmatch_counter = 0;

    #[allow(clippy::cast_possible_truncation)]
    let mut name_iter = name[from..].chars();
    let mut query_iter = query.chars();

    let mut actual_query_char = query_iter.next();
    let mut actual_name_char = name_iter.next();

    while let (Some(q), Some(n)) = (actual_query_char, actual_name_char) {
        if q.to_ascii_lowercase() == n.to_ascii_lowercase() {
            if first_match_pos.is_none() {
                first_match_pos = Some(visited_name_chars)
            }

            match segments.last_mut() {
                Some(s) => s.push(q),
                None => segments.push(q.to_string()),
            };

            actual_query_char = query_iter.next();
        } else if let Some(s) = segments.last() {
            if !s.is_empty() {
                segments.push(String::from(""));
            }
            missmatch_counter += 1;
        }
        actual_name_char = name_iter.next();
        visited_name_chars += 1;
    }

    let last_is_empty = if let Some(l) = segments.last() {
        l.is_empty()
    } else {
        false
    };

    if last_is_empty {
        segments.remove(segments.len() - 1);
    }

    SegmentsInfo {
        first_match: first_match_pos.map(|pos| pos + from),
        segments,
        missmatch_count: missmatch_counter,
    }
}

/// There are multiple `SegmentsInfo` to choose depending on where did we start the matcher.
/// This function tries to give a goodness value which we can use to compare multiple `SegmentsInfo`
fn calculate_goodness(segments: &[String]) -> usize {
    let overall_length = segments.iter().fold(0, |acc, x| acc + x.len());

    let len_of_biggest_segment = segments
        .iter()
        .fold(0, |acc, x| if x.len() > acc { x.len() } else { acc });

    len_of_biggest_segment * 2 + overall_length
}

/// Start the matching from multiple positions and return the best segments info
fn get_matching_segments(query: &str, name: &str) -> SegmentsInfo {
    let mut segments_description = get_matching_segments_from_index(query, name, 0);
    let mut best_segments_description = segments_description.clone();
    let mut best_goodness = calculate_goodness(&best_segments_description.segments);

    // It doesn't visit every letter since that's done in the [get_matching_segments_from_index] function.
    // If we don't have any matches for pos 0 then we won't have any matches for pos 1,2,3,.. either
    // if we have a match for pos 5 then we don't get the same match for position 6,
    // that's why we only check the segments for (last match position) + 1 until no match.
    while !segments_description.segments.is_empty() {
        let from_idx = segments_description.first_match.unwrap_or(0) + 1;
        segments_description = get_matching_segments_from_index(query, name, from_idx);

        let goodness = calculate_goodness(&segments_description.segments);
        if goodness > best_goodness {
            best_goodness = goodness;
            best_segments_description = segments_description.clone();
        }
    }

    best_segments_description
}

/// Represent some information about the final result which can be used to sort our dataset
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct FuzzyInfo {
    pub segments: Vec<String>,
    pub fitness: i32,
}

const MAX_NAME_LENGTH: usize = 127;

/// # The entry point of getting the `FuzzyInfo`.
///
/// This will reduce the query by uncommon chars.
/// After reducing the query, this will find **sequential** matching segments.
/// For e.g: We want to match `clomium` with `chromium`, this will give the segments: `["c", "omium"]`
///
pub fn get_fuzzy_info(query: &str, name: &str) -> FuzzyInfo {
    let (query, _) = get_without_uncommon_chars(query, name);

    let new_length = min(name.len(), MAX_NAME_LENGTH);
    let name = &name[..new_length];

    if query.is_empty() || name.is_empty() {
        return FuzzyInfo {
            segments: vec![],
            fitness: 0,
        };
    }

    let segments_info = get_matching_segments(&query, name);
    if segments_info.first_match.is_none() {
        return FuzzyInfo {
            segments: segments_info.segments,
            fitness: 0,
        };
    }

    let mut fitness = segments_info.get_fitness();

    fitness -= name.len() as i32;

    FuzzyInfo {
        fitness,
        segments: segments_info.segments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_fuzzy_info() {
        let name = "chromium".to_string();
        let query = "chomium".to_string();
        let info = get_fuzzy_info(&query, &name);

        assert_eq!(info.segments, vec!["ch", "omium"]);
        assert!(info.fitness > 0);
    }

    #[test]
    fn test_fitness1() {
        let info1 = get_fuzzy_info("clo", "chromium");
        let info2 = get_fuzzy_info("clo", "chrootas");
        let info3 = get_fuzzy_info("clo", "chromapr");
        println!("{:?}", info1);
        println!("{:?}", info2);
        println!("{:?}", info3);

        assert_eq!(info1.segments, vec!["c", "o"]);
        assert_eq!(info1.fitness, info2.fitness);
        assert_eq!(info1.fitness, info3.fitness);
        assert_eq!(info2.fitness, info3.fitness);
    }

    #[test]
    fn test_fitness2() {
        let info1 = get_fuzzy_info("slack", "badlocks");
        let info2 = get_fuzzy_info("slack", "com.slack.Slack");

        assert!(info1.fitness < info2.fitness);
    }

    #[test]
    fn test_fitness3() {
        let info1 = get_fuzzy_info("chromim", "commmium Web Browser");
        let info2 = get_fuzzy_info("chromim", "Chromium Web Browser");
        println!("{}", info1.fitness);
        println!("{}", info2.fitness);

        assert!(info1.fitness < info2.fitness);
    }

    #[test]
    fn test_shorter_first() {
        let info1 = get_fuzzy_info("files", "filess");
        let info2 = get_fuzzy_info("files", "files");
        println!("{}", info1.fitness);
        println!("{}", info2.fitness);

        assert!(info1.fitness < info2.fitness);
    }

    #[test]
    fn test_fitness4() {
        let info1 = get_fuzzy_info("files", "something files");
        let info2 = get_fuzzy_info("files", "files something");
        println!("{}", info1.fitness);
        println!("{}", info2.fitness);

        assert!(info1.fitness < info2.fitness);
    }

    #[test]
    fn test_fitness5() {
        let info1 = get_fuzzy_info("clomium", "alikialiki");
        let info2 = get_fuzzy_info("clomium", "Chromium/usr/bin/chromium %UWeb Browser");
        println!("{}", info1.fitness);
        println!("{}", info2.fitness);

        assert!(info1.fitness < info2.fitness);
    }

    #[test]
    fn test_fitness6() {
        let info1 = get_fuzzy_info("clomiumbrowser", "DevhelpdevhelpAPI Documentation Browser");
        let info2 = get_fuzzy_info("clomiumbrowser", "Chromium/usr/bin/chromium %UWeb Browser");
        println!("{}", info1.fitness);
        println!("{}", info2.fitness);
        assert!(info1.fitness < info2.fitness);
    }

    #[test]
    fn test_fitness7() {
        let info1 = get_fuzzy_info("clomium", "commcomm");
        let info2 = get_fuzzy_info("clomium", "Chromium/usr/bin/chromium %UWeb Browser");
        println!("{}", info1.fitness);
        println!("{}", info2.fitness);
        assert!(info1.fitness < info2.fitness);
    }

    #[test]
    fn test_fitness8() {
        let info1 = get_fuzzy_info("clomium", "gcloud-crc32cgcloud-crc32c");
        let info2 = get_fuzzy_info("clomium", "Chromium/usr/bin/chromium %UWeb Browser");
        println!("{}", info1.fitness);
        println!("{}", info2.fitness);
        assert!(info1.fitness < info2.fitness);
    }

    #[test]
    fn test_distance() {
        let info1 = get_fuzzy_info("ac", "abcdefghijkl");
        let info2 = get_fuzzy_info("ad", "abcdefghijkl");
        let info3 = get_fuzzy_info("ae", "abcdefghijkl");
        println!("{}", info1.fitness);
        println!("{}", info2.fitness);
        println!("{}", info3.fitness);
        assert!(info1.fitness > info2.fitness);
        assert!(info2.fitness > info3.fitness);
    }
}
