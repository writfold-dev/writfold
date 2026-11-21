//! Check receipts: `.writfold/checks/<kind>/<time>-<hex>.json`, what a
//! checker said about each token in one `kb stale --record` run, keyed by
//! the SHA-256 of the token's exact text. Written once, never edited; read
//! strictly (a file with anything unexpected is skipped whole, with a note,
//! and is never evidence). A dot path, so never a record.
use super::super::kinds::Kinds;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use writfold_core::verdict::Said;

pub const DIR: &str = ".writfold/checks";
pub const SCHEMA: &str = "writfold-check-receipt/1";
/// A larger file is skipped unread.
pub const LIMIT: u64 = 4 * 1024 * 1024;

/// Where a checker said the cited content moved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Moved {
    pub path: Option<String>,
    pub first: u32,
    pub last: u32,
}

/// One token's recorded answer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    /// `sha256:<64 hex>` of the token's exact text.
    pub token: String,
    pub said: Said,
    pub at: Option<String>,
    pub detail: Option<String>,
    pub moved: Option<Moved>,
}

/// One receipt file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    /// Relative to the KB root.
    pub path: String,
    pub kind: String,
    pub checker: String,
    pub version: Option<String>,
    pub protocol: Option<String>,
    /// As written, and in seconds since the epoch.
    pub checked_at: String,
    pub at: i64,
    pub verdicts: Vec<Entry>,
}

/// The key a receipt files a token under.
pub fn digest(raw: &str) -> String {
    format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(raw.as_bytes()))
    )
}

pub fn said_name(said: Said) -> &'static str {
    match said {
        Said::Fresh => "fresh",
        Said::Moved => "moved",
        Said::Changed => "changed",
        Said::Gone => "gone",
        Said::Unverifiable => "unverifiable",
    }
}

fn keys(object: &Map<String, Value>, allowed: &[&str], what: &str) -> Result<(), String> {
    match object.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(key) => Err(format!("unknown key {key} in {what}")),
        None => Ok(()),
    }
}

fn text(object: &Map<String, Value>, key: &str, what: &str) -> Result<Option<String>, String> {
    match object.get(key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("{what} {key} is not a string")),
    }
}

fn line(object: &Map<String, Value>, key: &str) -> Option<u32> {
    object
        .get(key)?
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
}

fn entry(value: &Value) -> Result<Entry, String> {
    let object = value.as_object().ok_or("a verdict that is not an object")?;
    keys(
        object,
        &["token", "status", "at", "detail", "moved"],
        "a verdict",
    )?;
    let token = text(object, "token", "verdict")?.ok_or("a verdict without token")?;
    let hex = token.strip_prefix("sha256:").unwrap_or_default();
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("token {token:?} is not sha256: and 64 hex digits"));
    }
    let said = match text(object, "status", "verdict")?.as_deref() {
        Some("fresh") => Said::Fresh,
        Some("moved") => Said::Moved,
        Some("changed") => Said::Changed,
        Some("gone") => Said::Gone,
        other => {
            return Err(format!(
                "status {other:?} is not fresh, moved, changed or gone"
            ));
        }
    };
    let moved = match (object.get("moved"), said) {
        (None, Said::Moved) => return Err(format!("moved without a range for {token}")),
        (None, _) => None,
        (Some(_), s) if s != Said::Moved => return Err(format!("moved beside {}", said_name(s))),
        (Some(m), _) => {
            let m = m.as_object().ok_or("moved is not an object")?;
            keys(m, &["path", "first", "last"], "moved")?;
            match (line(m, "first"), line(m, "last")) {
                (Some(first), Some(last)) if first >= 1 && last >= first => Some(Moved {
                    path: text(m, "path", "moved")?,
                    first,
                    last,
                }),
                _ => return Err(format!("moved without a valid range for {token}")),
            }
        }
    };
    Ok(Entry {
        at: text(object, "at", "verdict")?,
        detail: text(object, "detail", "verdict")?,
        token,
        said,
        moved,
    })
}

/// Read one receipt strictly. `kind` is its directory's name.
pub fn parse(path: &str, kind: &str, bytes: &str) -> Result<Receipt, String> {
    let value: Value = serde_json::from_str(bytes).map_err(|e| format!("not JSON: {e}"))?;
    let object = value.as_object().ok_or("not a JSON object")?;
    keys(
        object,
        &["schema", "kind", "checker", "checked_at", "verdicts"],
        "the receipt",
    )?;
    if object.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return Err(format!("schema is not {SCHEMA}"));
    }
    if object.get("kind").and_then(Value::as_str) != Some(kind) {
        return Err(format!("kind is not {kind}, its directory"));
    }
    let checker = object.get("checker").and_then(Value::as_object);
    let checker = checker.ok_or("checker is not an object")?;
    keys(checker, &["name", "version", "protocol"], "checker")?;
    let name = text(checker, "name", "checker")?.ok_or("checker without name")?;
    let checked_at = text(object, "checked_at", "receipt")?.unwrap_or_default();
    let at = crate::time::parse(&checked_at)
        .filter(|_| checked_at.len() == 20)
        .ok_or(format!("checked_at {checked_at:?} is unreadable"))?;
    let list = object.get("verdicts").and_then(Value::as_array);
    let verdicts = list
        .ok_or("verdicts is not a list")?
        .iter()
        .map(entry)
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = std::collections::BTreeSet::new();
    if let Some(dup) = verdicts.iter().find(|e| !seen.insert(e.token.as_str())) {
        return Err(format!("duplicate token {}", dup.token));
    }
    Ok(Receipt {
        path: path.to_owned(),
        kind: kind.to_owned(),
        version: text(checker, "version", "checker")?,
        protocol: text(checker, "protocol", "checker")?,
        checker: name,
        checked_at,
        at,
        verdicts,
    })
}

