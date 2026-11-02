pub use crate::document_lines::{UnfencedLines, unfenced_lines};
use crate::{CodecError, document_lines, unicode};
use std::collections::HashSet;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Span {
    pub(crate) name: String,
    pub(crate) range: Range<usize>,
}

/// Immutable UTF-8 document, retaining every original byte and ordered span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDocument {
    pub(crate) text: String,
    pub(crate) fields: Vec<Span>,
    pub(crate) sections: Vec<Span>,
    pub(crate) frontmatter_end: usize,
}

impl MarkdownDocument {
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }

    pub fn newline(&self) -> &str {
        newline_at(&self.text).map_or("\n", |range| &self.text[range])
    }

    pub fn body(&self) -> &str {
        let rest = &self.text[self.frontmatter_end..];
        newline_at(rest).map_or("", |range| &rest[range.end..])
    }

    pub fn frontmatter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.fields.iter().map(|span| {
            (
                span.name.as_str(),
                unicode::trim(&self.text[span.range.clone()]),
            )
        })
    }

    pub fn sections(&self) -> impl Iterator<Item = (&str, &str)> {
        self.sections.iter().map(|span| {
            (
                span.name.as_str(),
                unicode::trim(&self.text[span.range.clone()]),
            )
        })
    }

    pub fn field(&self, name: &str) -> Option<&str> {
        self.frontmatter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value)
    }

    pub fn section(&self, name: &str) -> Option<&str> {
        self.sections()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value)
    }

    pub fn field_span(&self, name: &str) -> Option<Range<usize>> {
        self.fields
            .iter()
            .find(|span| span.name == name)
            .map(|span| span.range.clone())
    }

    pub fn section_span(&self, name: &str) -> Option<Range<usize>> {
        self.sections
            .iter()
            .find(|span| span.name == name)
            .map(|span| span.range.clone())
    }

    /// Matches the historical LF-based body selection of this particular view.
    pub fn unfenced_lines(&self) -> impl Iterator<Item = &str> {
        let rest = &self.text[self.frontmatter_end..];
        let body = rest.find('\n').map_or("", |index| &rest[index + 1..]);
        unfenced_lines(body).map(|(_, line)| line.trim_end_matches(['\r', '\n']))
    }

    pub fn require(&self, keys: &[&str], sections: &[&str]) -> Result<(), CodecError> {
        for key in keys {
            if self.field(key).is_none_or(str::is_empty) {
                return Err(CodecError::malformed(format!(
                    "missing required frontmatter key {key:?}"
                )));
            }
        }
        for name in sections {
            if self.section(name).is_none() {
                return Err(CodecError::malformed(format!(
                    "missing required section {name:?}"
                )));
            }
        }
        Ok(())
    }
}

fn newline_at(text: &str) -> Option<Range<usize>> {
    let start = text.find(['\r', '\n'])?;
    let width = if text[start..].starts_with("\r\n") {
        2
    } else {
        1
    };
    Some(start..start + width)
}

pub fn parse_document(text: impl Into<String>) -> Result<MarkdownDocument, CodecError> {
    let text = text.into();
    let (fields, frontmatter_end, body_start) = parse_fields(&text)?;
    let sections = parse_sections(&text, body_start)?;
    Ok(MarkdownDocument {
        text,
        fields,
        sections,
        frontmatter_end,
    })
}

pub fn parse_document_required(
    text: impl Into<String>,
    keys: &[&str],
    sections: &[&str],
) -> Result<MarkdownDocument, CodecError> {
    let document = parse_document(text)?;
    document.require(keys, sections)?;
    Ok(document)
}

fn parse_fields(text: &str) -> Result<(Vec<Span>, usize, usize), CodecError> {
    let mut lines = unicode::lines(text);
    if lines
        .next()
        .is_none_or(|(_, line)| unicode::trim(line) != "---")
    {
        return Err(CodecError::malformed("document file missing frontmatter"));
    }
    let mut fields = Vec::new();
    let mut names = HashSet::new();
    for (offset, line) in lines {
        if unicode::trim(line) == "---" {
            return Ok((fields, offset, offset + line.len()));
        }
        if unicode::trim_start(line).starts_with('#') {
            continue;
        }
        if let Some(colon) = line.find(':') {
            let key = unicode::trim(&line[..colon]);
            if !names.insert(key) {
                return Err(CodecError::malformed(format!(
                    "duplicate frontmatter key {key:?}"
                )));
            }
            let value = line[colon + 1..].trim_end_matches(['\r', '\n']);
            let leading = value.len() - unicode::trim_start(value).len();
            let start = offset + colon + 1 + leading;
            fields.push(Span {
                name: key.into(),
                range: start..start + unicode::trim(value).len(),
            });
        }
    }
    Err(CodecError::malformed(
        "document file frontmatter never closes",
    ))
}

fn parse_sections(text: &str, body_start: usize) -> Result<Vec<Span>, CodecError> {
    let mut sections: Vec<Span> = Vec::new();
    let mut names = HashSet::new();
    for (relative, line) in unfenced_lines(&text[body_start..]) {
        if let Some(name) = document_lines::heading(line) {
            if !names.insert(name) {
                return Err(CodecError::malformed(format!(
                    "duplicate section heading {name:?}"
                )));
            }
            let start = body_start + relative;
            if let Some(previous) = sections.last_mut() {
                previous.range.end = start;
            }
            sections.push(Span {
                name: name.into(),
                range: start + line.len()..text.len(),
            });
        }
    }
    Ok(sections)
}

pub fn render_document(
    frontmatter: &[(&str, Option<&str>)],
    sections: &[(&str, &str)],
    title: &str,
) -> Result<MarkdownDocument, CodecError> {
    if unicode::has_line_break(title) {
        return Err(CodecError::malformed("document title must occupy one line"));
    }
    let mut text = String::from("---\n");
    for (key, value) in frontmatter {
        if unicode::has_line_break(key) || value.is_some_and(unicode::has_line_break) {
            return Err(CodecError::malformed(format!(
                "frontmatter field {key:?} must occupy one line"
            )));
        }
        text.push_str(key);
        text.push_str(": ");
        text.push_str(value.unwrap_or(""));
        text.push('\n');
    }
    text.push_str("---\n");
    if !title.is_empty() {
        text.push_str(&format!("\n# {title}\n"));
    }
    for (name, content) in sections {
        if unicode::has_line_break(name) {
            return Err(CodecError::malformed(
                "section heading must occupy one line",
            ));
        }
        text.push_str(&format!("\n## {name}\n\n{}\n", unicode::trim(content)));
    }
    parse_document(text)
}
