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

    let mut without_uncommon_chars = query.to_string();
    without_uncommon_chars.retain(|c| !difference.contains(&InCaseSensitiveChar(c)));

    let number_of_deleted_chars = query.len() - without_uncommon_chars.len();
    (without_uncommon_chars, number_of_deleted_chars)
}

/// This struct represents the found segments matching to the query, and a position number for marking where was the first match.
struct SegmentsInfo {
    first_match: u64,
    missmatch_count: u64,
    segments: Vec<String>,
}

impl Clone for SegmentsInfo {
    fn clone(&self) -> Self {
        Self {
            first_match: self.first_match,
            missmatch_count: self.missmatch_count,
            segments: self.segments.clone(),
        }
    }
}

/// Matches query with the name and returns the `SegmentsInfo`
fn get_matching_segments_from_index(query: &str, name: &str, from: u64) -> SegmentsInfo {
    let mut query_iter = query.chars();
    #[allow(clippy::cast_possible_truncation)]
    let mut name_iter = name[from as usize..].chars();
    let mut first_match = 0;
    let mut name_counter = 0;

    let mut segments: Vec<String> = vec![];

    let mut next_query_char = query_iter.next();
    let mut next_name_char = name_iter.next();

    let mut missmatch_counter = 0;

    while let (Some(q), Some(n)) = (next_query_char, next_name_char) {
        if q.to_ascii_lowercase() == n.to_ascii_lowercase() {
            first_match = if first_match == 0 {
                name_counter
            } else {
                first_match
            };

            match segments.last_mut() {
                Some(s) => {
                    s.push(q);
                }
                None => segments.push(q.to_string()),
            };

            next_query_char = query_iter.next();
        } else if let Some(s) = segments.last() {
            if !s.is_empty() {
                segments.push(String::from(""));
            }
            missmatch_counter += 1;
        }
        next_name_char = name_iter.next();
        name_counter += 1;
    }

    SegmentsInfo {
        first_match: first_match + from,
        segments,
        missmatch_count: missmatch_counter,
    }
}

/// There are multiple `SegmentsInfo` to choose depending on where did we start the matcher.
/// This function tries to give a goodness value which we can use to compare multiple `SegmentsInfo`
fn calculate_goodness(segments: &[String]) -> u64 {
    let overall_length = segments.iter().fold(0, |acc, x| acc + x.len());

    let len_of_biggest_segment = segments
        .iter()
        .fold(0, |acc, x| if x.len() > acc { x.len() } else { acc });

    (len_of_biggest_segment * 2 + overall_length) as u64
}

/// Starts the matching from multiple positions and returns the best segments
fn get_matching_segments(query: &str, name: &str) -> SegmentsInfo {
    let mut segments_description = get_matching_segments_from_index(query, name, 0);
    let mut best_segments_description = segments_description.clone();
    let mut best_goodness = calculate_goodness(&best_segments_description.segments);

    while !segments_description.segments.is_empty() {
        segments_description =
            get_matching_segments_from_index(query, name, segments_description.first_match + 1);

        let goodness = calculate_goodness(&segments_description.segments);
        if goodness > best_goodness {
            best_goodness = goodness;
            best_segments_description = segments_description.clone();
        }
    }

    best_segments_description
}

#[allow(clippy::cast_precision_loss)]
fn calculate_segment_goodness(segments: &Vec<String>, query_length: usize) -> f64 {
    let segments_length: usize = segments.iter().fold(0usize, |acc, s| acc + s.len());
    let segments_length_goodness: f64 = segments_length as f64 / query_length as f64; // [0,1]
    let segments_fragment_goodness: f64 = 1.0 / segments.len() as f64; // (0;1]
    segments_length_goodness * segments_fragment_goodness
}

#[allow(clippy::cast_precision_loss)]
fn calculate_common_char_goodness(
    number_of_deleted_chars: usize,
    modified_query_length: usize,
) -> f64 {
    let del: f64 = number_of_deleted_chars as f64; // deleted chars
    let ol: f64 = modified_query_length as f64 + del; // original length
    (ol / (del + 1.0)) / ol // (0;1]
}

#[allow(clippy::cast_precision_loss)]
fn calculate_missmatch_goodness(number_of_missmatch: u64) -> f64 {
    1.0 / (number_of_missmatch as f64 + 1.0)
}

#[allow(clippy::cast_precision_loss)]
fn calculate_first_match_goodness(first_match: u64, max_name_length: usize) -> f64 {
    1f64 - first_match as f64 / max_name_length as f64 // [0;1]
}

#[allow(clippy::cast_precision_loss)]
fn calculate_length_goodness(name_length: usize) -> f64 {
    1f64 / name_length as f64
}

/// Represents some information about the final result which can be used to sort our dataset
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct FuzzyInfo {
    pub segments: Vec<String>,
    pub fitness: f64,
}

const MAX_NAME_LENGTH: usize = 127;

/// # The entry point of getting the `FuzzyInfo`.
///
/// This will reduce the query by uncommon chars.
/// After reducing the query, this will find **sequential** matching segments.
/// For e.g: We want to match `clomium` with `chromium`, this will give the segments: `["c", "omium"]`
///
pub fn get_fuzzy_info(query: &str, name: &str) -> FuzzyInfo {
    let (query, number_of_deleted_chars) = get_without_uncommon_chars(query, name);

    let new_length = min(name.len(), MAX_NAME_LENGTH);
    let name = &name[..new_length].to_owned();

    if query.is_empty() || name.is_empty() {
        return FuzzyInfo {
            segments: vec![],
            fitness: 0f64,
        };
    }

    let segments_info = get_matching_segments(&query, name);
    let first_match_goodness =
        calculate_first_match_goodness(segments_info.first_match, MAX_NAME_LENGTH);
    let segment_goodness = calculate_segment_goodness(
        &segments_info.segments,
        query.len() + number_of_deleted_chars,
    );
    let common_char_goodness = calculate_common_char_goodness(number_of_deleted_chars, query.len());
    let missmatch_goodness = calculate_missmatch_goodness(segments_info.missmatch_count);
    let length_goodness = calculate_length_goodness(name.len());

    FuzzyInfo {
        segments: segments_info.segments,
        fitness: (segment_goodness
            + common_char_goodness
            + missmatch_goodness
            + length_goodness
            + first_match_goodness)
            / 5f64,
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
        assert!(info.fitness > 0.0);
        assert!(info.fitness <= 1f64);
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
