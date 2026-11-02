pub use crate::citation_fields::{CiteFields, format_cite, parse_cite_body};
pub use crate::line_range::{LineNumber, LineRange, parse_range};
use crate::{CodecError, citation_fields::format_field, unicode};
use sha2::{Digest, Sha256};
use std::ops::Range;

const OPENER: &str = "⟦cite";
const CLOSER: char = '⟧';

/// A borrowed occurrence; offsets are UTF-8 bytes, line numbers count only LF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiteToken<'a> {
    body: &'a str,
    raw: &'a str,
    line_no: usize,
    complete: bool,
    range: Range<usize>,
}

impl<'a> CiteToken<'a> {
    pub fn body(&self) -> &'a str {
        self.body
    }
    pub fn raw(&self) -> &'a str {
        self.raw
    }
    pub fn line_no(&self) -> usize {
        self.line_no
    }
    pub fn complete(&self) -> bool {
        self.complete
    }
    pub fn span(&self) -> Range<usize> {
        self.range.clone()
    }
    pub fn fields(&self) -> Result<CiteFields<'a>, CodecError> {
        parse_cite_body(self.body)
    }
    pub fn is_illustrative(&self) -> bool {
        is_illustrative(self.body)
    }
}

pub struct CiteTokens<'a> {
    text: &'a str,
    offset: usize,
    line: usize,
    split_openers: bool,
}

pub fn iter_cite_tokens(text: &str) -> CiteTokens<'_> {
    CiteTokens {
        text,
        offset: 0,
        line: 1,
        split_openers: true,
    }
}

impl<'a> Iterator for CiteTokens<'a> {
    type Item = CiteToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.offset + self.text[self.offset..].find(OPENER)?;
        let body_start = start + OPENER.len();
        let rest = &self.text[body_start..];
        let closing = rest.find(CLOSER);
        let next = if self.split_openers {
            rest.find(OPENER)
        } else {
            None
        };
        let (body_end, end, complete) = match (closing, next) {
            (Some(close), Some(open)) if open < close => (open, open, false),
            (Some(close), _) => (close, close + CLOSER.len_utf8(), true),
            (None, Some(open)) => (open, open, false),
            (None, None) if self.split_openers => (rest.len(), rest.len(), false),
            _ => {
                self.offset = self.text.len();
                return None;
            }
        };
        self.line += self.text[self.offset..start]
            .bytes()
            .filter(|&byte| byte == b'\n')
            .count();
        let line_no = self.line;
        let end = body_start + end;
        let raw = &self.text[start..end];
        self.line += raw.bytes().filter(|&byte| byte == b'\n').count();
        self.offset = end;
        Some(CiteToken {
            body: &self.text[body_start..body_start + body_end],
            raw,
            line_no,
            complete,
            range: start..end,
        })
    }
}

/// Historical complete-token regex behavior: unlike iter_cite_tokens, a later
/// opener is part of the current body until the first closing delimiter.
pub fn find_cite_lines(
    text: &str,
) -> impl Iterator<Item = Result<(CiteToken<'_>, CiteFields<'_>), CodecError>> {
    CiteTokens {
        text,
        offset: 0,
        line: 1,
        split_openers: false,
    }
    .map(|token| token.fields().map(|fields| (token, fields)))
}

pub fn is_illustrative(body: &str) -> bool {
    unicode::trim_start(body)
        .strip_prefix("illus")
        .is_some_and(|rest| {
            rest.chars()
                .next()
                .is_none_or(|character| !unicode::word(character))
        })
}

fn complete_body(raw: &str) -> Result<&str, CodecError> {
    let body = raw
        .strip_prefix(OPENER)
        .and_then(|rest| rest.strip_suffix(CLOSER));
    match body {
        Some(body) => Ok(body),
        _ => Err(CodecError::malformed(
            "expected one complete ⟦cite …⟧ token",
        )),
    }
}

pub fn replace_field(raw: &str, key: &str, new_value: &str) -> Result<String, CodecError> {
    let replacement = format_field(key, new_value)?;
    let body = complete_body(raw)?;
    let fields = parse_cite_body(body)?;
    if let Some(range) = fields.span(key) {
        let start = OPENER.len() + range.start;
        let end = OPENER.len() + range.end;
        let mut output = String::with_capacity(raw.len() - (end - start) + replacement.len());
        output.push_str(&raw[..start]);
        output.push_str(&replacement);
        output.push_str(&raw[end..]);
        Ok(output)
    } else {
        Ok(format!(
            "{} {replacement}⟧",
            unicode::trim_end(&raw[..raw.len() - CLOSER.len_utf8()])
        ))
    }
}

pub fn remove_field(raw: &str, key: &str) -> Result<String, CodecError> {
    let body = complete_body(raw)?;
    let fields = parse_cite_body(body)?;
    let Some(range) = fields.span(key) else {
        return Ok(raw.into());
    };
    let start = OPENER.len() + unicode::trim_end(&body[..range.start]).len();
    let end = OPENER.len() + range.end;
    let mut output = String::with_capacity(raw.len() - (end - start));
    output.push_str(&raw[..start]);
    output.push_str(&raw[end..]);
    Ok(output)
}

/// First sixteen lower-case hex SHA-256 digits of nonempty stripped lines,
/// joined with LF, without allocating a second normalized document.
pub fn normalize_span(lines: &[&str]) -> String {
    let mut hash = Sha256::new();
    let mut first = true;
    for line in lines {
        let line = unicode::trim(line);
        if line.is_empty() {
            continue;
        }
        if !first {
            hash.update(b"\n");
        }
        hash.update(line.as_bytes());
        first = false;
    }
    let digest = hash.finalize();
    let mut output = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        for nibble in [byte >> 4, byte & 15] {
            output.push(char::from(if nibble < 10 {
                b'0' + nibble
            } else {
                b'a' + nibble - 10
            }));
        }
    }
    output
}

/// Matches the historical unpinned locator warning pattern, without resolution.
pub fn find_unpinned(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.match_indices('@').filter_map(|(start, _)| {
        ["@main:", "@HEAD:", "@master:"]
            .into_iter()
            .find(|prefix| text[start..].starts_with(*prefix))
            .map(|prefix| (start, &text[start..start + prefix.len()]))
    })
}
