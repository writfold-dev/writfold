//! The Weft assessed gate: a gate of class `assessed` is met by a Weft
//! assessment of a plan step, not by a command's exit status.
//!
//! The agent states the step's assumptions and checks in a Weft file
//! ([`spec`]: shipped beside the definition, or submitted in the run). An
//! assumption cites its evidence with Weft's own binding,
//! `claim <name> = citation "<reference>";`, declared directly above it
//! ([`report`]). `gate run` resolves every citation ([`crate::gate::ground`]: a KB
//! citation must be fresh, a receipt satisfied or attested), runs the
//! pinned `weft` binary ([`weft`]), and records one receipt
//! ([`receipt`]). A required check is met only when Weft entails it and
//! every assumption Weft used for it is grounded
//! ([`writfold_core::assess`]); one resting on an ungrounded assumption
//! keeps Weft's result and is reported `hypothetical`.
//!
//! The obligation state is G1's ([`writfold_core::gate::state`]): a
//! receipt is about the present when the definition, the Weft source's
//! bytes and the standing of every cited piece of evidence are what it
//! recorded. Waivers and attestations of an assessed gate are not built.
mod judge;
pub mod receipt;
pub mod report;
mod run;
mod show;
pub mod spec;
pub mod weft;

pub use judge::{Assessment, judge};
pub use run::run;
pub use show::{line, row};
pub use spec::Spec;

use std::path::Path;

/// One assessed receipt file of a run.
pub struct Filed {
    /// KB-relative path.
    pub path: String,
    pub name: String,
    pub parsed: std::result::Result<receipt::Assessed, String>,
}

/// Every assessed receipt of the run whose directory is `run_dir`.
pub fn filed(root: &Path, run_dir: &str, run: &str) -> Vec<Filed> {
    let Ok(entries) = std::fs::read_dir(root.join(run_dir).join(receipt::DIR)) else {
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
