//! Obligation state, derived on every read from the definitions, the run's
//! receipts and its statements (never stored): `open`, `satisfied`,
//! `stale`, `failed`, `unknown`, and the two statement standings
//! `attested` and `waived`. The comparisons are the kernel's
//! [`writfold_core::gate::state`] and [`writfold_core::gate::standing`];
//! this module gathers the facts and renders rows for `gate status`,
//! `gate lines` and `resume`.
use super::def::{self, Definition, Origin};
use super::receipt::{self, Receipt};
use super::statement;
use super::subject::{Snapshot, Subject, snapshot};
use crate::emit::{More, Row};
use crate::error::{KbError, Result};
use crate::session::Session;
use std::path::Path;
use writfold_core::gate::{Evidence, Standing, State, in_scope, standing, state};
use writfold_runtime::git::GitExecutable;

pub use super::receipt::{Filed, filed};

/// A run's receipts, statements and assessed receipts.
pub struct Filings {
    pub receipts: Vec<Filed>,
    pub statements: Vec<statement::Filed>,
    pub assessed: Vec<super::assessed::Filed>,
    /// `runs/<run>`.
    pub run_dir: String,
}

impl Filings {
    pub fn read(root: &Path, run_dir: &str, run: &str) -> Self {
        Self {
            receipts: filed(root, run_dir, run),
            statements: statement::filed(root, run_dir, run),
            assessed: super::assessed::filed(root, run_dir, run),
            run_dir: run_dir.to_owned(),
        }
    }

    /// Receipts and statements that do not read as valid.
    pub fn invalid(&self) -> usize {
        self.receipts.iter().filter(|f| f.parsed.is_err()).count()
            + self.statements.iter().filter(|f| f.parsed.is_err()).count()
            + self.assessed.iter().filter(|f| f.parsed.is_err()).count()
    }
}

/// One gate's derived state.
pub struct Judged<'a> {
    pub definition: Definition,
    /// The state from receipts alone.
    pub state: State,
    /// What stands once statements are counted.
    pub standing: Standing,
    /// The receipt that decides the state, if any.
    pub receipt: Option<(&'a Filed, &'a Receipt)>,
    pub same_definition: bool,
    pub statements: statement::Summary<'a>,
    /// An assessed gate's receipt and checks (its `receipt` is `None`).
    pub assessment: Option<super::assessed::Assessment<'a>>,
}

/// The whole picture for one run and subject.
pub struct Report<'a> {
    pub gates: Vec<Judged<'a>>,
    pub problems: Vec<(Origin, String, String)>,
    pub snapshot: Snapshot,
}

impl Report<'_> {
    /// Whether every gate is met (satisfied, attested or waived).
    pub fn all_met(&self) -> bool {
        !self.gates.is_empty() && self.gates.iter().all(|j| j.standing.met())
    }
}

/// Judge every defined gate on `subject` from the run's filings. Assessed
/// gates are judged last, against the others ([`super::assessed::judge`]).
pub fn judge<'a>(
    session: &Session<'_>,
    subject: &Subject,
    filings: &'a Filings,
) -> Result<Report<'a>> {
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("gates need Git".into()))?;
    let (mut definitions, problems) = def::all(&session.ws.root, Some(&subject.top));
    // Candidate gates judge delivery drafts, never a repository: a run
    // owes them only through a delivery mode (`deliver status`).
    definitions.retain(|d| !d.on_candidate());
    let assessed: Vec<Definition>;
    (assessed, definitions) = definitions.into_iter().partition(|d| d.assessed.is_some());
    let patterns: Vec<Vec<&str>> = definitions.iter().map(Definition::patterns).collect();
    let wanted = |p: &str| patterns.iter().any(|scope| in_scope(scope, p));
    let snapshot = snapshot(git, subject, &wanted)?;
    let mut valid: Vec<(&Filed, &Receipt)> = filings
        .receipts
        .iter()
        .filter_map(|f| f.parsed.as_ref().ok().map(|r| (f, r)))
        .filter(|(_, r)| r.repo == subject.name)
        .collect();
    valid.sort_by(|a, b| (&a.1.recorded, &a.0.name).cmp(&(&b.1.recorded, &b.0.name)));
    let mut gates = Vec::new();
    for definition in definitions {
        let scoped = snapshot.scoped(&definition.patterns());
        let mine: Vec<(&Filed, &Receipt)> = valid
            .iter()
            .filter(|(_, r)| r.gate == definition.id)
            .copied()
            .collect();
        let evidence: Vec<Evidence> = mine
            .iter()
            .map(|(_, r)| Evidence {
                outcome: r.outcome,
                same_definition: r.definition == definition.digest,
                same_scope: r.scope == scoped.digest,
            })
            .collect();
        let (state, at) = state(&evidence);
        let receipt = at.and_then(|i| mine.get(i).copied());
        let statements = statement::summary(
            &filings.statements,
            &definition.id,
            &subject.name,
            &definition.digest,
            &scoped.digest,
        );
        gates.push(Judged {
            same_definition: receipt.is_none_or(|(_, r)| r.definition == definition.digest),
            standing: standing(definition.attested(), state, statements.statements),
            definition,
            state,
            receipt,
            statements,
            assessment: None,
        });
    }
    let mut report = Report {
        gates,
        problems,
        snapshot,
    };
    let more = super::assessed::judge(session, subject, filings, &report, assessed)?;
    report.gates.extend(more);
    Ok(report)
}

