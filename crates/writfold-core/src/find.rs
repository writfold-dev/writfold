//! Name forms for `find`'s `normalized`, `prefix` and `heading` tiers:
//! what a phrase and a record's name have in common once case and
//! punctuation are set aside. Pure; the caller reads the names.
//!
//! The prefix decision ([`query_ok`], [`starts`], [`next_from`],
//! [`prefixed`]), the row admission ([`admit`]) and when the rows are
//! [`full`] are translated and proved (`proofs/Writfold/Find.lean`,
//! `Walk.lean`); [`norm`] and [`tokens`] are text plumbing, tested.

use alloc::string::String;
use alloc::vec::Vec;

/// Lower-case; every run of characters that are not letters or digits
/// becomes one `-`; leading and trailing `-` removed. `Harbor_API` and
/// `harbor api` are both `harbor-api`.
pub fn norm(text: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in text.chars() {
        if c.is_alphanumeric() {
            if gap && !out.is_empty() {
                out.push('-');
            }
            gap = false;
            out.extend(c.to_lowercase());
        } else {
            gap = true;
        }
    }
    out
}

/// The parts of a normalized name between its `-`.
pub fn tokens(normed: &str) -> Vec<&str> {
    normed.split('-').filter(|t| !t.is_empty()).collect()
}

/// Code points of each token.
fn points(tokens: &[&str]) -> Vec<Vec<u32>> {
    tokens
        .iter()
        .map(|t| t.chars().map(u32::from).collect())
        .collect()
}

/// Whether the query's tokens may match by prefix at all: two or more,
/// or one of four characters or more ([`query_ok`]).
pub fn prefix_query(query: &[&str]) -> bool {
    query_ok(&points(query))
}

/// Whether each query token is a prefix of a distinct name token, in the
/// same order, gaps allowed; `Some` with the count of name tokens left
/// unmatched. `quay work` matches `quay-worker`; `work quay` does not
/// ([`prefixed`]).
pub fn prefix_match(query: &[&str], name: &[&str]) -> Option<usize> {
    prefixed(&points(query), &points(name))
}

/// [`prefix_query`] over code points.
pub fn query_ok(query: &[Vec<u32>]) -> bool {
    query.len() >= 2 || (query.len() == 1 && query[0].len() >= 4)
}

/// Whether `word` begins with `start`.
pub fn starts(word: &[u32], start: &[u32]) -> bool {
    if start.len() > word.len() {
        return false;
    }
    let mut i = 0;
    while i < start.len() {
        if word[i] != start[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The first name token at or after `at` that begins with `q`.
pub fn next_from(name: &[Vec<u32>], q: &[u32], at: usize) -> Option<usize> {
    let mut j = at;
    while j < name.len() {
        if starts(&name[j], q) {
            return Some(j);
        }
        j += 1;
    }
    None
}

/// [`prefix_match`] over code points: each query token, in order, takes
/// the first name token after the last one taken that it begins.
pub fn prefixed(query: &[Vec<u32>], name: &[Vec<u32>]) -> Option<usize> {
    if !query_ok(query) {
        return None;
    }
    let mut at = 0;
    let mut k = 0;
    while k < query.len() {
        match next_from(name, &query[k], at) {
            None => return None,
            Some(j) => at = j + 1,
        }
        k += 1;
    }
    Some(name.len() - query.len())
}

/// Whether `find` shows a hit: its record is not already shown (by an
/// earlier tier, or earlier in this one), it passes the filter, and fewer
/// than `limit` rows are shown. Every tier's hits go through it in tier
/// order, so a record appears once, under the first tier that found it.
pub fn admit(shown: usize, limit: usize, seen: bool, passes: bool) -> bool {
    shown < limit && !seen && passes
}

/// Whether `find` has shown as many rows as it may: then [`admit`] takes
/// no further hit, so a later tier need not run; before, it takes any hit
/// not yet shown that passes the filter.
pub fn full(shown: usize, limit: usize) -> bool {
    shown >= limit
}

#[cfg(test)]
mod tests {
    use super::{norm, prefix_match, tokens};

    #[test]
    fn norm_folds_case_and_punctuation() {
        let table = [
            ("harbor api", "harbor-api"),
            ("harbor_api", "harbor-api"),
            ("HAPI", "hapi"),
            ("  Harbor -- API!  ", "harbor-api"),
            ("runbook:ops/restart-harbor", "runbook-ops-restart-harbor"),
            ("Staging sweep", "staging-sweep"),
            ("Ünïcode Ä", "ünïcode-ä"),
            ("", ""),
            ("--", ""),
        ];
        for (text, want) in table {
            assert_eq!(norm(text), want, "{text}");
        }
        assert_eq!(tokens("a-b-c"), ["a", "b", "c"]);
        assert!(tokens("").is_empty());
    }

    #[test]
    fn prefix_match_keeps_order_and_needs_enough_query() {
        let name = ["repo", "jmga", "web", "app"];
        let table: [(&[&str], &[&str], Option<usize>); 9] = [
            (&["jmga", "web"], &name, Some(2)),
            (&["quay", "work"], &["quay", "worker"], Some(0)),
            (&["work", "quay"], &["quay", "worker"], None),
            (&["quay"], &["quay", "worker"], Some(1)),
            (&["qua"], &["quay", "worker"], None),
            (&["re", "a"], &name, Some(2)),
            (&["web", "web"], &name, None),
            (&["x", "y"], &name, None),
            (&[], &name, None),
        ];
        for (query, name, want) in table {
            assert_eq!(prefix_match(query, name), want, "{query:?} {name:?}");
        }
    }
}
