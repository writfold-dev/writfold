//! Pure match location over exact bytes, line numbers and whole-line ranges.
use memchr::{memchr, memmem, memrchr};
use regex::bytes::{Regex, RegexBuilder};

/// The admitted regex dialect: Rust `regex` 1.11.1 over bytes, Unicode mode
/// off (ASCII `\w \d \s \b`, `.` is any byte except LF), multi-line `^`/`$`,
/// leftmost-first non-overlapping matches. Inline flags, Unicode classes and
/// backreferences refuse; the automaton is size-bounded, so matching is linear.
pub const REGEX_PROFILE: &str = "rust-regex-1.11.1/bytes-ascii-multiline-v0";
const MAX_REGEX_BYTES: usize = 1_024;
const AUTOMATON_BYTES: usize = 1 << 20;

#[derive(Clone, Debug)]
pub struct Pattern {
    source: String,
    regex: Regex,
}

/// A regex produced an empty match; Tuft refuses rather than inventing a range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZeroWidth {
    pub offset: usize,
}

impl Pattern {
    pub fn compile(source: &str) -> Result<Self, String> {
        if source.is_empty() || source.len() > MAX_REGEX_BYTES {
            return Err(format!("regex must be 1..={MAX_REGEX_BYTES} bytes"));
        }
        screen(source)?;
        let regex = RegexBuilder::new(source)
            .unicode(false)
            .multi_line(true)
            .nest_limit(32)
            .size_limit(AUTOMATON_BYTES)
            .dfa_size_limit(AUTOMATON_BYTES)
            .build()
            .map_err(|error| match error {
                regex::Error::CompiledTooBig(_) => "regex automaton exceeds its size bound".into(),
                _ => "regex outside the admitted dialect".to_owned(),
            })?;
        Ok(Self {
            source: source.to_owned(),
            regex,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Every leftmost-first, non-overlapping match as `[lo, hi)`.
    pub fn matches(&self, content: &[u8]) -> Result<Vec<(usize, usize)>, ZeroWidth> {
        let mut out = Vec::new();
        for found in self.regex.find_iter(content) {
            if found.start() == found.end() {
                return Err(ZeroWidth {
                    offset: found.start(),
                });
            }
            out.push((found.start(), found.end()));
        }
        Ok(out)
    }
}

/// Refuse constructs whose meaning the dialect does not admit, instead of
/// letting the engine reinterpret them.
fn screen(source: &str) -> Result<(), String> {
    let bytes = source.as_bytes();
    let mut at = 0;
    while let Some(&b) = bytes.get(at) {
        match b {
            b'\\' => {
                if matches!(bytes.get(at + 1), Some(b'p' | b'P')) {
                    return Err("Unicode classes (\\p, \\P) are refused".into());
                }
                at += 2;
                continue;
            }
            b'(' if bytes.get(at + 1) == Some(&b'?') => {
                let rest = bytes.get(at + 2..).unwrap_or_default();
                if !(rest.starts_with(b":") || rest.starts_with(b"<") || rest.starts_with(b"P<")) {
                    return Err("inline flags are refused".into());
                }
            }
            _ => {}
        }
        at += 1;
    }
    Ok(())
}

/// Every occurrence of `needle`, including overlapping starts, as `[lo, hi)`.
pub fn text_matches(content: &[u8], needle: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }
    let finder = memmem::Finder::new(needle);
    let mut from = 0;
    while let Some(found) = content.get(from..).and_then(|rest| finder.find(rest)) {
        let lo = from + found;
        out.push((lo, lo + needle.len()));
        from = lo + 1;
    }
    out
}

/// 1-based line number of `offset`: one plus the LFs strictly before it.
pub fn line_at(content: &[u8], offset: usize) -> u64 {
    let before = content.get(..offset).unwrap_or(content);
    1 + memchr::memchr_iter(b'\n', before).count() as u64
}

/// The whole lines holding `[lo, hi)`, widened by `n` whole lines on each
/// side, including the last line's LF (or ending at end of file). CR is
/// ordinary content.
pub fn around(content: &[u8], lo: usize, hi: usize, n: usize) -> (usize, usize) {
    let len = content.len();
    let lo = lo.min(len);
    let mut start = line_start(content, lo);
    for _ in 0..n {
        if start == 0 {
            break;
        }
        start = line_start(content, start - 1);
    }
    let last = if hi > lo { hi - 1 } else { lo };
    let mut end = line_end(content, last.min(len));
    for _ in 0..n {
        if end >= len {
            break;
        }
        end = line_end(content, end);
    }
    (start, end)
}

fn line_start(content: &[u8], offset: usize) -> usize {
    content
        .get(..offset)
        .and_then(|before| memrchr(b'\n', before))
        .map_or(0, |at| at + 1)
}

fn line_end(content: &[u8], offset: usize) -> usize {
    content
        .get(offset..)
        .and_then(|rest| memchr(b'\n', rest))
        .map_or(content.len(), |at| offset + at + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_text_matches() {
        assert_eq!(text_matches(b"aaaa", b"aa"), [(0, 2), (1, 3), (2, 4)]);
        assert!(text_matches(b"abc", b"z").is_empty());
    }

    #[test]
    fn lines_and_around() {
        let text = b"one\ntwo\nthree\nfour";
        assert_eq!(line_at(text, 0), 1);
        assert_eq!(line_at(text, 8), 3);
        assert_eq!(around(text, 9, 11, 0), (8, 14));
        assert_eq!(around(text, 9, 11, 1), (4, 18));
        assert_eq!(around(text, 0, 1, 5), (0, 18));
        assert_eq!(around(text, 4, 8, 0), (4, 8));
    }

    #[test]
    fn regex_dialect() {
        assert!(Pattern::compile("(?i)x").is_err());
        assert!(Pattern::compile(r"\pL").is_err());
        assert!(Pattern::compile(r"(\w)\1").is_err());
        let p = Pattern::compile(r"(?:open)\(\w+\)").map_err(|e| e.to_string());
        assert_eq!(
            p.map(|p| p.matches(b"x open(url) open(u)")),
            Ok(Ok(vec![(2, 11), (12, 19)]))
        );
        let empty = Pattern::compile("x*").map(|p| p.matches(b"ab").is_err());
        assert_eq!(empty, Ok(true));
    }
}
