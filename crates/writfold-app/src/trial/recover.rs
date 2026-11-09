//! `writfold trial recover`: resolve a run whose `Claimed` has no outcome,
//! for example after a crash between install and the `Applied` record.
//!
//! The worktree file is observed, never rewritten: the exact candidate bytes
//! mean the install happened (`Applied`, reconciled); the exact source bytes
//! mean it did not (`Abandoned`). Anything else stays Unknown and refuses, so
//! a person decides; recovery never re-dispatches the edit. The decision is
//! the proved `writfold_core::trial::recovery` (`recovery_spec`).
use super::apply::append;
use super::flow::{Event, Premises};
use super::state::Trial;
use super::{BUDGET, Result, stream};
use std::io::Read;
use std::path::Path;
use writfold_kernel::trial::Outcome;

/// Returns the outcome that was recorded.
pub fn recover(state: &Path, run: &str) -> Result<&'static str> {
    let trial = Trial::open(state)?;
    let executable = trial.executable()?;
    let lease = trial.writer.try_lease()?;
    let ledger = trial.ledger(&lease, &executable)?;
    let loaded = ledger.load::<Event, Premises>(&stream(run)?)?;
    let [first, _, claimed] = loaded.records.as_slice() else {
        return Err(format!("run {run} has no claim awaiting an outcome").into());
    };
    let Event::Prepared {
        path,
        source,
        candidate,
        ..
    } = &first.payload
    else {
        return Err(format!("run {run} has no prepared edit").into());
    };
    let mut current = Vec::new();
    std::fs::File::open(trial.worktree(run).join(path))?
        .take(BUDGET as u64 + 1)
        .read_to_end(&mut current)?;
    let decided =
        writfold_kernel::trial::recovery(&current, &trial.fetch(source)?, &trial.fetch(candidate)?);
    match decided {
        Outcome::Applied => {
            let event = Event::Applied {
                candidate: *candidate,
                reconciled: true,
            };
            append(&ledger, run, claimed, 4, "applied", event)?;
            Ok("applied (reconciled: the candidate is installed)")
        }
        Outcome::Abandoned => {
            let reason = "recovered: the original was never replaced".to_owned();
            append(
                &ledger,
                run,
                claimed,
                4,
                "abandoned",
                Event::Abandoned { reason },
            )?;
            Ok("abandoned (the original is unchanged)")
        }
        Outcome::Unknown => Err(format!(
            "{path} holds neither the source nor the candidate; the outcome stays unknown"
        )
        .into()),
    }
}
