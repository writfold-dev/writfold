//! Statements: the two obligation classes that are not checks. An
//! `attested` statement is what a human or an agent says they did ("I read
//! this for tone"); a `waived` statement sets an obligation aside, with a
//! reason. Both are recorded as such, shown under their own names, and
//! never counted as a passing check ([`writfold_core::gate::standing`]).
//!
//! One JSON file per statement, written create-new through the write
//! protocol: `runs/<run>/statements/<gate>-<time>-<hex>.json` for a gate,
//! `runs/<run>/deliveries/<candidate>/statements/<check>-<time>-<hex>.json`
//! for a delivery check. A statement binds the obligation's definition
//! digest and the subject's content digest (a gate's scope digest, a
//! candidate's bytes), so it stops standing once either moves, like a
//! receipt. Who made it is `by`, as given; Writfold cannot check it.
use super::receipt::canonical;
use crate::error::{KbError, Result};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;
use writfold_core::gate::Statements;

pub const SCHEMA: &str = "writfold-statement/1";
/// The directory, under a run or a candidate, that holds statements.
pub const DIR: &str = "statements";
/// The longest statement or reason kept.
const MAX_TEXT: usize = 2000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Attested,
    Waived,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Attested => "attested",
            Kind::Waived => "waived",
        }
    }
}

/// One statement, as written and as read back.
#[derive(Clone, Debug)]
pub struct Statement {
    pub kind: Kind,
    /// The gate ID or delivery check name.
    pub obligation: String,
    pub run: String,
    /// The subject: a repository name (gates) or a candidate ID.
    pub subject: String,
    /// The subject repository's commit (gates only).
    pub commit: Option<String>,
    /// The digest of the subject's content: a scope digest or the
    /// candidate's bytes.
    pub content: String,
    /// The obligation's definition digest.
    pub definition: String,
    /// The statement (attested) or the reason (waived).
    pub text: String,
    pub by: String,
    pub recorded: String,
}

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{}", crate::index::hex(&Sha256::digest(bytes)))
}

/// Check the free-text fields of a statement before writing it.
pub fn check_text(flag: &str, text: &str) -> Result<String> {
    let text = crate::write::one_line(flag, text)?;
    if text.chars().count() > MAX_TEXT {
        return Err(KbError::Usage(format!(
            "{flag} is longer than {MAX_TEXT} characters"
        )));
    }
    Ok(text)
}

/// The statement's bytes: two-space JSON ending in a line break, with a
/// digest over the canonical form of the other fields.
pub fn bytes(s: &Statement) -> Vec<u8> {
    let mut value = json!({
        "schema": SCHEMA,
        "kind": s.kind.name(),
        "obligation": s.obligation,
        "run": s.run,
        "subject": {"name": s.subject, "commit": s.commit, "content": s.content},
        "definition": s.definition,
        "text": s.text,
        "by": s.by,
        "recorded": s.recorded,
    });
    let digest = sha(canonical(&value).as_bytes());
    if let Value::Object(map) = &mut value {
        map.insert("digest".into(), Value::String(digest));
    }
    let mut out = serde_json::to_vec_pretty(&value).unwrap_or_default();
    out.push(b'\n');
    out
}

fn text_at(value: &Value, path: &[&str]) -> std::result::Result<String, String> {
    path.iter()
        .try_fold(value, |v, k| v.get(k))
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty() && !t.contains(['\n', '\r']))
        .map(str::to_owned)
        .ok_or_else(|| format!("{} is missing or not a one-line string", path.join(".")))
}

/// Read one statement of `run` filed as `name` (`<obligation>-….json`).
pub fn parse(text: &str, run: &str, name: &str) -> std::result::Result<Statement, String> {
    let mut value: Value = serde_json::from_str(text).map_err(|e| format!("not JSON ({e})"))?;
    let claimed = value
        .as_object_mut()
        .ok_or("not a JSON object")?
        .remove("digest");
    if text_at(&value, &["schema"])? != SCHEMA {
        return Err(format!("schema is not {SCHEMA}"));
    }
    if claimed.as_ref().and_then(Value::as_str) != Some(sha(canonical(&value).as_bytes()).as_str())
    {
        return Err("its digest does not match its content".into());
    }
    let kind = match text_at(&value, &["kind"])?.as_str() {
        "attested" => Kind::Attested,
        "waived" => Kind::Waived,
        other => return Err(format!("kind {other} is not attested or waived")),
    };
    let obligation = text_at(&value, &["obligation"])?;
    let filed_run = text_at(&value, &["run"])?;
    if filed_run != run || !name.starts_with(&format!("{obligation}-")) {
        return Err("its run or obligation does not match where it is filed".into());
    }
    let commit = match value.pointer("/subject/commit") {
        Some(Value::Null) | None => None,
        Some(_) => Some(text_at(&value, &["subject", "commit"])?),
    };
    Ok(Statement {
        kind,
        obligation,
        run: filed_run,
        subject: text_at(&value, &["subject", "name"])?,
        commit,
        content: text_at(&value, &["subject", "content"])?,
        definition: text_at(&value, &["definition"])?,
        text: text_at(&value, &["text"])?,
        by: text_at(&value, &["by"])?,
        recorded: text_at(&value, &["recorded"])?,
    })
}

