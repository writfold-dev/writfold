//! `writfold trial`: the first connected prepare → approve → apply path, in
//! the cooperating-user `writfold-trial-worktree/1` profile.
//!
//! `prepare` adds a detached worktree of the user's repository at HEAD inside
//! the private state directory, replays a Splicery recipe through the pinned
//! kernel, retains source, recipe, candidate and report, and appends
//! `Prepared`. `approve` signs an operator grant and a prepared-edit approval
//! with the state's operator key and appends `Approved`. `apply` re-verifies
//! both signatures and the current grant, rechecks the worktree file against
//! the retained source, installs the candidate through the staged installer
//! and appends `Applied`. Every append goes through the durable ledger, which
//! replays the run's history through the kernel first.
//!
//! `writfold serve` exposes `show`, `retain` and `prepare` as the negotiated
//! `writfold.trial/1` capability ([`rpc`]); approve, apply and recover stay
//! CLI-only.
mod admit;
mod apply;
mod approve;
#[cfg(test)]
mod assessment_tests;
pub mod cli;
pub mod evidence;
pub mod flow;
mod prepare;
mod recover;
#[cfg(test)]
mod recovery_tests;
mod refused;
pub mod retain;
#[cfg(test)]
mod retain_tests;
pub(crate) mod rpc;
#[cfg(test)]
mod rpc_tests;
pub mod run;
mod show;
#[cfg(test)]
mod sparse_tests;
mod state;
#[cfg(test)]
mod tests;

pub use apply::{apply, authorize};
pub use approve::approve;
pub use prepare::{Prepared, prepare};
pub use recover::recover;
pub use refused::BeforeEffects;
pub use state::Trial;

use std::time::Duration;
use writfold_domain::worktree::Scope;
use writfold_domain::{ByteCount, StreamId};
use writfold_runtime::git::GitBudgets;
use writfold_wire::Limits;

pub type Result<T, E = Box<dyn std::error::Error + Send + Sync>> = std::result::Result<T, E>;

pub const PROFILE: &str = "writfold-trial-worktree/1";
/// Largest file, recipe, report or record this profile handles.
pub const BUDGET: usize = 1 << 20;
/// The grant and approval validity window.
pub const VALIDITY_SECONDS: u64 = 3600;

pub fn limits() -> Result<Limits> {
    Ok(Limits::new(BUDGET, 64, 65_536, 65_536)?)
}

pub fn budgets() -> Result<GitBudgets> {
    let bytes = ByteCount::new(BUDGET as u64 * 4)?;
    Ok(GitBudgets::new(
        bytes,
        ByteCount::new(1 << 16)?,
        bytes,
        Duration::from_secs(120),
    ))
}

/// One run's scope: the trial realm, the run, its own worktree and the base
/// commit it was prepared from.
pub fn scope(run: &str, base: &str) -> Result<Scope> {
    let short = base.get(..12).ok_or("base commit")?;
    Ok(Scope::new(
        state::REALM.parse()?,
        run.parse()?,
        "repo".parse()?,
        run.parse()?,
        format!("head.{short}").parse()?,
    )?)
}

pub fn stream(run: &str) -> Result<StreamId> {
    Ok(run.parse()?)
}

pub fn now() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}
