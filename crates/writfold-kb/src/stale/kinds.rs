//! Kind declarations: `.writfold/sources/<kind>.toml` at the KB root, one
//! file per citation kind that is checked by time (every kind except
//! `code`, `gate`, `record` and `proposal`, which are checked against Git
//! and the index and cannot be declared).
//!
//! A declaration gives the kind's freshness rule (`ttl`, or `immutable`:
//! fresh at any age), a default lifetime for tokens without `ttl=`, and
//! the fields every token of the kind must carry. A kind without a file
//! behaves as before: fresh until `seen + ttl`, and fresh for ever when the
//! token has no `ttl=`. Files use the gate definitions' TOML subset
//! ([`crate::gate::toml`]); an unknown key, a wrong schema or a bad value
//! refuses every check that reads them, so a misspelt field never silently
//! changes a verdict. The choice of lifetime is the kernel's
//! ([`writfold_core::verdict::lifetime`]).
use super::{Checked, Cite, Verdict, unverifiable};
use crate::error::{KbError, Result};
use crate::gate::toml::{self, Value};
use crate::time;
use std::collections::BTreeMap;
use std::path::Path;
use writfold_core::verdict::{self as kernel, Lifetime, Observed};

/// Where kind declarations live, under the KB root.
pub const DIR: &str = ".writfold/sources";
pub const SCHEMA: &str = "writfold-source/1";
/// Kinds checked against Git or the index, which a file may not declare.
pub const BUILT_IN: [&str; 4] = ["code", "gate", "record", "proposal"];
const KEYS: [&str; 6] = [
    "schema",
    "kind",
    "freshness",
    "default_ttl",
    "required",
    "description",
];

/// How a declared kind ages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    /// Fresh until `seen + ttl`.
    Ttl,
    /// Fresh at any age: what it cites never changes.
    Immutable,
}

/// One declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Kind {
    pub name: String,
    pub freshness: Freshness,
    /// The lifetime of a token without `ttl=` (`<N>d` or `<N>h`).
    pub default_ttl: Option<String>,
    /// Fields a token of this kind must carry.
    pub required: Vec<String>,
    pub description: Option<String>,
}

/// Every declaration of a KB, by kind.
#[derive(Clone, Debug, Default)]
pub struct Kinds(BTreeMap<String, Kind>);

impl Kinds {
    pub fn new(kinds: impl IntoIterator<Item = Kind>) -> Self {
        Self(kinds.into_iter().map(|k| (k.name.clone(), k)).collect())
    }

    pub fn get(&self, kind: &str) -> Option<&Kind> {
        self.0.get(kind)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn is_word(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
        && !text.starts_with(['.', '-', '_'])
}

fn string(map: &BTreeMap<String, Value>, key: &str) -> std::result::Result<Option<String>, String> {
    match map.get(key) {
        None => Ok(None),
        Some(Value::Str(text)) => Ok(Some(text.clone())),
        Some(_) => Err(format!("{key} must be a string")),
    }
}

/// Read one declaration; `name` is its file name without `.toml`.
pub fn parse(text: &str, name: &str) -> std::result::Result<Kind, String> {
    let map = toml::parse(text)?;
    if let Some(key) = map.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(format!("unknown key {key}"));
    }
    if string(&map, "schema")?.as_deref() != Some(SCHEMA) {
        return Err(format!("schema must be {SCHEMA}"));
    }
    let kind = string(&map, "kind")?.ok_or("kind is required")?;
    if kind != name {
        return Err(format!(
            "kind {kind:?} must match the file name {name}.toml"
        ));
    }
    if !is_word(&kind) {
        return Err(format!("kind {kind:?} must be [a-z0-9._-]"));
    }
    if BUILT_IN.contains(&kind.as_str()) {
        return Err(format!(
            "kind {kind} is checked against Git and the index and cannot be declared"
        ));
    }
    let freshness = match string(&map, "freshness")?.as_deref() {
        Some("ttl") => Freshness::Ttl,
        Some("immutable") => Freshness::Immutable,
        _ => return Err("freshness must be \"ttl\" or \"immutable\"".into()),
    };
    let default_ttl = string(&map, "default_ttl")?;
    if let Some(ttl) = &default_ttl {
        if freshness == Freshness::Immutable {
            return Err("default_ttl cannot be given with freshness = \"immutable\"".into());
        }
        if !matches!(time::ttl(Some(ttl)), Some(time::Ttl::Seconds(_))) {
            return Err(format!("default_ttl {ttl:?} must be <N>d or <N>h"));
        }
    }
    let required = match map.get("required") {
        None => Vec::new(),
        Some(Value::List(fields)) => fields.clone(),
        Some(_) => return Err("required must be an array of strings".into()),
    };
    if let Some(field) = required.iter().find(|f| !is_word(f) && f.as_str() != "L") {
        return Err(format!("required field {field:?} is not a field name"));
    }
    Ok(Kind {
        name: kind,
        freshness,
        default_ttl,
        required,
        description: string(&map, "description")?,
    })
}

/// Every declaration under the KB root, or a refusal naming the first file
/// that cannot be read. No directory means no declarations.
pub fn read(root: &Path) -> Result<Kinds> {
    let entries = match std::fs::read_dir(root.join(DIR)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Kinds::default()),
        Err(e) => return Err(e.into()),
    };
    let mut names = Vec::new();
    for entry in entries {
        let file = entry?.file_name().to_string_lossy().into_owned();
        if let Some(name) = file.strip_suffix(".toml") {
            names.push(name.to_owned());
        }
    }
    names.sort();
    let mut kinds = BTreeMap::new();
    for name in names {
        let path = format!("{DIR}/{name}.toml");
        let text = std::fs::read_to_string(root.join(&path))
            .map_err(|e| KbError::Refused(format!("{path}: {e}")))?;
        let kind = parse(&text, &name).map_err(|why| KbError::Refused(format!("{path}: {why}")))?;
        kinds.insert(name, kind);
    }
    Ok(Kinds(kinds))
}

