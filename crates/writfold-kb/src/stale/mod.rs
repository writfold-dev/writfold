//! Citation freshness. Code citations are checked against a local clone of
//! the cited repository at `origin/HEAD` (else `HEAD`); v1 never fetches.
//! Record and decision citations compare the cited state with the index;
//! every other kind is fresh until `seen + ttl`.
mod code;
pub mod repos;
pub mod span;

use crate::error::Result;
use crate::session::Session;
use crate::time;

/// One stored citation, as the index holds it.
#[derive(Clone, Debug, Default)]
pub struct Cite {
    pub path: String,
    pub ord: u32,
    pub line: u32,
    pub kind: Option<String>,
    pub repo: Option<String>,
    pub rev: Option<String>,
    pub cpath: Option<String>,
    pub first: Option<u32>,
    pub last: Option<u32>,
    pub blob: Option<String>,
    pub span: Option<String>,
    pub seen: Option<String>,
    pub ttl: Option<String>,
    pub state: Option<String>,
    pub marked: Option<String>,
    pub malformed: Option<String>,
    pub fields: String,
    pub raw: String,
}

impl Cite {
    /// A token read from text that is not indexed (a draft, a Weft
    /// citation), filed under `path` at position `ord`.
    pub fn from_citation(path: &str, ord: usize, c: crate::model::Citation) -> Self {
        Self {
            path: path.to_owned(),
            ord: u32::try_from(ord).unwrap_or(u32::MAX),
            line: c.line,
            kind: c.kind,
            repo: c.repo,
            rev: c.rev,
            cpath: c.path,
            first: c.first,
            last: c.last,
            blob: c.blob,
            span: c.span,
            seen: c.seen,
            ttl: c.ttl,
            state: c.state,
            marked: c.marked,
            malformed: c.malformed,
            fields: c.fields,
            raw: c.raw,
        }
    }

    pub fn field(&self, key: &str) -> Option<String> {
        let value: serde_json::Value = serde_json::from_str(&self.fields).ok()?;
        value.get(key)?.as_str().map(str::to_owned)
    }
}

/// Exactly one verdict per citation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Verdict {
    Fresh,
    Moved {
        path: Option<String>,
        first: u32,
        last: u32,
    },
    Changed {
        detail: Option<String>,
    },
    Gone {
        detail: Option<String>,
    },
    Expired,
    Unverifiable(String),
    /// An authored `stale=` mark, passed through.
    Marked(String),
    /// Past `seen + ttl` (offline mode only).
    Due,
}

impl Verdict {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Moved { .. } => "moved",
            Self::Changed { .. } => "changed",
            Self::Gone { .. } => "gone",
            Self::Expired => "expired",
            Self::Unverifiable(_) => "unverifiable",
            Self::Marked(_) => "marked",
            Self::Due => "due",
        }
    }
}

/// A verdict and the repository head it was computed at (or `seen` for
/// time-based kinds).
#[derive(Clone, Debug)]
pub struct Checked {
    pub verdict: Verdict,
    pub head: String,
}

/// Classify every citation. `offline` reads no repository and reports only
/// authored marks and due TTLs.
pub fn check(session: &Session<'_>, cites: &[Cite], offline: bool) -> Result<Vec<Checked>> {
    let mut out: Vec<Option<Checked>> = vec![None; cites.len()];
    let mut code = Vec::new();
    for (index, cite) in cites.iter().enumerate() {
        if let Some(reason) = &cite.malformed {
            out[index] = Some(unverifiable(format!("malformed: {reason}")));
        } else if offline {
            out[index] = Some(offline_verdict(cite, session.now));
        } else {
            match cite.kind.as_deref() {
                Some("code" | "gate") => code.push(index),
                _ => out[index] = Some(other(session, cite)?),
            }
        }
    }
    for (index, checked) in code::check(session, cites, &code)? {
        out[index] = Some(checked);
    }
    Ok(cites
        .iter()
        .zip(out)
        .map(|(cite, checked)| {
            let checked = checked.unwrap_or_else(|| unverifiable("not checked".into()));
            marked(cite, checked)
        })
        .collect())
}

