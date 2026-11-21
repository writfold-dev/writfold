//! A loop step in `plan status` (#76): its `loop:` line, and on the
//! current step a `next:` that says to go around, to advance, or (spent) to
//! stop and report ([`writfold_core::plan::round::next`]); a step of an
//! earlier round is marked as history. Rendering only.
use super::fold::{Graph, Step};
use super::judge::Gate;
use crate::emit::{More, Row};
use writfold_core::gate::{Standing, State};
use writfold_core::plan::round::{Next, exit, next, over};

fn line(label: &'static str, text: String) -> More {
    More {
        label,
        text,
        ..More::default()
    }
}

/// Whether a gate has a verdict about the present bytes: it ran and did
/// not pass, as opposed to open or stale.
fn judged(g: &Gate) -> bool {
    matches!(
        g.standing,
        Some(Standing::Checked(State::Failed | State::Unknown))
    )
}

pub fn annotate(
    graph: &Graph,
    step: &Step,
    (current, superseded): (bool, bool),
    gates: &[Gate],
    row: &mut Row,
) {
    if superseded {
        row.more.push(line(
            "round",
            "an earlier round: history, not judged again".into(),
        ));
    }
    if let Some(place) = graph.places(&step.path).last() {
        row.extra.push(("round", serde_json::json!(place.round)));
        row.extra
            .push(("superseded", serde_json::json!(superseded)));
    }
    let Some(((body, _), b)) = step.repeat.as_ref().zip(super::rounds::bounds(step)) else {
        return;
    };
    let rounds = graph.rounds(step);
    if over(&b, rounds) {
        row.flags.push("over-max".into());
    }
    let text = match (&step.done, &step.skipped) {
        (Some(_), _) => format!("{body}, {rounds} rounds of at most {}", b.max),
        (None, Some(_)) => format!("{body}, gave up after {rounds} of at most {} rounds", b.max),
        (None, None) => format!("{body}, round {rounds} of at most {}", b.max),
    };
    let met: Vec<bool> = gates.iter().map(Gate::met).collect();
    // Only a judged step has an exit to state: a later step's gates were
    // not judged, and an empty list would read as met.
    let judged_all = gates.len() == step.gates.len() && (current || step.done.is_some());
    let state = judged_all.then(|| exit(&b, rounds, &met));
    let spent = matches!(state, Some(writfold_core::plan::round::Exit::Spent));
    let left = if spent && step.done.is_none() && step.skipped.is_none() {
        "; no round is left"
    } else {
        ""
    };
    row.more.push(line("loop", format!("{text}{left}")));
    let word = match state {
        Some(writfold_core::plan::round::Exit::Met) => serde_json::json!("met"),
        Some(writfold_core::plan::round::Exit::Again) => serde_json::json!("again"),
        Some(writfold_core::plan::round::Exit::Spent) => serde_json::json!("spent"),
        None => serde_json::Value::Null,
    };
    row.extra.push((
        "loop",
        serde_json::json!({"template": body, "min": b.min, "max": b.max, "rounds": rounds, "exit": word}),
    ));
    if !current {
        return;
    }
    let verdicts: Vec<bool> = gates.iter().map(judged).collect();
    match next(&b, rounds, &met, &verdicts) {
        Next::Advance | Next::Judge(_) => {}
        Next::Again => {
            row.more.retain(|m| m.label != "next");
            row.more.push(line("next", "writfold kb plan again".into()));
        }
        Next::Report => {
            row.more.retain(|m| m.label != "next");
            row.flags.push("spent".into());
            row.more.push(line(
                "why",
                format!("{rounds} rounds did not meet the step's gates: stop and report; whoever owns the run decides how the step closes"),
            ));
        }
    }
}

/// The trailer's `loops=` and `spent=` (when the plan has a loop), and
/// `!over-max` making the answer exit 2 (two clones each added a round).
pub fn counts(answer: &mut crate::emit::Answer, graph: &Graph) {
    let loops = graph.steps.iter().filter(|s| s.repeat.is_some()).count();
    if loops == 0 {
        return;
    }
    let spent = answer
        .rows
        .iter()
        .filter(|r| r.flags.iter().any(|f| f == "spent"))
        .count();
    let over = answer
        .rows
        .iter()
        .any(|r| r.flags.iter().any(|f| f == "over-max"));
    answer.counts.push(("loops", loops));
    answer.counts.push(("spent", spent));
    answer.flagged |= over;
}
