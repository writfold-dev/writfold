//! Checkpoint drafts: local, untracked notes the harness hooks keep for one
//! session until someone confirms one with `checkpoint --from-draft`.
//!
//! A draft is `<git dir>/writfold/drafts/<session>.json`, beside the index
//! and the current-run pointer. It is never committed and vanishes with
//! the clone. Hooks fill it: mechanical facts before a compaction, the
//! compaction summary after it, the last assistant message at each stop,
//! and the end reason. Confirming one appends its text to a run's
//! `checkpoint.md` through the write protocol and then removes the draft.
use crate::error::{KbError, Result};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "writfold-draft/1";
/// The longest text a draft keeps per field, in characters.
pub const TEXT_LIMIT: usize = 4000;

/// The drafts directory under a KB's local state (`<git dir>/writfold`).
pub fn dir(state: &Path) -> PathBuf {
    state.join("drafts")
}

/// A session ID usable as a file name: 1 to 128 of `[A-Za-z0-9._-]`, not
/// starting with a dot.
pub fn valid_session(id: &str) -> bool {
    (1..=128).contains(&id.len())
        && !id.starts_with('.')
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// The draft file of a session.
pub fn path(state: &Path, session: &str) -> Result<PathBuf> {
    if !valid_session(session) {
        return Err(KbError::Usage(format!(
            "{session:?} is not a session ID (letters, digits, '.', '-', '_')"
        )));
    }
    Ok(dir(state).join(format!("{session}.json")))
}

/// A session's draft, if present and a JSON object.
pub fn load(state: &Path, session: &str) -> Result<Option<Map<String, Value>>> {
    let file = path(state, session)?;
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    match serde_json::from_str(&text) {
        Ok(Value::Object(map)) => Ok(Some(map)),
        _ => Err(KbError::Usage(format!("{} is not a draft", file.display()))),
    }
}

/// Write a draft whole: a temporary beside it, then a rename.
pub fn save(state: &Path, session: &str, draft: &Map<String, Value>) -> Result<()> {
    let file = path(state, session)?;
    std::fs::create_dir_all(dir(state))?;
    let temporary = dir(state).join(format!(".{session}-{}.tmp", std::process::id()));
    let text = serde_json::to_string_pretty(&Value::Object(draft.clone()))
        .map_err(|e| KbError::Usage(format!("draft: {e}")))?;
    std::fs::write(&temporary, format!("{text}\n"))?;
    std::fs::rename(&temporary, &file)?;
    Ok(())
}

/// Remove a session's draft (after it was confirmed).
pub fn remove(state: &Path, session: &str) -> Result<()> {
    match std::fs::remove_file(path(state, session)?) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

/// Every readable draft, newest `updated` first, then by session.
pub fn all(state: &Path) -> Vec<(String, Map<String, Value>)> {
    let Ok(entries) = std::fs::read_dir(dir(state)) else {
        return Vec::new();
    };
    let mut out: Vec<(String, Map<String, Value>)> = entries
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().to_str()?.to_owned();
            let session = name.strip_suffix(".json")?;
            Some((session.to_owned(), load(state, session).ok()??))
        })
        .collect();
    out.sort_by(|a, b| {
        let at = |m: &Map<String, Value>| text(m, "updated").unwrap_or_default();
        at(&b.1).cmp(&at(&a.1)).then_with(|| a.0.cmp(&b.0))
    });
    out
}

/// A string field.
pub fn text(draft: &Map<String, Value>, key: &str) -> Option<String> {
    draft
        .get(key)?
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
}

/// At most [`TEXT_LIMIT`] characters, cut at a line break when one is
/// near, with a marker when cut.
pub fn clip(text: &str) -> String {
    let text = text.trim();
    let Some((cut, _)) = text.char_indices().nth(TEXT_LIMIT) else {
        return text.to_owned();
    };
    let head = &text[..cut];
    let head = match head.rfind('\n') {
        Some(at) if at > cut / 2 => &head[..at],
        _ => head,
    };
    format!("{}\n[… cut at {TEXT_LIMIT} characters]", head.trim_end())
}

/// The `<summary>` block of a compaction summary when it has one (its
/// `<analysis>` preamble is the model's working, not the checkpoint), else
/// the whole text.
pub fn summary_block(text: &str) -> &str {
    let Some((_, rest)) = text.split_once("<summary>") else {
        return text;
    };
    rest.split_once("</summary>")
        .map_or(rest, |(inner, _)| inner)
        .trim()
}

/// The draft's body: the compaction summary, else the last assistant
/// message.
pub fn body(draft: &Map<String, Value>) -> Option<String> {
    text(draft, "summary")
        .map(|s| summary_block(&s).to_owned())
        .or_else(|| text(draft, "last_message"))
}

/// The checkpoint note a draft becomes: its body made safe for
/// `checkpoint.md` (a `## ` line would start a new entry, an unclosed
/// fence would swallow later ones), then where it came from.
pub fn note(session: &str, draft: &Map<String, Value>) -> Result<String> {
    let body = body(draft).ok_or_else(|| {
        KbError::Refused(format!(
            "draft {session} holds no text yet (no stop or compaction recorded)"
        ))
    })?;
    let mut out = String::new();
    let mut fences = 0usize;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fences += 1;
        }
        if line.starts_with("## ") || line.starts_with("# ") {
            out.push_str("##");
        }
        out.push_str(line);
        out.push('\n');
    }
    if fences % 2 == 1 {
        out.push_str("```\n");
    }
    let mut origin = format!("(draft from session {session}");
    if let Some(place) = draft.get("place").and_then(Value::as_object) {
        for key in ["repo", "branch", "head"] {
            if let Some(value) = place.get(key).and_then(Value::as_str) {
                let value = if key == "head" {
                    value.get(..12).unwrap_or(value)
                } else {
                    value
                };
                origin.push_str(&format!(" {key}={value}"));
            }
        }
    }
    Ok(format!("{}\n\n{origin})", out.trim_end()))
}

#[cfg(test)]
mod tests {
    use super::{clip, note, summary_block, valid_session};
    use serde_json::{Map, Value, json};

    fn draft(value: Value) -> Map<String, Value> {
        match value {
            Value::Object(map) => map,
            _ => Map::new(),
        }
    }

    #[test]
    fn sessions_and_notes() {
        assert!(valid_session("abc-123_x.y"));
        assert!(!valid_session("") && !valid_session("../x") && !valid_session(".x"));
        assert!(!valid_session(&"a".repeat(129)));
        let d = draft(json!({"last_message": "## Done\nok\n```\ncode",
                              "place": {"repo": "demo", "head": "0123456789abcdef"}}));
        let made = note("s1", &d).unwrap_or_default();
        assert_eq!(
            made,
            "#### Done\nok\n```\ncode\n```\n\n(draft from session s1 repo=demo head=0123456789ab)"
        );
        let summary = draft(json!({"summary": "S", "last_message": "M"}));
        assert!(note("s", &summary).is_ok_and(|n| n.starts_with("S\n")));
        assert!(note("s", &draft(json!({}))).is_err());
        let long = "x".repeat(5000);
        assert!(clip(&long).ends_with("[… cut at 4000 characters]"));
        assert_eq!(clip(" short "), "short");
        assert_eq!(
            summary_block("<analysis>a</analysis>\n<summary>\nS\n</summary>"),
            "S"
        );
        assert_eq!(summary_block("plain"), "plain");
    }
}