fn unverifiable(reason: String) -> Checked {
    Checked {
        verdict: Verdict::Unverifiable(reason),
        head: "none".into(),
    }
}

/// An authored mark wins over every verdict except changed, gone and
/// expired, which say more (decided by the kernel in `writfold-core`).
fn marked(cite: &Cite, checked: Checked) -> Checked {
    let Some(mark) = &cite.marked else {
        return checked;
    };
    match writfold_core::verdict::with_mark(kernel(&checked.verdict), true) {
        writfold_core::verdict::Verdict::Marked => Checked {
            verdict: Verdict::Marked(mark.clone()),
            head: checked.head,
        },
        _ => checked,
    }
}

/// The kernel's name for a verdict.
fn kernel(verdict: &Verdict) -> writfold_core::verdict::Verdict {
    use writfold_core::verdict::Verdict as K;
    match verdict {
        Verdict::Fresh => K::Fresh,
        Verdict::Moved { path: Some(_), .. } => K::Renamed,
        Verdict::Moved { .. } => K::Moved,
        Verdict::Changed { .. } => K::Changed,
        Verdict::Gone { .. } => K::Gone,
        Verdict::Expired => K::Expired,
        Verdict::Unverifiable(_) => K::Unverifiable,
        Verdict::Marked(_) => K::Marked,
        Verdict::Due => K::Due,
    }
}

fn offline_verdict(cite: &Cite, now: i64) -> Checked {
    let due = cite
        .seen
        .as_deref()
        .and_then(|seen| time::expired(seen, cite.ttl.as_deref(), now))
        == Some(true);
    let verdict = if due { Verdict::Due } else { Verdict::Fresh };
    Checked {
        verdict,
        head: "offline".into(),
    }
}

/// Record, decision and time-based citations.
fn other(session: &Session<'_>, cite: &Cite) -> Result<Checked> {
    let head = session.pin.label();
    let state_of = |id: Option<String>, by_path: bool| -> Result<Checked> {
        let Some(id) = id else {
            return Ok(unverifiable(format!(
                "missing {}=",
                if by_path { "path" } else { "id" }
            )));
        };
        let column = if by_path { "path" } else { "id" };
        let sql = format!("SELECT status FROM records WHERE {column} = ?1 ORDER BY id LIMIT 1");
        let id = if by_path {
            id
        } else {
            format!("proposal/{id}")
        };
        let found: Option<Option<String>> = rusqlite::OptionalExtension::optional(
            session.conn.query_row(&sql, [id], |r| r.get(0)),
        )?;
        let verdict = match (found, &cite.state) {
            (None, _) => Verdict::Gone { detail: None },
            (Some(None), _) => Verdict::Unverifiable("record has no status".into()),
            (Some(Some(now)), Some(cited)) if &now != cited => Verdict::Changed {
                detail: Some(format!("state {cited} → {now}")),
            },
            _ => Verdict::Fresh,
        };
        Ok(Checked {
            verdict,
            head: head.clone(),
        })
    };
    match cite.kind.as_deref() {
        None => Ok(unverifiable("missing kind=".into())),
        Some("record") => state_of(cite.cpath.clone(), true),
        Some("proposal") => state_of(cite.field("id"), false),
        Some(_) => {
            let Some(seen) = &cite.seen else {
                return Ok(unverifiable("missing seen=".into()));
            };
            let verdict = match time::expired(seen, cite.ttl.as_deref(), session.now) {
                Some(true) => Verdict::Expired,
                Some(false) => Verdict::Fresh,
                None => Verdict::Unverifiable("unreadable seen= or ttl=".into()),
            };
            Ok(Checked {
                verdict,
                head: format!("seen:{seen}"),
            })
        }
    }
}
