//! Which paths a gate's scope covers: anchored, segment-wise glob patterns
//! with `!` exclusions.
//!
//! The decision is translated by Charon/Aeneas and proved
//! (`proofs/Writfold/Scope.lean`, docs/writfold/aeneas.md): [`covers`],
//! [`segments`] and [`segment`] work on code points split on `/`, and are
//! proved equal to an inductive matcher for every input. [`in_scope`] and
//! [`glob`] only decode text into those inputs; they are tested, not
//! translated.
//!
//! Both matchers are the same two-index loop with one backtrack point: the
//! last wildcard seen (`*` within a segment, a `**` segment within a path)
//! and where its stretch ends. A wildcard is tried before a literal, so a
//! `*` in a path is still covered by a `*` in the pattern.

use alloc::vec::Vec;

/// `*`: any run of characters within one segment, none included.
pub const STAR: u32 = 42;
/// `?`: exactly one character.
pub const ANY: u32 = 63;

/// One pattern: `exclude` for a `!` prefix, then its segments as code points.
pub struct Pattern {
    pub exclude: bool,
    pub segments: Vec<Vec<u32>>,
}

/// Where a matcher stands: `i` in the pattern, `j` in the text; when
/// `starred`, the last wildcard is at `at` and its stretch ends at `from`.
pub struct Cursor {
    pub i: usize,
    pub j: usize,
    pub starred: bool,
    pub at: usize,
    pub from: usize,
}

/// One step of a matcher: go on from a cursor, or stop with the answer.
pub enum Next {
    Go(Cursor),
    Done(bool),
}

/// The cursor every match starts from.
pub fn start() -> Cursor {
    Cursor {
        i: 0,
        j: 0,
        starred: false,
        at: 0,
        from: 0,
    }
}

/// Whether one pattern character matches one text character (not `*`).
pub fn same_char(pattern: u32, text: u32) -> bool {
    pattern == ANY || pattern == text
}

/// One step of [`segment`]: a `*` is recorded first; at the end of the
/// text only trailing `*`s remain to match; then `?` or the same
/// character; then the last `*` stretches by one; else no match.
pub fn char_step(pattern: &[u32], text: &[u32], c: Cursor) -> Next {
    if c.i < pattern.len() && pattern[c.i] == STAR {
        return Next::Go(Cursor {
            i: c.i + 1,
            j: c.j,
            starred: true,
            at: c.i,
            from: c.j,
        });
    }
    if c.j >= text.len() {
        return Next::Done(c.i == pattern.len());
    }
    if c.i < pattern.len() && same_char(pattern[c.i], text[c.j]) {
        return Next::Go(Cursor {
            i: c.i + 1,
            j: c.j + 1,
            starred: c.starred,
            at: c.at,
            from: c.from,
        });
    }
    if c.starred {
        return Next::Go(Cursor {
            i: c.at + 1,
            j: c.from + 1,
            starred: true,
            at: c.at,
            from: c.from + 1,
        });
    }
    Next::Done(false)
}

/// One segment pattern against one path segment, by code points.
pub fn segment(pattern: &[u32], text: &[u32]) -> bool {
    let mut c = start();
    loop {
        match char_step(pattern, text, c) {
            Next::Go(n) => c = n,
            Next::Done(b) => return b,
        }
    }
}

/// Whether a pattern segment is `**`.
pub fn globstar(segment: &[u32]) -> bool {
    segment.len() == 2 && segment[0] == STAR && segment[1] == STAR
}

/// One step of [`segments`]: [`char_step`] one level up, with a `**`
/// segment for `*` and [`segment`] for the same character.
pub fn path_step(wanted: &[Vec<u32>], parts: &[Vec<u32>], c: Cursor) -> Next {
    if c.i < wanted.len() && globstar(&wanted[c.i]) {
        return Next::Go(Cursor {
            i: c.i + 1,
            j: c.j,
            starred: true,
            at: c.i,
            from: c.j,
        });
    }
    if c.j >= parts.len() {
        return Next::Done(c.i == wanted.len());
    }
    if c.i < wanted.len() && segment(&wanted[c.i], &parts[c.j]) {
        return Next::Go(Cursor {
            i: c.i + 1,
            j: c.j + 1,
            starred: c.starred,
            at: c.at,
            from: c.from,
        });
    }
    if c.starred {
        return Next::Go(Cursor {
            i: c.at + 1,
            j: c.from + 1,
            starred: true,
            at: c.at,
            from: c.from + 1,
        });
    }
    Next::Done(false)
}

/// A pattern's segments against a path's: a `**` segment matches any
/// number of segments, none included; every other segment matches one.
pub fn segments(wanted: &[Vec<u32>], parts: &[Vec<u32>]) -> bool {
    let mut c = start();
    loop {
        match path_step(wanted, parts, c) {
            Next::Go(n) => c = n,
            Next::Done(b) => return b,
        }
    }
}

/// Whether a path is included so far: it was, or a pattern without `!`
/// matches it.
pub fn include(included: bool, exclude: bool, hit: bool) -> bool {
    included || (!exclude && hit)
}

/// Whether a path is covered: some pattern without `!` matches it and no
/// `!` pattern does.
pub fn covers(patterns: &[Pattern], path: &[Vec<u32>]) -> bool {
    let mut included = false;
    let mut index = 0;
    while index < patterns.len() {
        let hit = segments(&patterns[index].segments, path);
        if patterns[index].exclude && hit {
            return false;
        }
        included = include(included, patterns[index].exclude, hit);
        index += 1;
    }
    included
}

/// Text split on `/`, each segment as code points.
fn split(text: &str) -> Vec<Vec<u32>> {
    text.split('/')
        .map(|s| s.chars().map(u32::from).collect())
        .collect()
}

/// Whether `path` (repository-relative, `/`-separated) is in a scope
/// ([`covers`]). Patterns are anchored at the top of the repository: `*`
/// and `?` match within one segment, and a `**` segment matches any number
/// of segments, none included.
pub fn in_scope(patterns: &[&str], path: &str) -> bool {
    let patterns: Vec<Pattern> = patterns
        .iter()
        .map(|p| match p.strip_prefix('!') {
            Some(excluded) => Pattern {
                exclude: true,
                segments: split(excluded),
            },
            None => Pattern {
                exclude: false,
                segments: split(p),
            },
        })
        .collect();
    covers(&patterns, &split(path))
}

/// One anchored pattern against one path ([`segments`]).
pub fn glob(pattern: &str, path: &str) -> bool {
    segments(&split(pattern), &split(path))
}

#[cfg(test)]
mod tests;