/// The lifetime a token of `kind` is judged by.
fn lifetime<'a>(kinds: &'a Kinds, cite: &'a Cite, kind: &str) -> Lifetime<&'a str> {
    let declared = kinds.get(kind);
    kernel::lifetime(
        declared.is_some_and(|d| d.freshness == Freshness::Immutable),
        cite.ttl.as_deref(),
        declared.and_then(|d| d.default_ttl.as_deref()),
    )
}

/// The verdict of a time-based citation of `kind` (not a built-in kind).
pub fn judge(kinds: &Kinds, cite: &Cite, kind: &str, now: i64) -> Checked {
    if let Some(declared) = kinds.get(kind)
        && let Some(field) = declared.required.iter().find(|f| cite.field(f).is_none())
    {
        return unverifiable(format!("missing {field}= (required by {DIR}/{kind}.toml)"));
    }
    let ttl = match lifetime(kinds, cite, kind) {
        Lifetime::Immutable => {
            let head = cite
                .seen
                .as_ref()
                .map_or_else(|| "immutable".into(), |seen| format!("seen:{seen}"));
            return Checked {
                verdict: from_kernel(kernel::observed(Observed::Immutable)),
                head,
            };
        }
        Lifetime::Token(ttl) | Lifetime::Default(ttl) => Some(ttl),
        Lifetime::Never => None,
    };
    let Some(seen) = &cite.seen else {
        return unverifiable("missing seen=".into());
    };
    let fact = Observed::Timed(time::expired(seen, ttl, now));
    Checked {
        verdict: from_kernel(kernel::observed(fact)),
        head: format!("seen:{seen}"),
    }
}

/// Offline: whether `seen` plus the effective lifetime has passed.
pub fn due(kinds: &Kinds, cite: &Cite, now: i64) -> bool {
    let ttl = match cite.kind.as_deref().map(|kind| lifetime(kinds, cite, kind)) {
        Some(Lifetime::Immutable) => return false,
        Some(Lifetime::Token(ttl) | Lifetime::Default(ttl)) => Some(ttl),
        Some(Lifetime::Never) | None => cite.ttl.as_deref(),
    };
    cite.seen
        .as_deref()
        .and_then(|seen| time::expired(seen, ttl, now))
        == Some(true)
}

/// The kernel verdicts a time-based check can reach.
fn from_kernel(verdict: kernel::Verdict) -> Verdict {
    match verdict {
        kernel::Verdict::Fresh => Verdict::Fresh,
        kernel::Verdict::Expired => Verdict::Expired,
        _ => Verdict::Unverifiable("unreadable seen= or ttl=".into()),
    }
}
