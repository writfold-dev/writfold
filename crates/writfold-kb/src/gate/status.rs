//! Obligation state, derived on every read from the definitions and the
//! run's receipts (never stored): `open`, `satisfied`, `stale`, `failed`,
//! `unknown`. The comparison is the kernel's [`writfold_core::gate::state`];
//! this module gathers the facts and renders rows for `gate status`,
//! `gate lines` and `resume`.
use super::def::{self, Definition, Origin};
use super::receipt::{self, Receipt};
use super::subject::{Snapshot, Subject, snapshot};
use crate::emit::{More, Row};
use crate::error::Result;
use std::path::Path;
use writfold_core::gate::{Evidence, State, in_scope, state};
use writfold_runtime::git::GitExecutable;

/// One receipt file of a run.
pub struct Filed {
    /// KB-relative path.
    pub path: String,
    pub name: String,
    pub parsed: std::result::Result<Receipt, String>,
}

/// Every receipt file of the run whose directory is `run_dir`, by name.
pub fn filed(root: &Path, run_dir: &str, run: &str) -> Vec<Filed> {
    let dir = root.join(run_dir).join(receipt::DIR);
    let Ok(entries) = std::fs::read_dir(dir) else {
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
            let path = format!("{run_dir}/{}/{name}", receipt::DIR);
            let parsed = std::fs::read_to_string(root.join(&path))
                .map_err(|e| format!("unreadable ({e})"))
                .and_then(|text| receipt::parse(&text, run, &name));
            Filed { path, name, parsed }
        })
        .collect()
}

/// One gate's derived state.
pub struct Judged<'a> {
    pub definition: Definition,
    pub state: State,
    /// The receipt that decides the state, if any.
    pub receipt: Option<(&'a Filed, &'a Receipt)>,
    pub same_definition: bool,
}

/// The whole picture for one run and subject.
pub struct Report<'a> {
    pub gates: Vec<Judged<'a>>,
    pub problems: Vec<(Origin, String, String)>,
    pub snapshot: Snapshot,
}

/// Judge every defined gate on `subject` from the receipts `filed`.
pub fn judge<'a>(
    git: &GitExecutable<'_>,
    root: &Path,
    subject: &Subject,
    filed: &'a [Filed],
) -> Result<Report<'a>> {
    let (definitions, problems) = def::all(root, Some(&subject.top));
    let patterns: Vec<Vec<&str>> = definitions.iter().map(Definition::patterns).collect();
    let wanted = |p: &str| patterns.iter().any(|scope| in_scope(scope, p));
    let snapshot = snapshot(git, subject, &wanted)?;
    let mut valid: Vec<(&Filed, &Receipt)> = filed
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
        gates.push(Judged {
            same_definition: receipt.is_none_or(|(_, r)| r.definition == definition.digest),
            definition,
            state,
            receipt,
        });
    }
    Ok(Report {
        gates,
        problems,
        snapshot,
    })
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

/// The rows: one per gate (with why and the next command when it is not
/// satisfied), one per invalid receipt, one per unusable definition.
pub fn rows(
    git: &GitExecutable<'_>,
    subject: &Subject,
    report: &Report<'_>,
    filed: &[Filed],
) -> Result<Vec<Row>> {
    let mut out = Vec::new();
    for judged in &report.gates {
        out.push(gate_row(git, subject, report, judged)?);
    }
    for f in filed {
        if let Err(why) = &f.parsed {
            out.push(Row {
                kind: "receipt",
                id: f.name.trim_end_matches(".json").to_owned(),
                status: Some("invalid".into()),
                text: why.clone(),
                path: Some(f.path.clone()),
                flags: vec!["invalid".into()],
                section: Some("gates"),
                ..Row::default()
            });
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
        status: Some(judged.state.name().into()),
        section: Some("gates"),
        ..Row::default()
    };
    match judged.receipt {
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
    if judged.state == State::Satisfied {
        return Ok(row);
    }
    row.flags.push(judged.state.name().into());
    if let Some((_, r)) = judged.receipt
        && judged.state == State::Stale
    {
        if !judged.same_definition {
            row.more.push(line(
                "why",
                "the definition changed since this receipt".into(),
            ));
        } else {
            let patterns = d.patterns();
            let why = match report
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
            };
            row.more.push(line("why", why));
        }
    }
    row.more
        .push(line("next", format!("writfold kb gate run {}", d.id)));
    Ok(row)
}

/// Counts by state for the trailer, in a fixed order; invalid receipts and
/// definitions are counted together.
pub fn counts(report: &Report<'_>, filed: &[Filed]) -> Vec<(&'static str, usize)> {
    let count = |s: State| report.gates.iter().filter(|j| j.state == s).count();
    let invalid = filed.iter().filter(|f| f.parsed.is_err()).count() + report.problems.len();
    vec![
        ("gates", report.gates.len()),
        ("satisfied", count(State::Satisfied)),
        ("open", count(State::Open)),
        ("stale", count(State::Stale)),
        ("failed", count(State::Failed)),
        ("unknown", count(State::Unknown)),
        ("invalid", invalid),
    ]
}
