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
use crate::emit::{More, Row};
use crate::error::Result;
use crate::gate::statement;
use std::path::Path;
use writfold_core::gate::{Evidence, Standing, State, standing, state};

/// One obligation of one candidate.
pub struct Judged<'a> {
    pub obligation: Obligation,
    pub standing: Standing,
    /// The state from receipts alone (what a waiver sets aside).
    pub checked: State,
    pub receipt: Option<(&'a receipt::Filed, &'a Receipt)>,
    pub statements: statement::Summary<'a>,
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
            statements: statement::filed(root, &c.dir, run),
        }
    }
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
    valid.sort_by(|a, b| (&a.1.recorded, &a.0.name).cmp(&(&b.1.recorded, &b.0.name)));
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
            }
        })
        .collect()
}

fn short(digest: &str) -> &str {
    let d = digest.strip_prefix("sha256:").unwrap_or(digest);
    d.get(..12).unwrap_or(d)
}

/// The `check:` line of one obligation.
fn line(j: &Judged<'_>, id: &str) -> More {
    let ob = &j.obligation;
    let mut more = More {
        label: "check",
        ..More::default()
    };
    let said = match j.standing {
        Standing::Attested => j.statements.attested,
        Standing::Waived => j.statements.waived,
        Standing::Checked(_) => None,
    };
    let detail = match (said, j.receipt) {
        (Some((f, s)), _) => {
            more.path = Some(f.path.clone());
            more.line = Some(1);
            match j.standing {
                Standing::Waived if ob.class != "attested" => {
                    format!("by {}: {} (check: {})", s.by, s.text, j.checked.name())
                }
                _ => format!("by {}: {}", s.by, s.text),
            }
        }
        (None, Some((f, r))) => {
            more.path = Some(f.path.clone());
            more.line = Some(1);
            let found: Vec<String> = r
                .findings
                .iter()
                .take(3)
                .map(|x| format!("line {} {} ({})", x.line, x.rule, x.detail))
                .collect();
            let mut text = format!("{} for {}", r.outcome.name(), short(&r.content));
            if !found.is_empty() {
                text.push_str(&format!(": {}", found.join("; ")));
            }
            if r.total > 3 {
                text.push_str(&format!(" (+{} more)", r.total - 3));
            }
            if let Some(note) = r.note.as_ref().filter(|_| r.findings.is_empty()) {
                text.push_str(&format!(" ({note})"));
            }
            text
        }
        (None, None) if ob.class == "attested" => {
            format!(
                "needs: writfold kb deliver attest {id} --check {} --statement TEXT --by WHO",
                ob.name
            )
        }
        (None, None) => format!("needs: writfold kb deliver check {id}"),
    };
    more.text = format!("{} [{}] {} {detail}", ob.name, j.standing.name(), ob.class);
    if j.standing != Standing::Checked(State::Satisfied) {
        more.flags.push(j.standing.name().into());
    }
    more
}

/// Whether every obligation is met (satisfied, attested or waived).
pub fn ready(judged: &[Judged<'_>]) -> bool {
    judged.iter().all(|j| j.standing.met())
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
    if !ok {
        row.flags.push("blocked".into());
    }
    row.more = judged.iter().map(|j| line(j, &c.id)).collect();
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