fn short(text: &str) -> &str {
    let text = text.strip_prefix("sha256:").unwrap_or(text);
    text.get(..12).unwrap_or(text)
}

fn line(label: &'static str, text: String) -> More {
    More {
        label,
        text,
        ..More::default()
    }
}

/// Where a definition is shown: its KB path, or `<repo>:<path>`.
pub fn shown_path(d: &Definition, subject: &Subject) -> String {
    match d.origin {
        Origin::Kb => d.path.clone(),
        Origin::Repo => format!("{}:{}", subject.name, d.path),
    }
}

/// The `read:` line of a gate that names instructions: the file the
/// agent reads (on an attested gate, what is being attested to).
/// A repository definition's path is shown as `<repo>:<path>`.
pub fn read_line(d: &Definition, repo: &str) -> Option<More> {
    let path = d.instructions_path()?;
    Some(More {
        label: "read",
        text: format!("instructions for {}", d.id),
        path: Some(match d.origin {
            Origin::Kb => path,
            Origin::Repo => format!("{repo}:{path}"),
        }),
        line: Some(1),
        ..More::default()
    })
}

fn invalid_row(kind: &'static str, name: &str, path: &str, why: &str) -> Row {
    Row {
        kind,
        id: name.trim_end_matches(".json").to_owned(),
        status: Some("invalid".into()),
        text: why.to_owned(),
        path: Some(path.to_owned()),
        flags: vec!["invalid".into()],
        section: Some("gates"),
        ..Row::default()
    }
}

/// The rows: one per gate (with why and the next command when it is not
/// met), one per invalid receipt or statement, one per unusable
/// definition.
pub fn rows(
    git: &GitExecutable<'_>,
    subject: &Subject,
    report: &Report<'_>,
    filings: &Filings,
) -> Result<Vec<Row>> {
    let mut out = Vec::new();
    for judged in &report.gates {
        out.push(match &judged.assessment {
            Some(assessment) => super::assessed::row(subject, judged, assessment),
            None => gate_row(git, subject, report, judged)?,
        });
    }
    for f in &filings.receipts {
        if let Err(why) = &f.parsed {
            out.push(invalid_row("receipt", &f.name, &f.path, why));
        }
    }
    for f in &filings.statements {
        if let Err(why) = &f.parsed {
            out.push(invalid_row("statement", &f.name, &f.path, why));
        }
    }
    for f in &filings.assessed {
        if let Err(why) = &f.parsed {
            out.push(invalid_row("receipt", &f.name, &f.path, why));
        }
    }
    for (origin, path, why) in &report.problems {
        out.push(Row {
            kind: "gate-definition",
            id: path.clone(),
            status: Some("invalid".into()),
            text: format!("{} definition: {why}", origin.name()),
            flags: vec!["invalid".into()],
            section: Some("gates"),
            ..Row::default()
        });
    }
    Ok(out)
}