/// The file's bytes: one verdict per line, sorted by token.
pub fn encode(receipt: &Receipt) -> String {
    let mut checker = Map::new();
    checker.insert("name".into(), receipt.checker.clone().into());
    if let Some(v) = &receipt.version {
        checker.insert("version".into(), v.clone().into());
    }
    if let Some(p) = &receipt.protocol {
        checker.insert("protocol".into(), p.clone().into());
    }
    let head = |key: &str, value: Value| format!("  {}: {value},\n", Value::from(key));
    let mut out = String::from("{\n");
    out.push_str(&head("schema", SCHEMA.into()));
    out.push_str(&head("kind", receipt.kind.clone().into()));
    out.push_str(&head("checker", Value::Object(checker)));
    out.push_str(&head("checked_at", receipt.checked_at.clone().into()));
    out.push_str("  \"verdicts\": [\n");
    let mut sorted: Vec<&Entry> = receipt.verdicts.iter().collect();
    sorted.sort_by(|a, b| a.token.cmp(&b.token));
    let lines: Vec<String> = sorted
        .iter()
        .map(|e| format!("    {}", entry_json(e)))
        .collect();
    out.push_str(&lines.join(",\n"));
    out.push_str(if lines.is_empty() {
        "  ]\n}\n"
    } else {
        "\n  ]\n}\n"
    });
    out
}

/// One verdict on one line, keys in a fixed order: token, status, at,
/// moved, detail.
fn entry_json(e: &Entry) -> String {
    let s = |v: &str| Value::from(v).to_string();
    let mut out = format!(
        "{{\"token\":{},\"status\":{}",
        s(&e.token),
        s(said_name(e.said))
    );
    if let Some(at) = &e.at {
        out.push_str(&format!(",\"at\":{}", s(at)));
    }
    if let Some(m) = &e.moved {
        let path = m.path.as_deref().map(|p| format!(",\"path\":{}", s(p)));
        out.push_str(&format!(
            ",\"moved\":{{\"first\":{},\"last\":{}{}}}",
            m.first,
            m.last,
            path.unwrap_or_default()
        ));
    }
    if let Some(d) = &e.detail {
        out.push_str(&format!(",\"detail\":{}", s(d)));
    }
    out.push('}');
    out
}

/// Every readable receipt of `kind` (by file name), and a note for each
/// file skipped. Another checker's receipts are left out without a note.
pub fn read_kind(root: &Path, kind: &str, checker: &str) -> (Vec<Receipt>, Vec<String>) {
    let dir = root.join(DIR).join(kind);
    let (names, fault) = crate::gate::discover::evidence(&dir, crate::gate::discover::list);
    let (mut found, mut notes) = (Vec::new(), Vec::new());
    if let Some(why) = fault {
        notes.push(format!(
            "{DIR}/{kind}/: {}",
            crate::gate::discover::unlisted_evidence(&why)
        ));
    }
    for name in names {
        let path = format!("{DIR}/{kind}/{name}");
        let file = dir.join(&name);
        let read = match std::fs::metadata(&file) {
            Ok(m) if m.len() > LIMIT => Err("larger than 4 MiB".to_owned()),
            Ok(_) => std::fs::read_to_string(&file).map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        };
        match read.and_then(|bytes| parse(&path, kind, &bytes)) {
            Ok(r) if r.checker == checker => found.push(r),
            Ok(_) => {}
            Err(why) => notes.push(format!("check receipt {path} skipped: {why}")),
        }
    }
    (found, notes)
}

/// The receipts of the checked kinds a run needs, indexed by token digest.
#[derive(Clone, Debug, Default)]
pub struct Receipts {
    pub by_kind: BTreeMap<String, Vec<Receipt>>,
    index: BTreeMap<(String, String), Vec<(usize, usize)>>,
    /// One per file skipped.
    pub notes: Vec<String>,
}

impl Receipts {
    /// Read the receipts of each checked kind in `wanted` (only when
    /// `.writfold/checks/` exists).
    pub fn load<'a>(root: &Path, kinds: &Kinds, wanted: impl Iterator<Item = &'a str>) -> Self {
        let mut out = Self::default();
        if !root.join(DIR).is_dir() {
            return out;
        }
        let wanted: std::collections::BTreeSet<&str> = wanted.collect();
        for kind in wanted {
            let Some(checker) = kinds.checker(kind) else {
                continue;
            };
            let (found, notes) = read_kind(root, kind, checker);
            out.notes.extend(notes);
            for (r, receipt) in found.iter().enumerate() {
                for (e, entry) in receipt.verdicts.iter().enumerate() {
                    let key = (kind.to_owned(), entry.token.clone());
                    out.index.entry(key).or_default().push((r, e));
                }
            }
            out.by_kind.insert(kind.to_owned(), found);
        }
        out
    }

    /// Every recorded answer for the token text `raw` of `kind`, in file
    /// order.
    pub fn lookup(&self, kind: &str, raw: &str) -> Vec<(&Receipt, &Entry)> {
        let Some(found) = self.by_kind.get(kind) else {
            return Vec::new();
        };
        let key = (kind.to_owned(), digest(raw));
        let hits = self.index.get(&key).map(Vec::as_slice).unwrap_or_default();
        hits.iter()
            .filter_map(|(r, e)| {
                let receipt = found.get(*r)?;
                Some((receipt, receipt.verdicts.get(*e)?))
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "receipt_tests.rs"]
mod tests;
