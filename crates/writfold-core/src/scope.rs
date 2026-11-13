//! Which paths a gate's scope covers: anchored, segment-wise glob patterns
//! with `!` exclusions. Split out of `gate` so the gate's decisions can be
//! translated by Charon/Aeneas: this module uses iterators, closures and
//! recursion over string slices, which the translation does not take.
//!
//! Not translated and not proved. The tests below, and the scope tests of
//! `writfold-kb`, check it.

/// Whether `path` (repository-relative, `/`-separated) is in a scope: some
/// pattern matches it and no `!`-prefixed pattern does. Patterns are
/// anchored at the top of the repository: `*` and `?` match within one
/// segment, and a `**` segment matches any number of segments, none
/// included.
pub fn in_scope(patterns: &[&str], path: &str) -> bool {
    let mut included = false;
    for pattern in patterns {
        match pattern.strip_prefix('!') {
            Some(excluded) if glob(excluded, path) => return false,
            Some(_) => {}
            None => included = included || glob(pattern, path),
        }
    }
    included
}

/// One anchored pattern against one path.
pub fn glob(pattern: &str, path: &str) -> bool {
    let wanted: alloc::vec::Vec<&str> = pattern.split('/').collect();
    let parts: alloc::vec::Vec<&str> = path.split('/').collect();
    segments(&wanted, &parts)
}

fn segments(wanted: &[&str], parts: &[&str]) -> bool {
    match wanted.split_first() {
        None => parts.is_empty(),
        Some((&"**", rest)) => (0..=parts.len()).any(|skip| segments(rest, &parts[skip..])),
        Some((first, rest)) => match parts.split_first() {
            Some((part, others)) => segment(first, part) && segments(rest, others),
            None => false,
        },
    }
}

/// `*` and `?` within one segment, by characters, with one backtrack point.
fn segment(pattern: &str, text: &str) -> bool {
    let p: alloc::vec::Vec<char> = pattern.chars().collect();
    let t: alloc::vec::Vec<char> = text.chars().collect();
    let (mut i, mut j) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while j < t.len() {
        if i < p.len() && (p[i] == '?' || p[i] == t[j]) {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == '*' {
            star = Some((i, j));
            i += 1;
        } else if let Some((at, from)) = star {
            i = at + 1;
            j = from + 1;
            star = Some((at, from + 1));
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == '*' {
        i += 1;
    }
    i == p.len()
}

#[cfg(test)]
mod tests {
    use super::{glob, in_scope};

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

    #[test]
    fn exclusions_win_and_nothing_is_in_an_empty_scope() {
        let scope = ["**", "!docs/**", "!README.md"];
        assert!(in_scope(&scope, "crates/a.rs") && in_scope(&scope, "x/README.md"));
        assert!(!in_scope(&scope, "docs/x.md") && !in_scope(&scope, "README.md"));
        assert!(!in_scope(&[], "a") && !in_scope(&["!a"], "b"));
    }
}
