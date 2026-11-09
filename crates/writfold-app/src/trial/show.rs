//! `writfold trial show`: the run's validated ledger history and five
//! independent badges (evidence, assessment, approval, claim, effect). There
//! is deliberately no aggregate status: each badge says only what its records
//! establish.
use super::flow::{Blob, Event, Premises};
use super::{Result, Trial, limits, now, stream};
use std::path::Path;
use writfold_kernel::trial::CheckResult;
use writfold_wire::signatures::edit::SignedEditApproval;
use writfold_wire::weft_report::parse_report;

fn history(trial: &Trial, run: &str, event: &Event) -> String {
    match event {
        Event::Prepared {
            path,
            base,
            candidate,
            ..
        } => format!(
            "prepared  {path} at {base}, candidate {} bytes",
            candidate.bytes
        ),
        Event::Approved { .. } => "approved  operator grant + signed approval retained".to_owned(),
        Event::Claimed { .. } => "claimed   durable claim before touching the file".to_owned(),
        Event::Applied { reconciled, .. } => format!(
            "applied   candidate installed in {}{}",
            trial.worktree(run).display(),
            if *reconciled {
                " (reconciled by recover)"
            } else {
                ""
            }
        ),
        Event::Abandoned { reason } => format!("abandoned {reason}"),
    }
}

fn assessment(trial: &Trial, report: &Blob) -> Result<String> {
    if report.bytes == 0 {
        return Ok("none (no Weft report bound)".to_owned());
    }
    let parsed = parse_report(&trial.fetch(report)?, limits()?)?;
    let count = |r: CheckResult| parsed.checks.iter().filter(|c| c.result == r).count();
    let entailed = count(CheckResult::Entailed);
    let refuted = count(CheckResult::Refuted);
    let open = parsed.checks.len() - entailed - refuted;
    Ok(format!(
        "{entailed} entailed, {open} open, {refuted} refuted (Weft {}, hypothetical)",
        parsed.scope
    ))
}

fn approval(trial: &Trial, approval: &Blob) -> Result<String> {
    let signed = SignedEditApproval::decode(&trial.fetch(approval)?, limits()?)?;
    let expires = signed.approval().validity().expires().get();
    Ok(if now()? < expires {
        format!("signed, valid until {expires}")
    } else {
        format!("signed, expired at {expires}")
    })
}

/// The run's history and badges.
pub fn show(state: &Path, run: &str) -> Result<String> {
    let trial = Trial::open(state)?;
    let executable = trial.executable()?;
    let lease = trial.writer.try_lease()?;
    let loaded = trial
        .ledger(&lease, &executable)?
        .load::<Event, Premises>(&stream(run)?)?;
    let mut out = format!(
        "run {run}: {} record(s), history validated\n",
        loaded.records.len()
    );
    let (mut assessed, mut approved) = ("none".to_owned(), "none".to_owned());
    let (mut claim, mut effect) = ("none", "none".to_owned());
    for record in &loaded.records {
        let event = &record.payload;
        out.push_str(&format!(
            "  {}. {}\n",
            record.sequence.get(),
            history(&trial, run, event)
        ));
        match event {
            Event::Prepared { report, .. } => assessed = assessment(&trial, report)?,
            Event::Approved { approval: a, .. } => approved = approval(&trial, a)?,
            Event::Claimed { .. } => {
                claim = "recorded before touching the file";
                effect = "unknown (claimed, no outcome yet; see recover)".to_owned();
            }
            Event::Applied { reconciled, .. } => {
                effect = if *reconciled {
                    "applied (reconciled)"
                } else {
                    "applied"
                }
                .to_owned();
            }
            Event::Abandoned { reason } => effect = format!("abandoned: {reason}"),
        }
    }
    out.push_str(&format!(
        "  evidence    none retained\n  assessment  {assessed}\n  approval    {approved}\n  claim       {claim}\n  effect      {effect}\n"
    ));
    Ok(out)
}
