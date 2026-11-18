//! Drift (policy spec 2.5): each pinned gate compared with the present on
//! every read. An open step's gate is compared with its definition digest
//! now (a gate that no longer loads counts as different); a done step's
//! with the definition its done record names (a delivery is not compared
//! once done: its own `!stale` covers a changed mode); the pinned
//! required file with that file's digest now. What is pinned is the entry
//! in force ([`pin::gate_in_force`], [`pin::required_in_force`]): the
//! newest pin naming the gate, so a `plan repin` replaces what earlier
//! pins held. One difference makes the run's policy moved. The
//! comparisons are [`writfold_core::policy::pin_one`] and
//! [`writfold_core::policy::pin_all`]; this module gathers them.
//! Drift blocks no step: it changes the summary, never `plan advance`.
use super::Loaded;
use super::pin::{self, Pin};
use super::pin_write::definition_now;
use crate::runs;
use std::path::Path;
use writfold_core::policy::{self, Pin as Same};

/// The run's policy against its pins.
#[derive(Clone, Debug, Default)]
pub struct Drift {
    /// `same`, `moved` or `unpinned`.
    pub word: &'static str,
    /// `(step index, why)` for each step whose gate moved.
    pub steps: Vec<(usize, String)>,
    /// The gates that moved, once each, in step order.
    pub gates: Vec<String>,
    /// The pins, by `seq`.
    pub pins: Vec<Pin>,
    pub required_moved: bool,
    /// The pin files that do not read as written, each with why.
    pub invalid: Vec<(String, String)>,
}

fn short(digest: &str) -> String {
    let d = digest.strip_prefix("sha256:").unwrap_or(digest);
    d.get(..12).unwrap_or(d).to_owned()
}

fn compare(pinned: &str, now: Option<&str>) -> Same {
    let pinned = Some(pinned.as_bytes().to_vec());
    let now = now.map(|n| n.as_bytes().to_vec());
    policy::pin_one(&pinned, &now)
}

fn moved(p: &Same) -> bool {
    matches!(p, Same::Moved)
}

/// Every pinned gate, once.
fn pinned(pins: &[Pin]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (gate, _) in pins.iter().flat_map(|p| &p.gates) {
        if !out.contains(gate) {
            out.push(gate.clone());
        }
    }
    out
}

/// Why a step's gate reads moved: `done` when the step is done (then
/// `now` is the definition its record names).
fn why(gate: &str, digest: &str, done: bool, now: Option<&str>, path: &str) -> String {
    let then = format!("{gate}@{}", short(digest));
    match (done, now) {
        (true, n) => format!(
            "met under {gate}@{}; this run pinned {then} (@{path}:1)",
            short(n.unwrap_or("none"))
        ),
        (false, Some(n)) => format!(
            "{gate} is now @{}; this run pinned {then}: evidence is judged under the definition as it is now",
            short(n)
        ),
        (false, None) => format!("{gate} no longer loads; this run pinned {then} (@{path}:1)"),
    }
}

