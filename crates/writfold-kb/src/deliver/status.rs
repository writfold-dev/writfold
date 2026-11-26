//! Each candidate's obligations, derived on every read (never stored) from
//! its mode, the audience's deny list, its check receipts and its
//! statements, with the same kernel as gates: a receipt counts only when
//! it is about the candidate's current bytes and the check's current
//! configuration ([`writfold_core::gate::state`]), and statements stand as
//! `attested` or `waived`, never as satisfied
//! ([`writfold_core::gate::standing`]). Receipts live in the candidate's
//! own directory, so a check of one candidate never counts for another.
use super::candidate::Candidate;
use super::config::{self, Obligation};
use super::receipt::{self, Receipt};
use super::status_line::{line, rejected, short};
use crate::emit::Row;
use crate::error::Result;
use crate::gate::statement;
use std::path::Path;
use writfold_core::delivery::Owed;
use writfold_core::gate::{Evidence, Standing, State, standing, state};

/// One obligation of one candidate.
pub struct Judged<'a> {
    pub obligation: Obligation,
    pub standing: Standing,
    /// The state from receipts alone (what a waiver sets aside).
    pub checked: State,
    pub receipt: Option<(&'a receipt::Filed, &'a Receipt)>,
    pub statements: statement::Summary<'a>,
    /// Citation tokens no longer fresh, of how many, when a passed
    /// `claims.cited-fresh` receipt was re-checked ([`super::recheck`]).
    pub moved: Option<(usize, usize)>,
    /// Why a passed `invocation.bound` receipt no longer holds
    /// ([`super::bound::recheck`]).
    pub unbound: Option<&'static str>,
}

/// A candidate's receipts and statements.
pub struct Filings {
    pub receipts: Vec<receipt::Filed>,
    pub statements: Vec<statement::Filed>,
}

impl Filings {
    pub fn read(root: &Path, c: &Candidate, run: &str) -> Self {
        Self {
            receipts: receipt::filed(root, &c.dir, run, &c.id),
            statements: statement::filed(root, &c.dir, run)
                .into_iter()
                .filter(|f| !withdrawal(root, f))
                .collect(),
        }
    }
}

/// A withdrawal beside the statements ([`super::withdraw`]): not one of them.
fn withdrawal(root: &Path, f: &statement::Filed) -> bool {
    f.name.starts_with(super::withdraw::PREFIX)
        && std::fs::read_to_string(root.join(&f.path))
            .is_ok_and(|t| super::withdraw::is_withdrawal(&t))
}

/// The candidate's obligations from its manifest's mode and audience.
pub fn obligations(root: &Path, c: &Candidate) -> Result<Vec<Obligation>> {
    let m = c
        .manifest
        .as_ref()
        .map_err(|why| crate::error::KbError::Refused(format!("candidate {}: {why}", c.id)))?;
    let mode = config::mode(root, &m.mode)?;
    let deny = config::deny(root, &m.audience)?;
    Ok(config::obligations(&mode, &deny))
}

/// Judge every obligation of `c` from its filings.
pub fn judge<'a>(obligations: Vec<Obligation>, c: &Candidate, f: &'a Filings) -> Vec<Judged<'a>> {
    let content = c.digest();
    let mut valid: Vec<(&receipt::Filed, &Receipt)> = f
        .receipts
        .iter()
        .filter_map(|x| x.parsed.as_ref().ok().map(|r| (x, r)))
        .collect();
    valid.sort_by(|a, b| receipt::order(*a, *b));
    obligations
        .into_iter()
        .map(|ob| {
            let mine: Vec<(&receipt::Filed, &Receipt)> = valid
                .iter()
                .filter(|(_, r)| r.check == ob.name)
                .copied()
                .collect();
            let evidence: Vec<Evidence> = mine
                .iter()
                .map(|(_, r)| Evidence {
                    outcome: r.outcome,
                    same_definition: r.definition == ob.digest,
                    same_scope: r.content == content,
                    bound: true,
                })
                .collect();
            let (checked, at) = state(&evidence);
            let statements =
                statement::summary(&f.statements, &ob.name, &c.id, &ob.digest, &content);
            let attested = ob.class == "attested";
            Judged {
                standing: standing(attested, checked, statements.statements),
                checked,
                receipt: at.and_then(|i| mine.get(i).copied()),
                statements,
                obligation: ob,
                moved: None,
                unbound: None,
            }
        })
        .collect()
}

