//! Rows: `plan status` (one per step, then invalid events), the one plan
//! line `resume` shows, and what a refused `plan advance` owes.
use super::fold::{Graph, Step};
use super::judge::Gate;
use super::moved::Moved;
use crate::emit::{More, Row};

fn line(label: &'static str, text: String) -> More {
    More {
        label,
        text,
        ..More::default()
    }
}

/// `unit [stale], wording [open]`.
pub fn listed(gates: &[&Gate]) -> String {
    let parts: Vec<String> = gates
        .iter()
        .map(|g| format!("{} [{}]", g.id, g.display()))
        .collect();
    parts.join(", ")
}

/// The command that would meet `gate`; `None` for a delivery nothing can
/// meet now (its `why:` line says so).
pub fn next(gate: &Gate) -> Option<String> {
    if let Some(command) = &gate.command {
        return Some(command.clone());
    }
    if gate.delivery.is_some() {
        return None;
    }
    Some(match (gate.standing.is_some(), gate.attested) {
        (false, _) => format!(
            "define {}/{}.toml in the KB root or the repository",
            crate::gate::def::DIR,
            gate.id
        ),
        (true, true) => format!(
            "writfold kb gate attest {} --statement TEXT --by WHO",
            gate.id
        ),
        (true, false) => format!("writfold kb gate run {}", gate.id),
    })
}

/// `next: <command>`, or `why: <reason>` for a delivery nothing can meet.
pub fn next_or_why(gate: &Gate) -> String {
    match (
        next(gate),
        gate.delivery.as_ref().and_then(|d| d.why.as_ref()),
    ) {
        (Some(command), _) => format!("next: {command}"),
        (None, Some(why)) => format!("why: {why}"),
        (None, None) => "next: writfold kb plan status".into(),
    }
}

/// What the current step owes, with `read:` and `next:` lines.
pub fn owes(row: &mut Row, gates: &[Gate]) {
    let owed: Vec<&Gate> = gates.iter().filter(|g| !g.met()).collect();
    super::delivery::lines(row, gates);
    if owed.is_empty() {
        row.text = match gates.is_empty() {
            true => "no gates".into(),
            false => format!("met: {}", listed(&gates.iter().collect::<Vec<_>>())),
        };
        for g in gates {
            let Some(mode) = crate::deliver::step::mode_of(&g.id) else {
                continue;
            };
            let posted = format!("{}{mode}", crate::deliver::step::POSTED);
            if !gates.iter().any(|o| o.id == posted) {
                row.more.push(line(
                    "note",
                    format!("nothing on this step records a post; add {posted} to the template to hold the step until one is recorded"),
                ));
            }
        }
        row.more
            .push(line("next", "writfold kb plan advance".into()));
        return;
    }
    row.text = format!("owes: {}", listed(&owed));
    for gate in &owed {
        if let Some(path) = &gate.read {
            row.more.push(More {
                label: "read",
                text: format!("instructions for {}", gate.id),
                path: Some(path.clone()),
                line: Some(1),
                ..More::default()
            });
        }
    }
    if let Some(command) = owed.first().and_then(|g| next(g)) {
        row.more.push(line("next", command));
    }
}

