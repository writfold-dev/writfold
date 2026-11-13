//! Checklist gates: a gate of class `checklist` declares items
//! ([`spec`]), and is met when every required item is covered by a row
//! someone records with `gate cover` ([`cover`], [`write`]): `supported`,
//! citing evidence Writfold grounds ([`crate::gate::ground`]), or
//! `not-applicable`, with a reason. `unsupported` records that an item
//! does not hold.
//!
//! Each item's standing is derived on every read ([`judge`]): its newest
//! row by `seq`, compared with the gate's current definition and the
//! content in its scope, with every cited reference re-grounded; the
//! kernel is [`writfold_core::checklist`]. A covered checklist stands as
//! `attested` through the gate kernel ([`writfold_core::gate::standing`]):
//! a cover row is a judgement with checked evidence, never a passing
//! check. `gate waive` sets a whole checklist aside as for any gate.
//!
//! Checklist and assessed gates are both judged after the other gates and
//! against them only, so neither can cite the other.
pub mod cover;
mod judge;
mod show;
pub mod spec;
pub mod write;

pub use judge::{Entry, View, judge};
pub use show::{line, next, row};
pub use spec::Spec;
pub use write::Cover;
pub use writfold_core::checklist::Outcome;

use std::path::Path;

/// One cover row file of a run.
pub struct Filed {
    /// KB-relative path.
    pub path: String,
    pub name: String,
    pub parsed: std::result::Result<cover::Cover, String>,
}

/// Every cover row of the run whose directory is `run_dir`, by name.
pub fn filed(root: &Path, run_dir: &str, run: &str) -> Vec<Filed> {
    let Ok(entries) = std::fs::read_dir(root.join(run_dir).join(cover::DIR)) else {
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
            let path = format!("{run_dir}/{}/{name}", cover::DIR);
            let parsed = std::fs::read_to_string(root.join(&path))
                .map_err(|e| format!("unreadable ({e})"))
                .and_then(|text| cover::parse(&text, run, &name));
            Filed { path, name, parsed }
        })
        .collect()
}

/// The valid rows of `gate` and `item` on `repo`, oldest first
/// ([`cover::order`]).
pub fn rows_of<'a>(
    filed: &'a [Filed],
    gate: &str,
    item: &str,
    repo: &str,
) -> Vec<(&'a Filed, &'a cover::Cover)> {
    let mut mine: Vec<(&Filed, &cover::Cover)> = filed
        .iter()
        .filter_map(|f| f.parsed.as_ref().ok().map(|c| (f, c)))
        .filter(|(_, c)| c.gate == gate && c.item == item && c.repo == repo)
        .collect();
    mine.sort_by(|a, b| cover::order((&a.0.name, a.1), (&b.0.name, b.1)));
    mine
}
