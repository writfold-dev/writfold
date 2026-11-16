//! Name forms for `find`'s `normalized`, `prefix` and `heading` tiers:
//! what a phrase and a record's name have in common once case and
//! punctuation are set aside. Pure; the caller reads the names.

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

/// Whether the query's tokens may match by prefix at all: two or more,
/// or one of four characters or more.
pub fn prefix_query(query: &[&str]) -> bool {
    match query {
        [] => false,
        [one] => one.chars().count() >= 4,
        _ => true,
    }
}

/// Whether each query token is a prefix of a distinct name token, in the
/// same order, gaps allowed; `Some` with the count of name tokens left
/// unmatched. `quay work` matches `quay-worker`; `work quay` does not.
pub fn prefix_match(query: &[&str], name: &[&str]) -> Option<usize> {
    if !prefix_query(query) {
        return None;
    }
    let mut at = 0;
    for q in query {
        let skip = name.get(at..)?.iter().position(|n| n.starts_with(q))?;
        at += skip + 1;
    }
    Some(name.len() - query.len())
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
