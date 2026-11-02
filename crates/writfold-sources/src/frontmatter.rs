use crate::{
    CodecError,
    documents::{MarkdownDocument, parse_document},
    unicode,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterBlock {
    document: Option<MarkdownDocument>,
}

impl FrontmatterBlock {
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.document.iter().flat_map(MarkdownDocument::frontmatter)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.document
            .as_ref()
            .and_then(|document| document.field(key))
    }

    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get(key).unwrap_or(default)
    }
}

/// Navigation permits missing frontmatter, but never ambiguous keys.
pub fn parse_frontmatter(text: &str) -> Result<FrontmatterBlock, CodecError> {
    let mut lines = unicode::lines(text);
    let Some((_, first)) = lines.next() else {
        return Ok(FrontmatterBlock { document: None });
    };
    if unicode::trim(first) != "---" {
        return Ok(FrontmatterBlock { document: None });
    }
    let mut normalized = unicode::without_line_end(first).to_owned();
    for (_, line) in lines {
        normalized.push('\n');
        normalized.push_str(unicode::without_line_end(line));
        if unicode::trim(line) == "---" {
            return Ok(FrontmatterBlock {
                document: Some(parse_document(normalized)?),
            });
        }
    }
    Ok(FrontmatterBlock {
        document: Some(parse_document(text)?),
    })
}

/// Caller-owned ordered entries (rather than a map that discards source order).
pub fn read_frontmatter(text: &str) -> Result<Vec<(String, String)>, CodecError> {
    Ok(parse_frontmatter(text)?
        .entries()
        .map(|(key, value)| (key.into(), value.into()))
        .collect())
}

pub fn split_commas_respecting_brackets(raw: &str) -> Vec<&str> {
    let track_quotes = raw.bytes().filter(|&byte| byte == b'"').count() % 2 == 0;
    let mut quoted = false;
    let mut depth = 0usize;
    let mut start = 0;
    let mut items = Vec::new();
    for (index, character) in raw.char_indices() {
        match character {
            '"' if track_quotes => quoted = !quoted,
            '⟦' if !quoted => depth += 1,
            '⟧' if !quoted => depth = depth.saturating_sub(1),
            ',' if depth == 0 && !quoted => {
                let item = unicode::trim(&raw[start..index]);
                if !item.is_empty() {
                    items.push(item);
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    let item = unicode::trim(&raw[start..]);
    if !item.is_empty() {
        items.push(item);
    }
    items
}

pub fn split_list_value(raw: &str) -> Vec<&str> {
    let raw = unicode::trim(raw);
    raw.strip_prefix('[')
        .and_then(|body| body.strip_suffix(']'))
        .map_or_else(Vec::new, split_commas_respecting_brackets)
}
