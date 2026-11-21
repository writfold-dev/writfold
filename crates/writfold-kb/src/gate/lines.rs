//! `gate lines`: plain lines to paste into a commit message. Without a
//! plan it exits 0 only when every gate is met: satisfied, or attested or
//! waived (each named as such, with its statement's file). With a plan it
//! agrees with the plan's word: the header ends `; plan <word>`, a gate
//! only skipped steps name is listed `skipped` and never counts, and it
//! exits 0 only as [`writfold_core::policy::lines::lines_clean`] decides.
use super::status::{self, Filings};
use super::{checklist, locate, receipt};
use crate::emit::Output;
use crate::error::{KbError, Result};
use crate::index::query::Rec;
use crate::session::Session;
use std::path::Path;
use writfold_core::gate::{Standing, State};
use writfold_core::policy::lines::{Listed, lines_clean};

pub(super) fn lines(
    session: &Session<'_>,
    rec: &Rec,
    repo: Option<&Path>,
    cwd: &Path,
) -> Result<Output> {
    let Some(subject) = locate(session, rec, repo, cwd)? else {
        return Err(KbError::Refused(
            "no subject repository: run inside one, or pass --repo DIR".into(),
        ));
    };
    let filings = Filings::for_run(session, rec);
    let report = status::judge(session, &subject, &filings)?;
    let plan = crate::plan::lines::of_run(session, rec, repo, cwd);
    let skipped =
        |j: &status::Judged<'_>| plan.as_ref().is_some_and(|p| p.skipped(&j.definition.id));
    let mut text = header(&report, &skipped, plan.as_ref().map(|p| p.word.as_str()));
    let head = report.snapshot.commit.as_str();
    text = format!(
        "Gates for {}@{} (writfold kb gate, run {}): {text}\n",
        subject.name,
        head.get(..12).unwrap_or(head),
        rec.id,
    );
    let width = (report.gates.iter())
        .map(|j| j.definition.id.len())
        .max()
        .unwrap_or(0);
    for judged in &report.gates {
        text.push_str(&match skipped(judged) {
            true => format!(
                "{:<w$} skipped\n",
                format!("{}:", judged.definition.id),
                w = width + 1
            ),
            false => gate_line(judged, width),
        });
    }
    let invalid = filings.invalid() + report.problems.len();
    if invalid > 0 {
        text.push_str(&format!(
            "invalid receipts or definitions: {invalid} (see writfold kb gate status)\n"
        ));
    }
    if let Some(note) = report.recorded_note() {
        text.push_str(&format!("note: {note}\n"));
    }
    let clean = match &plan {
        None => report.all_met() && invalid == 0,
        Some(plan) => {
            let listed: Vec<Listed> = (report.gates.iter())
                .map(|j| Listed {
                    skipped: skipped(j),
                    met: j.met(),
                })
                .collect();
            lines_clean(&plan.summary, &listed, invalid == 0)
        }
    };
    Ok(Output {
        text,
        exit: if clean { 0 } else { 2 },
        report: None,
        shown: None,
        drawn: None,
    })
}

/// The header's tally: `1 of 2 satisfied, 1 attested, 1 skipped`, then
/// `; plan <word>` for a planned run. A skipped gate counts only as
/// skipped.
fn header(
    report: &status::Report<'_>,
    skipped: &dyn Fn(&status::Judged<'_>) -> bool,
    word: Option<&str>,
) -> String {
    let owed = || report.gates.iter().filter(|j| !skipped(j));
    let count = |s: Standing| owed().filter(|j| j.standing == s).count();
    let satisfied = count(Standing::Checked(State::Satisfied));
    let mut said = String::new();
    for standing in [Standing::Attested, Standing::Waived] {
        let n = count(standing);
        if n > 0 {
            said.push_str(&format!(", {n} {}", standing.name()));
        }
    }
    let passed = report.gates.len() - owed().count();
    if passed > 0 {
        said.push_str(&format!(", {passed} skipped"));
    }
    let tally = match report.gates.len() {
        0 => "no gate is owed; nothing was checked".to_owned(),
        n => format!("{satisfied} of {n} satisfied{said}"),
    };
    let word = word.map_or_else(String::new, |w| format!("; plan {w}"));
    format!("{tally}{word}")
}

/// One `gate lines` line: the standing, then the receipt or statement.
fn gate_line(judged: &status::Judged<'_>, width: usize) -> String {
    if let Some(view) = &judged.checklist {
        return checklist::line(judged, view, width);
    }
    let id = format!("{}:", judged.definition.id);
    let standing = judged.standing.name();
    let said = match judged.standing {
        Standing::Attested => judged.statements.attested,
        Standing::Waived => judged.statements.waived,
        Standing::Checked(_) => None,
    };
    if let Some((f, s)) = said {
        return format!(
            "{id:<w$} {standing} by {}: {} {}\n",
            s.by,
            s.text,
            f.path,
            w = width + 1
        );
    }
    match judged.receipt {
        Some((f, r)) => format!(
            "{id:<w$} {standing} {} {} {}\n",
            r.outcome.name(),
            receipt::ended(r),
            f.path,
            w = width + 1
        ),
        None => format!("{id:<w$} {standing}\n", w = width + 1),
    }
}