/// What the kernel reads of each obligation: its standing and the
/// standings its policy takes.
pub fn owed(judged: &[Judged<'_>]) -> Vec<Owed> {
    judged.iter().map(Judged::owed).collect()
}

impl Judged<'_> {
    pub fn owed(&self) -> Owed {
        Owed {
            standing: self.standing,
            accept: self.obligation.accept(),
        }
    }

    /// Not met under its policy ([`writfold_core::delivery::unmet`]).
    pub fn unmet(&self) -> bool {
        writfold_core::delivery::unmet(&self.owed())
    }

    /// A met word (attested or waived) its policy does not take.
    pub fn rejected(&self) -> bool {
        self.standing.met() && self.unmet()
    }
}

/// Whether the candidate is ready: it has obligations and every one is met
/// under its policy; a mode that declares nothing never is
/// ([`writfold_core::delivery::ready`]).
pub fn ready(judged: &[Judged<'_>]) -> bool {
    writfold_core::delivery::ready(&owed(judged))
}

/// The candidate's row, with one `check:` line per obligation.
pub fn row(c: &Candidate, judged: &[Judged<'_>]) -> Row {
    let content = c.digest();
    let mut row = Row {
        kind: "delivery",
        id: c.id.clone(),
        path: Some(c.payload_path()),
        line: Some(1),
        section: Some("deliveries"),
        ..Row::default()
    };
    let bytes = c.bytes.as_ref().map_or(0, Vec::len);
    match &c.manifest {
        Ok(m) => {
            let to = m.destination.as_deref().unwrap_or("(unstated)");
            row.text = format!(
                "{} audience={} destination={to} candidate={} bytes={bytes}",
                m.mode,
                m.audience,
                short(&content)
            );
            if m.prepared != content {
                row.flags.push("edited".into());
            }
            row.extra = vec![
                ("mode", serde_json::json!(m.mode)),
                ("audience", serde_json::json!(m.audience)),
                ("destination", serde_json::json!(m.destination)),
                ("digest", serde_json::json!(content)),
                ("bytes", serde_json::json!(bytes)),
                ("seq", serde_json::json!(m.seq)),
            ];
        }
        Err(why) => {
            row.status = Some("invalid".into());
            row.text = why.clone();
            row.flags.push("invalid".into());
            return row;
        }
    }
    let ok = ready(judged);
    row.status = Some(if ok { "ready" } else { "blocked" }.into());
    row.extra.push(("ready", serde_json::json!(ok)));
    if !ok {
        row.flags.push("blocked".into());
    }
    if let (true, Ok(m)) = (judged.is_empty(), &c.manifest) {
        row.more.push(crate::emit::More {
            label: "why",
            text: super::step::empty(&m.mode),
            ..crate::emit::More::default()
        });
        return row;
    }
    // A gate that names instructions adds a `read:` line under its own.
    row.more = judged
        .iter()
        .flat_map(|j| {
            let read = j
                .obligation
                .gate
                .as_ref()
                .and_then(|g| crate::gate::status::read_line(g, ""));
            std::iter::once(line(j, &c.id))
                .chain(rejected(j))
                .chain(read)
        })
        .collect();
    row
}

/// Rows for receipts and statements that do not read as valid.
pub fn invalid(f: &Filings) -> Vec<Row> {
    let receipts = f
        .receipts
        .iter()
        .filter_map(|x| x.parsed.as_ref().err().map(|why| (&x.name, &x.path, why)));
    let statements = f
        .statements
        .iter()
        .filter_map(|x| x.parsed.as_ref().err().map(|why| (&x.name, &x.path, why)));
    receipts
        .map(|r| ("receipt", r))
        .chain(statements.map(|s| ("statement", s)))
        .map(|(kind, (name, path, why))| Row {
            kind,
            id: name.trim_end_matches(".json").to_owned(),
            status: Some("invalid".into()),
            text: why.clone(),
            path: Some(path.clone()),
            flags: vec!["invalid".into()],
            section: Some("deliveries"),
            ..Row::default()
        })
        .collect()
}
