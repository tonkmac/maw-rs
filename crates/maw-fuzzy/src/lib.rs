//! Fuzzy command matching ported from maw-js `src/core/util/fuzzy.ts`.
//!
//! The implementation mirrors maw-js' dependency-free two-row Levenshtein
//! helper used by CLI typo suggestions, including JavaScript-style UTF-16
//! code-unit distance.

/// Levenshtein edit distance between two strings.
#[must_use]
pub fn distance(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }

    let a_units: Vec<u16> = a.encode_utf16().collect();
    let b_units: Vec<u16> = b.encode_utf16().collect();

    if a_units.is_empty() {
        return b_units.len();
    }
    if b_units.is_empty() {
        return a_units.len();
    }

    let n = b_units.len();
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];

    for (i, a_unit) in a_units.iter().enumerate() {
        curr[0] = i + 1;
        for (j, b_unit) in b_units.iter().enumerate() {
            let cost = usize::from(a_unit != b_unit);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Return up to `max_results` candidates within `max_distance` of `input`.
///
/// Matching is case-insensitive. Duplicate candidates are suppressed before
/// scoring, preserving maw-js' exact-string duplicate semantics.
#[must_use]
pub fn fuzzy_match(
    input: &str,
    candidates: &[&str],
    max_results: usize,
    max_distance: usize,
) -> Vec<String> {
    if input.is_empty() || max_results == 0 {
        return Vec::new();
    }

    let lower_input = input.to_lowercase();
    let mut seen = Vec::<&str>::new();
    let mut scored = Vec::<ScoredCandidate>::new();

    for candidate in candidates {
        if candidate.is_empty() || seen.contains(candidate) {
            continue;
        }
        seen.push(candidate);
        let candidate_lower = candidate.to_lowercase();
        let d = distance(&lower_input, &candidate_lower);
        if d <= max_distance {
            scored.push(ScoredCandidate {
                name: (*candidate).to_owned(),
                distance: d,
            });
        }
    }

    scored.sort_by(|a, b| {
        a.distance
            .cmp(&b.distance)
            .then_with(|| a.name.cmp(&b.name))
    });
    scored
        .into_iter()
        .take(max_results)
        .map(|candidate| candidate.name)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScoredCandidate {
    name: String,
    distance: usize,
}