/// One step's row; `gates` is empty for steps not judged.
pub fn step_row(
    step: &Step,
    current: bool,
    gates: &[Gate],
    (judged, superseded): (bool, bool),
) -> Row {
    let mut row = Row {
        kind: "step",
        id: step.path.clone(),
        path: Some(step.splice.clone()),
        line: Some(1),
        section: Some("plan"),
        ..Row::default()
    };
    let names = match step.gates.is_empty() {
        true => "no gates".to_owned(),
        false => format!("gates: {}", step.gates.join(", ")),
    };
    if let Some((path, done)) = &step.done {
        row.status = Some("done".into());
        row.path = Some(path.clone());
        let owed: Vec<&Gate> = gates.iter().filter(|g| !g.met()).collect();
        row.text = match (judged, gates.is_empty()) {
            (true, false) => listed(&gates.iter().collect::<Vec<_>>()),
            _ => names,
        };
        let facts: Vec<_> = gates.iter().map(Gate::fact).collect();
        let met = writfold_core::plan::met_all(&facts);
        let node = writfold_core::plan::Node {
            done: true,
            skipped: step.skipped.is_some(),
        };
        if judged && writfold_core::plan::round::stale(&node, &met, superseded) {
            row.flags.push("stale".into());
            row.more.push(line(
                "why",
                format!(
                    "no longer holds: {} (done stays done; list the gate again on a later step if it must hold there)",
                    listed(&owed)
                ),
            ));
        }
        row.more
            .push(line("done", format!("recorded {}", done.recorded)));
    } else if let Some((path, skip)) = &step.skipped {
        row.status = Some("skipped".into());
        row.path = Some(path.clone());
        row.text = format!("{} (by {}, {})", skip.reason, skip.by, skip.recorded);
        row.flags.push("skipped".into());
    } else if current {
        row.status = Some("current".into());
        row.text = names;
        if judged {
            owes(&mut row, gates);
        }
    } else {
        row.status = Some("later".into());
        row.text = names;
    }
    row
}

fn has_moved(step: &Step, moved: &[Moved]) -> bool {
    moved.iter().any(|m| m.covers(&step.template, &step.digest))
}

/// The plan status rows: each step, then each invalid event. A step whose
/// template has moved is flagged `!template-moved`.
pub fn rows(graph: &Graph, gates: Option<&[Vec<Gate>]>, moved: &[Moved]) -> Vec<Row> {
    let current = graph.current();
    let mut out: Vec<Row> = graph
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let judged = gates.and_then(|g| g.get(i));
            let gates = judged.map_or(&[][..], Vec::as_slice);
            let superseded = graph.superseded(&s.path);
            let mut row = step_row(s, current == Some(i), gates, (judged.is_some(), superseded));
            super::show_loop::annotate(graph, s, (current == Some(i), superseded), gates, &mut row);
            if has_moved(s, moved) {
                row.flags.push("template-moved".into());
            }
            row
        })
        .collect();
    for (path, why) in &graph.invalid {
        out.push(Row {
            kind: "plan-event",
            id: path
                .rsplit('/')
                .next()
                .unwrap_or(path)
                .trim_end_matches(".json")
                .to_owned(),
            status: Some("invalid".into()),
            text: why.clone(),
            path: Some(path.clone()),
            line: Some(1),
            flags: vec!["invalid".into()],
            section: Some("plan"),
            ..Row::default()
        });
    }
    out
}

/// The one line `resume` shows: the current step and what it still owes.
pub fn resume_row(graph: &Graph, gates: Option<&[Vec<Gate>]>, moved: &[Moved]) -> Row {
    let total = graph.steps.len();
    let Some(i) = graph.current() else {
        return Row {
            kind: "plan",
            id: "complete".into(),
            status: Some("complete".into()),
            text: format!("every step is done or skipped ({total} steps)"),
            section: Some("plan"),
            ..Row::default()
        };
    };
    let step = &graph.steps[i];
    let mut row = Row {
        kind: "plan",
        id: step.path.clone(),
        status: Some("current".into()),
        path: Some(step.splice.clone()),
        line: Some(1),
        section: Some("plan"),
        ..Row::default()
    };
    let at = format!("step {}/{total}", i + 1);
    match gates.and_then(|g| g.get(i)) {
        Some(judged) => {
            owes(&mut row, judged);
            row.text = format!("{at} {}", row.text);
            row.more
                .retain(|m| matches!(m.label, "candidate" | "why" | "next"));
        }
        None => row.text = format!("{at} gates: {}", step.gates.join(", ")),
    }
    let stale = gates.is_some_and(|g| {
        graph
            .steps
            .iter()
            .zip(g)
            .any(|(s, judged)| s.done.is_some() && judged.iter().any(|j| !j.met()))
    });
    if stale {
        row.flags.push("stale".into());
    }
    if graph
        .steps
        .iter()
        .any(|s| !s.closed() && has_moved(s, moved))
    {
        row.flags.push("template-moved".into());
    }
    row
}
