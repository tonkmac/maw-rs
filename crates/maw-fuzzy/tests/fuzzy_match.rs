use maw_fuzzy::{distance, fuzzy_match};

#[test]
fn distance_oracle_to_oracl_is_one() {
    assert_eq!(distance("oracle", "oracl"), 1);
}

#[test]
fn distance_hey_to_hek_is_one() {
    assert_eq!(distance("hey", "hek"), 1);
}

#[test]
fn distance_handles_empty_inputs() {
    assert_eq!(distance("", ""), 0);
    assert_eq!(distance("abc", ""), 3);
    assert_eq!(distance("", "abc"), 3);
}

#[test]
fn distance_exact_match_is_zero() {
    assert_eq!(distance("plugin", "plugin"), 0);
}

#[test]
fn distance_substitution_plus_insertion() {
    assert_eq!(distance("kitten", "sitting"), 3);
}

#[test]
fn fuzzy_match_returns_top_three_closest_matches_sorted_by_distance() {
    let pool = ["oracle", "plugin", "peek", "hey", "ping", "find", "fleet"];
    let result = fuzzy_match("oracl", &pool, 3, 3);
    assert_eq!(result.first().map(String::as_str), Some("oracle"));
    assert!(result.len() <= 3);
}

#[test]
fn fuzzy_match_suggests_hey_for_hek() {
    let pool = ["oracle", "plugin", "peek", "hey", "ping", "find", "fleet"];
    assert!(fuzzy_match("hek", &pool, 3, 3).contains(&"hey".to_owned()));
}

#[test]
fn fuzzy_match_empty_input_returns_empty() {
    let pool = ["oracle", "plugin", "peek", "hey", "ping", "find", "fleet"];
    assert_eq!(fuzzy_match("", &pool, 3, 3), Vec::<String>::new());
}

#[test]
fn fuzzy_match_returns_empty_when_nothing_within_max_distance() {
    let pool = ["oracle", "plugin", "peek", "hey", "ping", "find", "fleet"];
    assert_eq!(
        fuzzy_match("xyz-not-a-command", &pool, 3, 3),
        Vec::<String>::new()
    );
}

#[test]
fn fuzzy_match_is_case_insensitive() {
    let pool = ["oracle", "plugin", "peek", "hey", "ping", "find", "fleet"];
    assert!(fuzzy_match("ORACL", &pool, 3, 3).contains(&"oracle".to_owned()));
}

#[test]
fn fuzzy_match_deduplicates_candidates() {
    let result = fuzzy_match("oracle", &["oracle", "oracle", "oracl"], 3, 3);
    assert_eq!(result.iter().filter(|name| *name == "oracle").count(), 1);
}

#[test]
fn fuzzy_match_sorts_ties_alphabetically() {
    let result = fuzzy_match("cat", &["bat", "dat", "aat"], 3, 3);
    assert_eq!(result, vec!["aat", "bat", "dat"]);
}

#[test]
fn fuzzy_match_keeps_case_distinct_candidates_like_maw_js() {
    let result = fuzzy_match("oracle", &["Oracle", "oracle"], 3, 3);
    assert_eq!(result.len(), 2);
    assert!(result.contains(&"Oracle".to_owned()));
    assert!(result.contains(&"oracle".to_owned()));
}

#[test]
fn fuzzy_match_respects_zero_max_results() {
    assert_eq!(
        fuzzy_match("oracle", &["oracle"], 0, 3),
        Vec::<String>::new()
    );
}