fn gate_row(
    git: &GitExecutable<'_>,
    subject: &Subject,
    report: &Report<'_>,
    judged: &Judged<'_>,
) -> Result<Row> {
    let d = &judged.definition;
    let mut row = Row {
        kind: "gate",
        id: d.id.clone(),
        status: Some(judged.standing.name().into()),
        section: Some("gates"),
        ..Row::default()
    };
    match judged.receipt {
        None if d.attested() => {
            row.text = format!(
                "attested: statements only, no command, for {}",
                subject.name
            );
            row.path = Some(shown_path(d, subject));
            row.line = Some(1);
        }
        None => {
            row.text = format!("{} no receipt in this run for {}", d.class, subject.name);
            row.path = Some(shown_path(d, subject));
            row.line = Some(1);
        }
        Some((f, r)) => {
            let dirty = if r.dirty > 0 {
                format!("+dirty:{}", r.dirty)
            } else {
                String::new()
            };
            row.text = format!(
                "{} {} {} at {}@{}{dirty} scope={} paths={}",
                r.class,
                r.outcome.name(),
                receipt::ended(r),
                r.repo,
                short(&r.commit),
                short(&r.scope),
                r.paths
            );
            row.path = Some(f.path.clone());
            row.line = Some(1);
        }
    }
    if let Some(more) = read_line(d, &subject.name) {
        row.more.push(more);
    }
    let shown = match judged.standing {
        Standing::Attested => judged.statements.attested,
        Standing::Waived => judged.statements.waived,
        Standing::Checked(_) => None,
    };
    if let Some((f, s)) = shown {
        row.more.push(statement::more(f, s));
    }
    if judged.standing == Standing::Checked(State::Satisfied) {
        return Ok(row);
    }
    row.flags.push(judged.standing.name().into());
    if judged.standing.met() {
        return Ok(row);
    }
    if judged.state == State::Stale
        && let Some((_, r)) = judged.receipt
    {
        let why = stale_why(git, subject, report, judged, r)?;
        row.more.push(line("why", why));
    }
    if judged.statements.waiver_moved {
        row.more.push(line(
            "why",
            "a waiver exists, but for other content or another definition".into(),
        ));
    }
    let next = match d.attested() {
        true => format!("writfold kb gate attest {} --statement TEXT --by WHO", d.id),
        false => format!("writfold kb gate run {}", d.id),
    };
    row.more.push(line("next", next));
    Ok(row)
}

/// Why a stale receipt is stale: the definition changed, or which
/// in-scope paths differ.
fn stale_why(
    git: &GitExecutable<'_>,
    subject: &Subject,
    report: &Report<'_>,
    judged: &Judged<'_>,
    r: &Receipt,
) -> Result<String> {
    if !judged.same_definition {
        return Ok("the definition changed since this receipt".into());
    }
    let patterns = judged.definition.patterns();
    Ok(
        match report
            .snapshot
            .changed_since(git, subject, &r.commit, &patterns)?
        {
            None => format!(
                "in-scope paths changed; commit {} is not in this clone",
                short(&r.commit)
            ),
            Some(paths) if paths.is_empty() => "in-scope content differs".into(),
            Some(paths) => {
                let first: Vec<&str> = paths.iter().take(5).map(String::as_str).collect();
                let more = paths.len().saturating_sub(5);
                let tail = if more > 0 {
                    format!(" (+{more} more)")
                } else {
                    String::new()
                };
                format!("changed in scope: {}{tail}", first.join(", "))
            }
        },
    )
}

/// Counts by standing for the trailer, in a fixed order; invalid
/// receipts, statements and definitions are counted together.
pub fn counts(report: &Report<'_>, filings: &Filings) -> Vec<(&'static str, usize)> {
    let count = |s: Standing| report.gates.iter().filter(|j| j.standing == s).count();
    vec![
        ("gates", report.gates.len()),
        ("satisfied", count(Standing::Checked(State::Satisfied))),
        ("open", count(Standing::Checked(State::Open))),
        ("stale", count(Standing::Checked(State::Stale))),
        ("failed", count(Standing::Checked(State::Failed))),
        ("unknown", count(Standing::Checked(State::Unknown))),
        ("attested", count(Standing::Attested)),
        ("waived", count(Standing::Waived)),
        ("invalid", filings.invalid() + report.problems.len()),
    ]
}