/// One statement file.
pub struct Filed {
    /// KB-relative path.
    pub path: String,
    pub name: String,
    pub parsed: std::result::Result<Statement, String>,
}

/// Every statement file under `<dir>/statements/` (KB-relative `dir`), by
/// name.
pub fn filed(root: &Path, dir: &str, run: &str) -> Vec<Filed> {
    let Ok(entries) = std::fs::read_dir(root.join(dir).join(DIR)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json") && !n.starts_with('.'))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let path = format!("{dir}/{DIR}/{name}");
            let parsed = std::fs::read_to_string(root.join(&path))
                .map_err(|e| format!("unreadable ({e})"))
                .and_then(|text| parse(&text, run, &name));
            Filed { path, name, parsed }
        })
        .collect()
}

/// What the statements say about one obligation on one subject: the
/// kernel's summary, and the newest current attestation and waiver.
pub struct Summary<'a> {
    pub statements: Statements,
    pub attested: Option<(&'a Filed, &'a Statement)>,
    pub waived: Option<(&'a Filed, &'a Statement)>,
    /// A waiver exists but is about other content or another definition.
    pub waiver_moved: bool,
}

/// Summarise the valid statements about `obligation` on `subject`.
pub fn summary<'a>(
    filed: &'a [Filed],
    obligation: &str,
    subject: &str,
    definition: &str,
    content: &str,
) -> Summary<'a> {
    let mut mine: Vec<(&Filed, &Statement)> = filed
        .iter()
        .filter_map(|f| f.parsed.as_ref().ok().map(|s| (f, s)))
        .filter(|(_, s)| s.obligation == obligation && s.subject == subject)
        .collect();
    mine.sort_by(|a, b| (&a.1.recorded, &a.0.name).cmp(&(&b.1.recorded, &b.0.name)));
    let current = |s: &Statement| s.definition == definition && s.content == content;
    let newest = |kind: Kind| {
        mine.iter()
            .rev()
            .find(|(_, s)| s.kind == kind && current(s))
            .copied()
    };
    let attested = newest(Kind::Attested);
    let waived = newest(Kind::Waived);
    Summary {
        statements: Statements {
            attested_current: attested.is_some(),
            attested_any: mine.iter().any(|(_, s)| s.kind == Kind::Attested),
            waived_current: waived.is_some(),
        },
        attested,
        waived,
        waiver_moved: waived.is_none() && mine.iter().any(|(_, s)| s.kind == Kind::Waived),
    }
}

/// The continuation line a standing statement shows: `attested: <text>
/// by <who>` or `waived: <reason> by <who>`, at the statement's file.
pub fn more(f: &Filed, s: &Statement) -> crate::emit::More {
    crate::emit::More {
        status: None,
        label: s.kind.name(),
        text: format!("{} (by {}, {})", s.text, s.by, s.recorded),
        path: Some(f.path.clone()),
        line: Some(1),
        flags: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, Statement, bytes, parse};

    fn sample() -> Statement {
        Statement {
            kind: Kind::Waived,
            obligation: "unit".into(),
            run: "run-1".into(),
            subject: "demo".into(),
            commit: Some("abc".into()),
            content: "sha256:00".into(),
            definition: "sha256:11".into(),
            text: "flaky on this host; CI covers it".into(),
            by: "owner".into(),
            recorded: "2026-11-11T00:00:00Z".into(),
        }
    }

    #[test]
    fn statements_round_trip_and_edits_are_invalid() -> Result<(), String> {
        let text = String::from_utf8(bytes(&sample())).map_err(|e| e.to_string())?;
        let back = parse(&text, "run-1", "unit-x.json")?;
        assert_eq!(
            (back.kind, back.text.as_str()),
            (Kind::Waived, sample().text.as_str())
        );
        assert!(
            parse(
                &text.replace("\"waived\"", "\"attested\""),
                "run-1",
                "unit-x.json"
            )
            .is_err()
        );
        assert!(parse(&text, "run-2", "unit-x.json").is_err());
        assert!(parse(&text, "run-1", "lint-x.json").is_err());
        Ok(())
    }
}
