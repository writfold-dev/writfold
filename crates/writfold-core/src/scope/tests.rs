use super::{glob, in_scope};
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

#[test]
fn globs_are_anchored_segment_patterns() {
    assert!(glob("**", "a/b/c.rs") && glob("**", "x"));
    assert!(glob("crates/**", "crates/a/src/lib.rs") && glob("crates/**", "crates"));
    assert!(!glob("crates/**", "xcrates/a"));
    assert!(glob("**/*.md", "README.md") && glob("**/*.md", "docs/a/b.md"));
    assert!(!glob("*.md", "docs/a.md") && glob("*.md", "a.md"));
    assert!(glob("src/*.rs", "src/lib.rs") && !glob("src/*.rs", "src/a/lib.rs"));
    assert!(glob("a?c", "abc") && !glob("a?c", "ac"));
    assert!(glob("*a*b", "xxaxxb") && !glob("*a*b", "xxaxxbx"));
    assert!(glob("Cargo.toml", "Cargo.toml") && !glob("Cargo.toml", "x/Cargo.toml"));
    assert!(glob("a/**/b", "a/b") && glob("a/**/b", "a/x/y/b") && !glob("a/**/b", "a/x/c"));
}

/// A literal `*` in a path is still matched by a `*` in the pattern
/// that may stretch over it: the star is tried before the literal.
#[test]
fn a_star_in_the_path_does_not_stop_a_star_in_the_pattern() {
    assert!(glob("*a", "*ba") && glob("*", "*") && glob("x*", "x*y"));
    assert!(glob("*a*", "**b*a") && !glob("*a", "*b"));
}

#[test]
fn exclusions_win_and_nothing_is_in_an_empty_scope() {
    let scope = ["**", "!docs/**", "!README.md"];
    assert!(in_scope(&scope, "crates/a.rs") && in_scope(&scope, "x/README.md"));
    assert!(!in_scope(&scope, "docs/x.md") && !in_scope(&scope, "README.md"));
    assert!(!in_scope(&[], "a") && !in_scope(&["!a"], "b"));
}

/// The matcher's definition, written as recursion: a `*` matches nothing
/// or stretches by one; `?` or the same character matches one.
fn reference(p: &[char], t: &[char]) -> bool {
    match (p.split_first(), t.split_first()) {
        (None, _) => t.is_empty(),
        (Some(('*', rest)), _) => {
            reference(rest, t) || t.split_first().is_some_and(|(_, more)| reference(p, more))
        }
        (Some(_), None) => false,
        (Some((&c, rest)), Some((&d, more))) => (c == '?' || c == d) && reference(rest, more),
    }
}

/// Every word of up to `n` characters over `alphabet`.
fn words(alphabet: &[char], n: usize) -> Vec<String> {
    let mut all = vec![String::new()];
    let mut last = vec![String::new()];
    for _ in 0..n {
        let next: Vec<String> = (last.iter())
            .flat_map(|w| alphabet.iter().map(move |c| format!("{w}{c}")))
            .collect();
        all.extend(next.iter().cloned());
        last = next;
    }
    all
}

/// Every pattern and segment of up to four characters over `a`, `b`, `*`
/// and `?`: the loop agrees with the recursive definition.
#[test]
fn segment_agrees_with_its_definition() {
    let all = words(&['a', 'b', '*', '?'], 4);
    for p in &all {
        for t in &all {
            let (pc, tc): (Vec<char>, Vec<char>) = (p.chars().collect(), t.chars().collect());
            assert_eq!(glob(p, t), reference(&pc, &tc), "{p:?} against {t:?}");
        }
    }
}

/// Paths of up to three segments over `a`, `b` and `**`: a `**` segment
/// matches any number of segments, none included.
#[test]
fn segments_agree_with_their_definition() {
    fn path_ref(w: &[&str], p: &[&str]) -> bool {
        match (w.split_first(), p.split_first()) {
            (None, _) => p.is_empty(),
            (Some((&"**", rest)), _) => {
                path_ref(rest, p) || p.split_first().is_some_and(|(_, more)| path_ref(w, more))
            }
            (Some(_), None) => false,
            (Some((s, rest)), Some((q, more))) => glob(s, q) && path_ref(rest, more),
        }
    }
    let segs = ["a", "b", "**", "*", "a*"];
    let mut paths: Vec<Vec<&str>> = vec![vec![]];
    for _ in 0..3 {
        let longer: Vec<Vec<&str>> = (paths.iter())
            .flat_map(|p| segs.iter().map(move |s| [p.clone(), vec![*s]].concat()))
            .collect();
        paths.extend(longer);
    }
    paths.retain(|p| !p.is_empty());
    for w in &paths {
        for p in &paths {
            let (wj, pj) = (w.join("/"), p.join("/"));
            assert_eq!(glob(&wj, &pj), path_ref(w, p), "{wj:?} against {pj:?}");
        }
    }
}
