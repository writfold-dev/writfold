//! Where a cited span now sits in a file: a pure decision over its lines.
use writfold_sources::citations::normalize_span;

/// Files longer than this are not searched; the citation is `changed`.
pub const SEARCH_LIMIT: usize = 50_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Located {
    /// The span digest matches at the cited lines.
    Same,
    /// It matches first at these one-based lines.
    At(u32, u32),
    /// No window of the cited height matches.
    Absent,
    /// The file is too long to search.
    Unsearched,
}

/// Slide a window of the cited height over `text`; the cited position is
/// tried first, then every position from the top.
pub fn locate(text: &str, first: u32, last: u32, span: &str) -> Located {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() > SEARCH_LIMIT {
        return Located::Unsearched;
    }
    let Some(height) = last.checked_sub(first).map(|h| h as usize + 1) else {
        return Located::Absent;
    };
    if first == 0 || height > lines.len() {
        return Located::Absent;
    }
    let cited = first as usize - 1;
    let matches = |start: usize| {
        lines
            .get(start..start + height)
            .is_some_and(|window| normalize_span(window) == span)
    };
    if matches(cited) {
        return Located::Same;
    }
    (0..=lines.len() - height)
        .find(|&start| matches(start))
        .map_or(Located::Absent, |start| {
            let to = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
            Located::At(to(start + 1), to(start + height))
        })
}

#[cfg(test)]
mod tests {
    use super::{Located, locate};
    use writfold_sources::citations::normalize_span;

    #[test]
    fn same_moved_absent() {
        let span = normalize_span(&["fn a() {", "  1", "}"]);
        let original = "x\nfn a() {\n  1\n}\n";
        assert_eq!(locate(original, 2, 4, &span), Located::Same);
        let shifted = "x\ny\nz\nfn a() {\n    1\n}\n";
        assert_eq!(locate(shifted, 2, 4, &span), Located::At(4, 6));
        let edited = "x\nfn a() {\n  2\n}\n";
        assert_eq!(locate(edited, 2, 4, &span), Located::Absent);
        assert_eq!(locate(original, 4, 2, &span), Located::Absent);
    }
}
