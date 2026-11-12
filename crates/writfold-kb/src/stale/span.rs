//! Where a cited span now sits in a file: a pure decision over its lines.
use writfold_sources::citations::normalize_span;

/// Files longer than this are not searched; the citation is `changed`.
pub const SEARCH_LIMIT: usize = 50_000;

pub use writfold_core::verdict::Located;

/// Slide a window of the cited height over `text` (the kernel in
/// `writfold-core` decides; this supplies the span digest of a window).
pub fn locate(text: &str, first: u32, last: u32, span: &str) -> Located {
    let lines: Vec<&str> = text.lines().collect();
    writfold_core::verdict::locate(lines.len(), first, last, SEARCH_LIMIT, |start, height| {
        lines
            .get(start..start + height)
            .is_some_and(|window| normalize_span(window) == span)
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
