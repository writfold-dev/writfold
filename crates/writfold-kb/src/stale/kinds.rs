//! Kind declarations: `.writfold/sources/<kind>.toml` at the KB root, one
//! file per citation kind that is checked by time (every kind except
//! `code`, `gate`, `record` and `proposal`, which are checked against Git
//! and the index and cannot be declared).
//!
//! A declaration gives the kind's freshness rule (`ttl`; `immutable`:
//! fresh at any age; or `checked`: asked of an external checker named by
//! `checker`, see [`super::checker`]), a default lifetime for tokens
//! without `ttl=`, the fields every token of the kind must carry, and for
//! a checked kind the fields its checker's `make` may write and the oldest
//! cached verdict a reader may use (`max_age`). A kind without a file
//! behaves as before: fresh until `seen + ttl`, and fresh for ever when the
//! token has no `ttl=`. Files use the gate definitions' TOML subset
//! ([`crate::defs::toml`]); an unknown key, a wrong schema or a bad value
//! refuses every check that reads them, so a misspelt field never silently
//! changes a verdict. The choice of lifetime is the kernel's
//! ([`writfold_core::verdict::lifetime`]).
use super::{Checked, Cite, Verdict, unverifiable};
use crate::error::{KbError, Result};
use crate::time;
use std::collections::BTreeMap;
use std::path::Path;
use writfold_core::verdict::{self as kernel, Lifetime, Observed};

/// Where kind declarations live, under the KB root.
pub const DIR: &str = ".writfold/sources";
pub const SCHEMA: &str = "writfold-source/1";
/// Kinds checked against Git or the index, which a file may not declare.
pub const BUILT_IN: [&str; 4] = ["code", "gate", "record", "proposal"];
/// Fields Writfold writes itself, which a checker may never return.
pub const OWNED: [&str; 5] = ["kind", "seen", "ttl", "stale", "v"];

/// How a declared kind ages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    /// Fresh until `seen + ttl`.
    Ttl,
    /// Fresh at any age: what it cites never changes.
    Immutable,
    /// Asked of the external checker the declaration names.
    Checked,
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
    /// A checked kind's checker: `writfold-check-<checker>` on `PATH`.
    pub checker: Option<String>,
    /// The fields a checked kind's `make` may write.
    pub fields: Vec<String>,
    /// A checked kind's oldest usable cached verdict (`<N>d` or `<N>h`).
    pub max_age: Option<String>,
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

    /// Every declaration, by kind name.
    pub fn iter(&self) -> impl Iterator<Item = &Kind> {
        self.0.values()
    }

    /// The declared `max_age` of `kind` in seconds, when it has one.
    pub fn max_age(&self, kind: &str) -> Option<i64> {
        match time::ttl(self.get(kind)?.max_age.as_deref())? {
            time::Ttl::Seconds(seconds) => Some(seconds),
            time::Ttl::Never => None,
        }
    }

    /// The checker of `kind` when it is declared `checked`.
    pub fn checker(&self, kind: &str) -> Option<&str> {
        self.get(kind)
            .filter(|k| k.freshness == Freshness::Checked)
            .and_then(|k| k.checker.as_deref())
    }
}

pub(crate) fn is_word(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
        && !text.starts_with(['.', '-', '_'])
}

/// Read one declaration; `name` is its file name without `.toml`. The
/// first fault, compact, when it does not check ([`super::kind_check`]).
pub fn parse(text: &str, name: &str) -> std::result::Result<Kind, String> {
    let shown = format!("{DIR}/{name}.toml");
    super::kind_check::check(text, name, &shown).map_err(|r| crate::defs::why(&r))
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
        let kind = super::kind_check::check(&text, &name, &path)
            .map_err(|report| KbError::Definition(Box::new(report)))?;
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
    if let Some(missing) = missing(kinds, cite, kind) {
        return missing;
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
                recorded: None,
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
        recorded: None,
    }
}

/// Unverifiable when the token lacks a field its declaration requires.
pub fn missing(kinds: &Kinds, cite: &Cite, kind: &str) -> Option<Checked> {
    let declared = kinds.get(kind)?;
    let field = declared.required.iter().find(|f| cite.field(f).is_none())?;
    Some(unverifiable(format!(
        "missing {field}= (required by {DIR}/{kind}.toml)"
    )))
}

/// Offline: whether `seen` plus the effective lifetime has passed. A
/// checked kind has no lifetime: it is never due.
pub fn due(kinds: &Kinds, cite: &Cite, now: i64) -> bool {
    if cite
        .kind
        .as_deref()
        .and_then(|k| kinds.checker(k))
        .is_some()
    {
        return false;
    }
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

/// Undeclared kinds (not built in, no declaration file) holding tokens
/// without `ttl=`, which therefore never expire, with their token counts.
pub fn never_expiring(kinds: &Kinds, cites: &[Cite]) -> BTreeMap<String, usize> {
    let mut found = BTreeMap::new();
    for cite in cites
        .iter()
        .filter(|c| c.malformed.is_none() && c.ttl.is_none())
    {
        let Some(kind) = cite.kind.as_deref() else {
            continue;
        };
        if !BUILT_IN.contains(&kind) && kinds.get(kind).is_none() {
            *found.entry(kind.to_owned()).or_insert(0) += 1;
        }
    }
    found
}

/// The kernel verdicts a time-based check can reach.
fn from_kernel(verdict: kernel::Verdict) -> Verdict {
    match verdict {
        kernel::Verdict::Fresh => Verdict::Fresh,
        kernel::Verdict::Expired => Verdict::Expired,
        _ => Verdict::Unverifiable("unreadable seen= or ttl=".into()),
    }
}
