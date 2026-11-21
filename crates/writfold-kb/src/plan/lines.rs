//! What `gate lines` reads from a run's plan: the plan's word and, for
//! each gate, how the steps naming it closed, so that a gate only skipped
//! steps name is listed `skipped`
//! ([`writfold_core::policy::lines::all_skipped`]).
use super::{Loaded, drift, status};
use crate::index::query::Rec;
use crate::session::Session;
use std::path::Path;
use writfold_core::policy::{Close, Summary, lines::all_skipped};

/// A planned run, as `gate lines` shows it.
pub struct PlanLines {
    /// The plan's word as shown (`complete-with-exceptions`).
    pub word: String,
    pub summary: Summary,
    /// Each step's gates with how the step closed.
    steps: Vec<(Vec<String>, Close)>,
    /// One line per loop step: how many rounds it took, or gave up after
    /// (#78). Shown only; the exit rule is unchanged.
    pub loops: Vec<String>,
}

impl PlanLines {
    /// Whether every step that names `gate` was skipped (and one does).
    pub fn skipped(&self, gate: &str) -> bool {
        let owners: Vec<Close> = (self.steps.iter())
            .filter(|(gates, _)| gates.iter().any(|g| g == gate))
            .map(|(_, close)| copy(close))
            .collect();
        all_skipped(&owners)
    }
}

fn copy(close: &Close) -> Close {
    match close {
        Close::Open => Close::Open,
        Close::Passed => Close::Passed,
        Close::Attested => Close::Attested,
        Close::Waived => Close::Waived,
        Close::Skipped => Close::Skipped,
    }
}

/// The run's plan for `gate lines`; `None` when it has none (or it cannot
/// be read: then `gate lines` answers as without a plan).
pub fn of_run(
    session: &Session<'_>,
    rec: &Rec,
    repo: Option<&Path>,
    cwd: &Path,
) -> Option<PlanLines> {
    let loaded = Loaded::open(session, Some(&rec.id), repo, cwd).ok()?;
    if !loaded.graph.planned() {
        return None;
    }
    let judged = loaded.judged(session).ok().flatten();
    let drift = drift::judge(&session.ws.root, &loaded);
    let (ended, summary, closes) = status::decided(session, &loaded, judged.as_deref(), &drift);
    let steps = (loaded.graph.steps.iter())
        .map(|s| s.gates.clone())
        .zip(closes)
        .collect();
    let graph = &loaded.graph;
    let loops = (graph.steps.iter())
        .filter_map(|s| Some((s, s.repeat.as_ref()?)))
        .map(|(s, (_, r))| {
            let rounds = graph.rounds(s);
            let how = match (&s.done, &s.skipped) {
                (Some(_), _) => format!("{rounds} rounds of at most {}", r.max),
                (None, Some(_)) => format!("gave up after {rounds} of at most {} rounds", r.max),
                (None, None) => format!("round {rounds} of at most {}, open", r.max),
            };
            format!("loop {}: {how}", s.path)
        })
        .collect();
    Some(PlanLines {
        word: ended.shown(),
        summary,
        steps,
        loops,
    })
}
