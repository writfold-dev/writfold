use crate::{CodecError, unicode};
use std::{collections::HashSet, ops::Range};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Field<'a> {
    key: &'a str,
    value: &'a str,
    range: Range<usize>,
}

/// Ordered borrowed decoded fields; duplicates cannot be constructed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiteFields<'a> {
    fields: Vec<Field<'a>>,
}

impl<'a> CiteFields<'a> {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&'a str, &'a str)> + '_ {
        self.fields.iter().map(|field| (field.key, field.value))
    }

    pub fn get(&self, key: &str) -> Option<&'a str> {
        self.fields
            .iter()
            .find(|field| field.key == key)
            .map(|field| field.value)
    }

    pub fn span(&self, key: &str) -> Option<Range<usize>> {
        self.fields
            .iter()
            .find(|field| field.key == key)
            .map(|field| field.range.clone())
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

/// Python FIELD_RE.finditer semantics, including ignored malformed surrounding text.
pub fn parse_cite_body(body: &str) -> Result<CiteFields<'_>, CodecError> {
    let mut offset = 0;
    let mut fields = Vec::new();
    let mut keys = HashSet::new();
    while offset < body.len() {
        let Some((relative, _)) = body[offset..]
            .char_indices()
            .find(|&(_, c)| unicode::word(c))
        else {
            break;
        };
        let start = offset + relative;
        let key_len = body[start..]
            .chars()
            .take_while(|&c| unicode::word(c))
            .map(char::len_utf8)
            .sum::<usize>();
        let key_end = start + key_len;
        offset = key_end;
        if !body[key_end..].starts_with('=') {
            continue;
        }
        let value_start = key_end + 1;
        let Some((value, end)) = field_value(body, value_start) else {
            continue;
        };
        let key = &body[start..key_end];
        if !keys.insert(key) {
            return Err(CodecError::malformed(format!(
                "duplicate citation field {key:?}"
            )));
        }
        fields.push(Field {
            key,
            value,
            range: start..end,
        });
        offset = end;
    }
    Ok(CiteFields { fields })
}

fn field_value(body: &str, start: usize) -> Option<(&str, usize)> {
    let rest = &body[start..];
    if let Some(quoted) = rest.strip_prefix('"')
        && let Some(end) = quoted.find('"')
    {
        return Some((&quoted[..end], start + 1 + end + 1));
    }
    let length = rest
        .chars()
        .take_while(|&c| !writfold_domain::text::whitespace(c) && c != '⟧')
        .map(char::len_utf8)
        .sum::<usize>();
    (length > 0).then_some((&rest[..length], start + length))
}

pub(crate) fn format_field(key: &str, value: &str) -> Result<String, CodecError> {
    if key.is_empty() || !key.chars().all(unicode::word) || value.contains('⟧') {
        return Err(CodecError::malformed(format!(
            "unrepresentable citation field {key:?}"
        )));
    }
    if value.chars().any(writfold_domain::text::whitespace)
        || value.is_empty()
        || value.starts_with('"')
    {
        if value.contains('"') {
            return Err(CodecError::malformed(format!(
                "citation field {key:?} cannot quote embedded double quotes"
            )));
        }
        Ok(format!("{key}=\"{value}\""))
    } else {
        Ok(format!("{key}={value}"))
    }
}

pub fn format_cite(fields: &[(&str, &str)], order: &[&str]) -> Result<String, CodecError> {
    let mut keys = HashSet::new();
    for (key, _) in fields {
        if !keys.insert(*key) {
            return Err(CodecError::malformed(format!(
                "duplicate citation field {key:?}"
            )));
        }
    }
    let mut emitted = HashSet::new();
    let mut output = String::from("⟦cite ");
    let mut separator = "";
    for key in order
        .iter()
        .copied()
        .chain(fields.iter().map(|(key, _)| *key))
    {
        if emitted.contains(key) {
            continue;
        }
        if let Some((_, value)) = fields
            .iter()
            .find(|(name, value)| *name == key && !value.is_empty())
        {
            output.push_str(separator);
            output.push_str(&format_field(key, value)?);
            separator = " ";
            emitted.insert(key);
        }
    }
    output.push('⟧');
    Ok(output)
}
