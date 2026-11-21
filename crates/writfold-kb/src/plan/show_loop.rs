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
    let spent = matches!(
        exit(&b, rounds, &met),
        writfold_core::plan::round::Exit::Spent
    );
    let left = if spent && step.done.is_none() && step.skipped.is_none() {
        "; no round is left"
    } else {
        ""
    };
    row.more.push(line("loop", format!("{text}{left}")));
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
