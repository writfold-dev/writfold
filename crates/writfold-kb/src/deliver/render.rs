//! A mode's `rendering`: how the payload (the bytes posted) is made from
//! the candidate. `verbatim` (the default) posts the candidate as it is;
//! `strip-citations` removes every complete, one-line citation token with
//! one space or tab directly before it, and nothing else: no line is added
//! or removed, so line numbers hold. No redaction, no templates, no
//! rewriting.
use super::candidate::Candidate;
use writfold_sources::citations::iter_cite_tokens;

pub const VERBATIM: &str = "verbatim";
pub const STRIP: &str = "strip-citations";
/// Every rendering a mode may name.
pub const RENDERINGS: [&str; 2] = [VERBATIM, STRIP];
/// Where a non-verbatim payload is written beside the candidate.
pub const FILE: &str = "payload.md";

/// The candidate's text with its citation tokens stripped.
pub fn strip(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for token in iter_cite_tokens(text) {
        if !token.complete() || token.raw().contains('\n') {
            continue;
        }
        let span = token.span();
        let mut start = span.start;
        if start > at && matches!(text.as_bytes()[start - 1], b' ' | b'\t') {
            start -= 1;
        }
        out.push_str(&text[at..start]);
        at = span.end;
    }
    out.push_str(&text[at..]);
    out
}

/// The payload of `bytes` under `rendering`.
pub fn render(bytes: &[u8], rendering: &str) -> Vec<u8> {
    match (rendering, std::str::from_utf8(bytes)) {
        (STRIP, Ok(text)) => strip(text).into_bytes(),
        _ => bytes.to_vec(),
    }
}

/// The present payload of `c` under its mode's rendering; `None` when the
/// candidate has no readable bytes or no valid manifest.
pub fn payload(root: &std::path::Path, c: &Candidate) -> Option<Vec<u8>> {
    let m = c.manifest.as_ref().ok()?;
    let rendering =
        super::mode::mode(root, &m.mode).map_or_else(|_| VERBATIM.into(), |m| m.rendering);
    Some(render(c.bytes.as_deref()?, &rendering))
}

#[cfg(test)]
mod tests {
    use super::strip;

    #[test]
    fn stripping_keeps_every_line_and_every_other_byte_in_order() {
        let text = "One ⟦cite kind=code repo=kb path=a L=1-1⟧.\nTwo\t⟦cite kind=web url=x⟧ and ⟦cite illus kind=web⟧\n⟦cite kind=x⟧ starts\nopen ⟦cite kind=y\n";
        let out = strip(text);
        assert_eq!(out, "One.\nTwo and\n starts\nopen ⟦cite kind=y\n");
        assert_eq!(out.lines().count(), text.lines().count());
        // every kept byte appears in the original, in order
        let mut rest = text;
        for ch in out.chars() {
            let at = rest.find(ch).unwrap_or(usize::MAX);
            assert!(at != usize::MAX, "{ch:?}");
            rest = &rest[at + ch.len_utf8()..];
        }
        assert_eq!(strip("no tokens\n"), "no tokens\n");
    }
}
