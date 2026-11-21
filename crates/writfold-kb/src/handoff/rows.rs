//! The rows of `kb handoff check`: one per required gate, accepted or
//! refused with the reason and the next command, then a note naming what
//! was judged against what. Rendering only.
use super::Judged;
use crate::emit::{Answer, More, Row};
use crate::index::query::Rec;
use writfold_core::handoff::{Reason, Verdict, item};
use writfold_domain::GitObjectId;

fn word(reason: &Reason) -> &'static str {
    match reason {
        Reason::Empty => "empty",
        Reason::Incomplete => "incomplete",
        Reason::Undefined => "undefined",
        Reason::DefinitionChanged => "definition-changed",
        Reason::NoEvidence => "no-evidence",
        Reason::Stale => "stale",
        Reason::NotPassed => "not-passed",
        Reason::Statement => "statement",
    }
}

/// Why a gate is refused, and what to do, in words.
fn next(reason: &Reason, id: &str, run: &str, policy: &str) -> String {
    match reason {
        Reason::Undefined => format!("no definition of {id} at the policy revision; next: define .writfold/gates/{id}.toml in a reviewed commit, then name that commit as --policy"),
        Reason::DefinitionChanged => format!("the run's receipts were made under another definition than .writfold/gates/{id}.toml at {policy}; next: run the policy's definition on the candidate: writfold kb gate run {id} --run {run}"),
        Reason::NoEvidence => format!("not run; next: writfold kb gate run {id} --run {run}"),
        Reason::Stale => format!("the receipt is about other files in scope than the candidate's; next: writfold kb gate run {id} --run {run} on the candidate"),
        Reason::NotPassed => format!("it ran and did not pass; next: fix it, then writfold kb gate run {id} --run {run}"),
        Reason::Statement => format!("only a waiver or attestation stands, and a handoff needs a mechanical pass; next: writfold kb gate run {id} --run {run}"),
        Reason::Empty => "the required set names no gate; next: list the gates a handoff needs in .writfold/handoff.toml".into(),
        Reason::Incomplete => "the policy revision's gate definitions could not all be read; next: check the revision exists with its .writfold/gates/".into(),
    }
}

fn row(j: &Judged, rec: &Rec, policy: &str) -> Row {
    let refused = item(&j.item);
    let (status, text) = match &refused {
        None => (
            "accepted",
            "satisfied by a current mechanical pass".to_owned(),
        ),
        Some(reason) => (
            "refused",
            format!("{}: {}", word(reason), next(reason, &j.id, &rec.id, policy)),
        ),
    };
    let mut more = Vec::new();
    match &j.definition {
        Ok(path) => more.push(More {
            label: "instructions",
            text: format!("{path} at {policy}"),
            ..More::default()
        }),
        Err(why) => more.push(More {
            label: "instructions",
            text: why.clone(),
            ..More::default()
        }),
    }
    if let Some((path, commit)) = &j.receipt {
        let text = format!("receipt taken at {}", super::policy::short(commit));
        more.push(More {
            label: "evidence",
            text,
            path: Some(path.clone()),
            line: Some(1),
            ..More::default()
        });
    }
    Row {
        kind: "handoff",
        id: j.id.clone(),
        status: Some(status.into()),
        text,
        flags: refused.iter().map(|r| word(r).to_owned()).collect(),
        more,
        ..Row::default()
    }
}

/// The answer: a row per gate, a set-level refusal as a note, and what was
/// judged against what.
pub fn answer(
    judged: &[Judged],
    verdict: &Verdict,
    rec: &Rec,
    policy: (&GitObjectId, &str),
    candidate: &GitObjectId,
    unread: &[String],
) -> Answer {
    let (commit, named) = policy;
    let shown = super::policy::short(commit.as_str());
    let mut answer = Answer {
        rows: judged.iter().map(|j| row(j, rec, shown)).collect(),
        ..Answer::default()
    };
    if let (Verdict::Refused(_, Reason::Incomplete), Some(first)) = (verdict, unread.first()) {
        answer.notes.push(format!(
            "refused: incomplete: {} of the run's receipts, statements or covers do not read, so its newest evidence is not known: {first}; next: repair or remove them, then check again",
            unread.len()
        ));
    } else if let Verdict::Refused(_, reason @ (Reason::Empty | Reason::Incomplete)) = verdict {
        answer.notes.push(format!(
            "refused: {}: {}",
            word(reason),
            next(reason, "", &rec.id, shown)
        ));
    }
    let ids: Vec<&str> = judged.iter().map(|j| j.id.as_str()).collect();
    answer.notes.push(format!(
        "judged candidate {} against policy {named} = {shown}; required: {} ({}); evidence: run {}; nothing was run",
        candidate.as_str(),
        ids.join(", "),
        super::policy::FILE,
        rec.id
    ));
    let accepted = matches!(verdict, Verdict::Accepted);
    answer.words.push((
        "handoff",
        if accepted { "accepted" } else { "refused" }.into(),
    ));
    answer.counts = vec![
        ("required", judged.len()),
        (
            "refused",
            judged.iter().filter(|j| item(&j.item).is_some()).count(),
        ),
    ];
    answer
}
