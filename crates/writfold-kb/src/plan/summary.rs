//! How a plan ended (policy spec 2.6): one word for a run's plan, decided
//! by [`writfold_core::policy::summary`] over each step's
//! [`writfold_core::policy::close`] and the run's facts. This module
//! gathers the inputs from the fold, the done records, the run's state and
//! its drift, and renders the line; the counts are for reading, the word
//! is the decision.
use super::drift::Drift;
use super::fold::Graph;
use super::judge::Gate;
use writfold_core::policy::{self, Close, Facts, How, Pin, Summary};

/// The summary of one run's plan.
#[derive(Clone, Debug, Default)]
pub struct Ended {
    pub word: &'static str,
    pub steps: usize,
    pub passed: usize,
    pub attested: usize,
    pub waived: usize,
    pub skipped: usize,
    pub unpinned: bool,
}

fn how(standing: &str) -> How {
    match standing {
        "waived" => How::Waived,
        "attested" => How::Attested,
        _ => How::Passed,
    }
}

fn word(s: &Summary) -> &'static str {
    match s {
        Summary::NoPlan => "no-plan",
        Summary::Incomplete => "incomplete",
        Summary::ClosedIncomplete => "closed-incomplete",
        Summary::PolicyMoved => "policy-moved",
        Summary::CompleteStale => "complete-stale",
        Summary::CompleteWithExceptions => "complete-with-exceptions",
        Summary::Complete => "complete",
    }
}

/// Whether every required template (named by the live file or locked by
/// the pin) is a root template of the plan.
pub fn required_present(graph: &Graph, names: &[String]) -> Vec<bool> {
    (names.iter())
        .map(|n| (graph.uses.iter()).any(|u| u.parent.is_empty() && &u.template == n))
        .collect()
}

/// The plan's word. `judged` holds each judged step's gates (for
/// `complete-stale`); `present` is [`required_present`].
pub fn ended(
    graph: &Graph,
    judged: Option<&[Vec<Gate>]>,
    (drift, run_closed): (&Drift, bool),
    present: &[bool],
) -> Ended {
    let mut closes: Vec<Close> = Vec::new();
    let mut e = Ended::default();
    for step in &graph.steps {
        let met: Vec<How> = (step.done.iter())
            .flat_map(|(_, d)| d.met.iter().map(|m| how(&m.standing)))
            .collect();
        let c = policy::close(step.done.is_some(), step.skipped.is_some(), &met);
        match c {
            Close::Passed => e.passed += 1,
            Close::Attested => e.attested += 1,
            Close::Waived => e.waived += 1,
            Close::Skipped => e.skipped += 1,
            Close::Open => {}
        }
        closes.push(c);
    }
    let stale = judged.is_some_and(|g| {
        (graph.steps.iter().zip(g))
            .any(|(s, gates)| s.done.is_some() && gates.iter().any(|j| !j.met()))
    });
    let facts = Facts {
        run_closed,
        required_present: writfold_core::plan::first_owed(present).is_none(),
        pin: match drift.word {
            "moved" => Pin::Moved,
            "same" => Pin::Same,
            _ => Pin::Unpinned,
        },
        repinned: drift.pins.iter().any(|p| p.kind == "repin"),
        stale,
    };
    e.word = word(&policy::summary(&closes, &facts));
    e.steps = graph.steps.len();
    e.unpinned = drift.word == "unpinned";
    e
}

impl Ended {
    /// `complete-with-exceptions`, or `complete (unpinned)`.
    pub fn shown(&self) -> String {
        match self.unpinned && self.word.starts_with("complete") {
            true => format!("{} (unpinned)", self.word),
            false => self.word.to_owned(),
        }
    }

    /// `2 steps: 0 passed, 0 attested, 1 waived, 1 skipped`.
    pub fn counts(&self) -> String {
        let mut parts = vec![format!("{} passed", self.passed)];
        if self.attested > 0 || self.waived > 0 || self.skipped > 0 {
            parts.push(format!("{} attested", self.attested));
            parts.push(format!("{} waived", self.waived));
            parts.push(format!("{} skipped", self.skipped));
        }
        let s = if self.steps == 1 { "" } else { "s" };
        format!("{} step{s}: {}", self.steps, parts.join(", "))
    }

    /// The JSON keys of the plan row and the trailer.
    pub fn keys(&self) -> Vec<(&'static str, serde_json::Value)> {
        vec![
            ("summary", serde_json::json!(self.word)),
            ("passed", serde_json::json!(self.passed)),
            ("attested", serde_json::json!(self.attested)),
            ("waived", serde_json::json!(self.waived)),
        ]
    }

    /// Whether the word exits 2 (`policy-moved`, `complete-stale`).
    pub fn flagged(&self) -> bool {
        matches!(self.word, "policy-moved" | "complete-stale")
    }
}

/// Make `resume`'s plan line say how the plan ended: the summary row when
/// every step is closed (`plan complete-with-exceptions: …`), the open
/// step with the closing when a closed run's plan is incomplete.
pub fn resume(row: &mut crate::emit::Row, ended: &Ended, closed: Option<String>) {
    match ended.word {
        "incomplete" | "no-plan" => {}
        "closed-incomplete" => {
            row.id = ended.word.into();
            row.status = Some(ended.word.into());
            let closed = closed.map_or_else(String::new, |c| format!("; closed {c}"));
            row.text = format!("{}{closed}", row.text);
        }
        _ => {
            row.id = ended.shown();
            row.status = Some(ended.word.into());
            row.text = ended.counts();
        }
    }
    row.extra.extend(ended.keys());
}