/// Judge the run's drift.
pub fn judge(root: &Path, loaded: &Loaded) -> Drift {
    let (pins, invalid) = pin::read(root, &runs::dir(&loaded.rec), &loaded.rec.id);
    let mut drift = Drift {
        word: "unpinned",
        ..Drift::default()
    };
    if pins.is_empty() && invalid.is_empty() {
        return drift;
    }
    let top = loaded.subject.as_ref().map(|s| s.top.as_path());
    // An edited pin reads invalid: the run is then judged moved.
    let mut checks: Vec<Same> = invalid.iter().map(|_| compare("pin", None)).collect();
    let mut now_of = std::collections::BTreeMap::new();
    for gate in pinned(&pins) {
        let naming =
            (loaded.graph.steps.iter().enumerate()).filter(|(_, s)| s.gates.contains(&gate));
        for (i, step) in naming {
            let Some((digest, pin)) = pin::gate_in_force(&pins, &gate, Some(&step.path)) else {
                continue;
            };
            let path = &pin.path;
            let delivery = crate::deliver::step::is_delivery_word(&gate);
            // A skipped step owes nothing: its gates are not compared.
            if step.done.is_none() && step.skipped.is_some() {
                continue;
            }
            let (now, done) = match &step.done {
                Some(_) if delivery => continue,
                Some((_, d)) => (
                    (d.met.iter().find(|m| m.gate == gate)).map(|m| m.definition.clone()),
                    true,
                ),
                None => {
                    let found = now_of.entry(gate.clone());
                    (
                        found
                            .or_insert_with(|| definition_now(root, top, &gate))
                            .clone(),
                        false,
                    )
                }
            };
            let r = compare(digest, now.as_deref());
            if moved(&r) {
                let why = why(&gate, digest, done, now.as_deref(), path);
                drift.steps.push((i, why));
                if !drift.gates.contains(&gate) {
                    drift.gates.push(gate.clone());
                }
            }
            checks.push(r);
        }
    }
    if let Some((_, req)) = pin::required_in_force(&pins) {
        let repo = (loaded.subject.as_ref()).map(|s| (s.top.as_path(), s.name.as_str()));
        let now = super::required::find(root, repo)
            .ok()
            .map(|r| r.digest.unwrap_or_else(|| "absent".into()));
        let r = compare(&req.digest, now.as_deref());
        drift.required_moved = moved(&r);
        checks.push(r);
    }
    drift.word = match policy::pin_all(&checks) {
        Same::Unpinned => "unpinned",
        Same::Same => "same",
        Same::Moved => "moved",
    };
    drift.pins = pins;
    drift.invalid = invalid;
    drift
}

impl Drift {
    pub fn moved(&self) -> bool {
        self.word == "moved"
    }

    /// The note `plan status` adds when the policy moved or was never pinned.
    pub fn note(&self) -> Option<String> {
        match self.word {
            "moved" => {
                let mut what = Vec::new();
                if !self.gates.is_empty() {
                    let n = self.gates.len();
                    let s = if n == 1 { "" } else { "s" };
                    what.push(format!("{n} gate{s} ({})", self.gates.join(", ")));
                }
                if self.required_moved {
                    what.push("the required steps file".into());
                }
                if what.is_empty() {
                    what.push("a pin file no longer reads as written".into());
                }
                Some(format!(
                    "policy moved since this run pinned it: {}; the plan cannot read complete until the definitions are restored or the run adopts them: writfold kb plan repin --reason TEXT --by WHO",
                    what.join(" and ")
                ))
            }
            "unpinned" => Some(
                "this run's plan was set before policy pins were recorded; its steps are judged under the definitions as they are now".into(),
            ),
            _ => None,
        }
    }
}

/// Drift from the records alone, without a subject repository (`runs
/// --plan`): each pinned gate against the definition the done records
/// name; open steps and the required file are not compared here.
pub fn recorded(root: &Path, graph: &super::fold::Graph, rec: &crate::index::query::Rec) -> Drift {
    let (pins, invalid) = pin::read(root, &runs::dir(rec), &rec.id);
    let mut drift = Drift {
        word: "unpinned",
        ..Drift::default()
    };
    if pins.is_empty() && invalid.is_empty() {
        return drift;
    }
    let mut checks: Vec<Same> = invalid.iter().map(|_| compare("pin", None)).collect();
    for gate in pinned(&pins) {
        let done = (graph.steps.iter())
            .filter(|s| s.gates.contains(&gate) && !crate::deliver::step::is_delivery_word(&gate))
            .filter_map(|s| Some((s, &s.done.as_ref()?.1)));
        for (step, d) in done {
            let Some((digest, _)) = pin::gate_in_force(&pins, &gate, Some(&step.path)) else {
                continue;
            };
            let now = d
                .met
                .iter()
                .find(|m| m.gate == gate)
                .map(|m| m.definition.as_str());
            checks.push(compare(digest, now));
        }
    }
    drift.word = match policy::pin_all(&checks) {
        Same::Moved => "moved",
        _ => "same",
    };
    drift.pins = pins;
    drift.invalid = invalid;
    drift
}
