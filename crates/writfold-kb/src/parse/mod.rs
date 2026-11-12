//! Tolerant decoders: one file in, facts out, no IO. A file that cannot
//! yield what its kind needs is unreadable, with a reason; unknown fields and
//! unknown vocabulary are kept, never rejected.
pub mod cite;
mod document;
pub(crate) mod json;
pub mod mention;

use crate::layout::Kind;
use crate::model::{Facts, Reference, Section};

/// Facts for `path` (root-relative) of `kind`, or why it is unreadable.
pub fn parse(path: &str, kind: Kind, bytes: &[u8]) -> Result<Facts, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "not UTF-8 text".to_owned())?;
    let mut facts = match kind {
        Kind::Asset | Kind::Relationship | Kind::Run => json::parse(path, kind, text)?,
        _ if path.ends_with(".json") => json::document(path, text)?,
        _ => document::parse(path, kind, text)?,
    };
    let markdown = path.ends_with(".md");
    if markdown {
        facts.sections = sections(text);
        facts.citations.extend(cite::tokens(text));
        facts.heading = first_heading(text);
    }
    let src = facts
        .record
        .as_ref()
        .map_or_else(|| format!("doc:{path}"), |r| r.id.clone());
    for (ord, citation) in facts.citations.iter().enumerate() {
        if let Some(token) = cite::target(citation) {
            facts.references.push(Reference {
                line: citation.line,
                src: src.clone(),
                rel: "cites".into(),
                token,
                origin: "cite",
                status: "explicit".into(),
                cite: u32::try_from(ord).ok(),
            });
        }
    }
    // Record JSON is structured: its fields are already field edges.
    let from = match kind {
        Kind::Asset | Kind::Relationship | Kind::Run => None,
        _ if markdown => Some(mention::body_start(text)),
        _ => Some(1),
    };
    for (token, line) in from
        .map(|from| mention::scan(text, from))
        .unwrap_or_default()
    {
        facts.references.push(Reference {
            line,
            src: src.clone(),
            rel: "mentions".into(),
            token,
            origin: "mention",
            status: "inferred".into(),
            cite: None,
        });
    }
    facts.body = text.to_owned();
    Ok(facts)
}

/// `## ` sections outside fenced code, with one-based inclusive line spans.
pub fn sections(text: &str) -> Vec<Section> {
    let mut found: Vec<Section> = Vec::new();
    let mut fence: Option<&str> = None;
    let mut count = 0u32;
    for (index, line) in text.lines().enumerate() {
        let number = u32::try_from(index + 1).unwrap_or(u32::MAX);
        count = number;
        let trimmed = line.trim_start();
        let marker = ["```", "~~~"].into_iter().find(|m| trimmed.starts_with(m));
        match (fence, marker) {
            (None, Some(marker)) => fence = Some(marker),
            (Some(open), Some(marker)) if open == marker => fence = None,
            _ => {}
        }
        if fence.is_some() || marker.is_some() {
            continue;
        }
        if let Some(name) = line.strip_prefix("## ") {
            if let Some(previous) = found.last_mut() {
                previous.last = number.saturating_sub(1);
            }
            found.push(Section {
                name: name.trim().to_owned(),
                first: number,
                last: number,
            });
        }
    }
    if let Some(previous) = found.last_mut() {
        previous.last = count;
    }
    found
}

/// The one-based line of front-matter `key:`, or 1.
pub fn field_line(text: &str, key: &str) -> u32 {
    let mut lines = text.lines().enumerate();
    if lines.next().is_none_or(|(_, first)| first.trim() != "---") {
        return 1;
    }
    for (index, line) in lines {
        if line.trim() == "---" {
            break;
        }
        if line
            .split_once(':')
            .is_some_and(|(name, _)| name.trim() == key)
        {
            return u32::try_from(index + 1).unwrap_or(1);
        }
    }
    1
}

/// The first `# ` heading outside the front matter, if any.
pub fn first_heading(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(|title| title.trim().to_owned())
        .filter(|title| !title.is_empty())
}

/// A list value written `[a, b]`; a bare non-empty value is one item.
pub fn list(value: &str) -> Vec<String> {
    let value = value.trim();
    if value.starts_with('[') {
        writfold_sources::frontmatter::split_list_value(value)
            .into_iter()
            .map(|item| unquote(item).to_owned())
            .filter(|item| !item.is_empty())
            .collect()
    } else if value.is_empty() || matches!(value, "null" | "none" | "~") {
        Vec::new()
    } else {
        vec![unquote(value).to_owned()]
    }
}

fn unquote(value: &str) -> &str {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(value)
}

/// `ABC-123` from forms such as `abc 123`, `ABC-123` or `{ABC123}`.
pub fn ticket(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let raw = raw.strip_prefix('{').unwrap_or(raw);
    let raw = raw.strip_suffix('}').unwrap_or(raw).trim();
    let letters = raw.bytes().take_while(u8::is_ascii_alphabetic).count();
    let rest = raw[letters..].trim_start_matches([' ', '\t', '-']);
    let well_formed = letters > 0 && !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit());
    well_formed.then(|| format!("{}-{rest}", raw[..letters].to_ascii_uppercase()))
}

/// A field reference with its provenance.
pub fn reference(src: &str, rel: &str, token: String, line: u32) -> Reference {
    Reference {
        line,
        src: src.to_owned(),
        rel: rel.to_owned(),
        token,
        origin: "field",
        status: "explicit".into(),
        cite: None,
    }
}

/// The token a typed list item resolves through: `prefix` is added unless
/// the item already carries it.
pub fn typed(prefix: &str, item: &str) -> String {
    if item.starts_with(prefix) {
        item.to_owned()
    } else {
        format!("{prefix}{item}")
    }
}

#[cfg(test)]
mod tests {
    use super::{field_line, list, sections, ticket};

    #[test]
    fn helpers() {
        let text = "---\nid: x\nstatus: open\n---\n# T\n## A\nx\n```\n## not\n```\n## B\ny\n";
        let found = sections(text);
        let names: Vec<_> = found
            .iter()
            .map(|s| (s.name.as_str(), s.first, s.last))
            .collect();
        assert_eq!(names, [("A", 6, 10), ("B", 11, 12)]);
        assert_eq!(field_line(text, "status"), 3);
        assert_eq!(list("[a, \"b, c\", ]"), ["a", "b, c"]);
        assert_eq!(list("solo"), ["solo"]);
        assert_eq!(ticket("abc 12").as_deref(), Some("ABC-12"));
        assert_eq!(ticket("{ABC-12}").as_deref(), Some("ABC-12"));
        assert_eq!(ticket("ABC-12x"), None);
    }
}
