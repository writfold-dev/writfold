use crate::{
    CodecError,
    documents::{MarkdownDocument, parse_document},
    unicode,
};
use std::collections::HashSet;
use std::ops::Range;

struct Patch<'a> {
    range: Range<usize>,
    value: &'a str,
}

fn apply(text: &str, patches: &[Patch<'_>]) -> String {
    let capacity = patches.iter().fold(text.len(), |size, patch| {
        size - patch.range.len() + patch.value.len()
    });
    let mut output = String::with_capacity(capacity);
    let mut offset = 0;
    for patch in patches {
        output.push_str(&text[offset..patch.range.start]);
        output.push_str(patch.value);
        offset = patch.range.end;
    }
    output.push_str(&text[offset..]);
    output
}

fn normalize(content: &str, newline: &str) -> String {
    let mut output = String::with_capacity(content.len());
    let mut characters = content.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\r' {
            if characters.peek() == Some(&'\n') {
                characters.next();
            }
            output.push_str(newline);
        } else if character == '\n' {
            output.push_str(newline);
        } else {
            output.push(character);
        }
    }
    output
}

impl MarkdownDocument {
    /// Ordered changes; None clears a field rather than removing its authored line.
    pub fn update_fields(&self, changes: &[(&str, Option<&str>)]) -> Result<Self, CodecError> {
        let mut names = HashSet::new();
        for (key, value) in changes {
            if unicode::has_line_break(key) || value.is_some_and(unicode::has_line_break) {
                return Err(CodecError::malformed(format!(
                    "frontmatter field {key:?} must occupy one line"
                )));
            }
            if !names.insert(*key) {
                return Err(CodecError::malformed(format!(
                    "duplicate field change {key:?}"
                )));
            }
        }
        let mut patches = Vec::new();
        for span in &self.fields {
            if let Some((_, value)) = changes.iter().find(|(key, _)| *key == span.name) {
                let value = value.unwrap_or("");
                patches.push(Patch {
                    range: span.range.clone(),
                    value,
                });
                names.remove(span.name.as_str());
            }
        }
        let mut insertion = String::new();
        for (key, value) in changes {
            if !names.contains(key) {
                continue;
            }
            let value = value.unwrap_or("");
            if key.is_empty() || key.contains(':') {
                return Err(CodecError::malformed(format!(
                    "invalid frontmatter field {key:?}"
                )));
            }
            insertion.push_str(&format!("{key}: {value}{}", self.newline()));
        }
        if !insertion.is_empty() {
            patches.push(Patch {
                range: self.frontmatter_end..self.frontmatter_end,
                value: &insertion,
            });
        }
        // Python sorts (start, end, value) tuples before reverse application;
        // the value tie-break matters for empty spans at Unicode line breaks.
        patches.sort_unstable_by(|left, right| {
            (left.range.start, left.range.end, left.value).cmp(&(
                right.range.start,
                right.range.end,
                right.value,
            ))
        });
        parse_document(apply(&self.text, &patches))
    }

    pub fn update_section(&self, name: &str, content: &str) -> Result<Self, CodecError> {
        if unicode::has_line_break(name) {
            return Err(CodecError::malformed(
                "section heading must occupy one line",
            ));
        }
        let content = normalize(content, self.newline());
        let content = unicode::trim(&content);
        if let Some(span) = self.sections.iter().find(|span| span.name == name) {
            let raw = &self.text[span.range.clone()];
            let (range, replacement);
            if !unicode::trim(raw).is_empty() {
                let start = span.range.start + raw.len() - unicode::trim_start(raw).len();
                let end = span.range.start + unicode::trim_end(raw).len();
                return parse_document(apply(
                    &self.text,
                    &[Patch {
                        range: start..end,
                        value: content,
                    }],
                ));
            } else {
                range = span.range.clone();
                let prefix = if raw.is_empty() { self.newline() } else { raw };
                let suffix = if span.range.end < self.text.len() {
                    self.newline().repeat(2)
                } else {
                    self.newline().into()
                };
                replacement = format!("{prefix}{content}{suffix}");
            }
            return parse_document(apply(
                &self.text,
                &[Patch {
                    range,
                    value: &replacement,
                }],
            ));
        }
        let newline = self.newline();
        let separator = if self.text.ends_with(&newline.repeat(2)) {
            ""
        } else if self.text.ends_with(newline) {
            newline
        } else {
            ""
        };
        let extra = if self.text.ends_with(newline) {
            ""
        } else {
            newline
        };
        let suffix =
            format!("{separator}{extra}{extra}## {name}{newline}{newline}{content}{newline}");
        parse_document(apply(
            &self.text,
            &[Patch {
                range: self.text.len()..self.text.len(),
                value: &suffix,
            }],
        ))
    }

    pub fn append_section(&self, name: &str, content: &str) -> Result<Self, CodecError> {
        let Some(span) = self.sections.iter().find(|span| span.name == name) else {
            return self.update_section(name, content);
        };
        let content = normalize(content, self.newline());
        let raw = &self.text[span.range.clone()];
        let nonempty = !unicode::trim(raw).is_empty();
        let end = if nonempty {
            span.range.start + raw.trim_end_matches(['\r', '\n']).len()
        } else {
            span.range.end
        };
        let prefix = if nonempty || !self.text[..end].ends_with(self.newline()) {
            self.newline()
        } else {
            ""
        };
        let suffix = if self.text[end..].starts_with(['\n', '\r']) {
            ""
        } else {
            self.newline()
        };
        let insertion = format!("{prefix}{}{suffix}", unicode::trim(&content));
        parse_document(apply(
            &self.text,
            &[Patch {
                range: end..end,
                value: &insertion,
            }],
        ))
    }
}
