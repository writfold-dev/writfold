//! Only events accepted by the plan fold enter a run's timeline.
use super::{event, fold::Graph};
use crate::emit::Row;

type Event = (String, String, Row);

fn row(path: &str, recorded: &str, status: &str, id: &str, suffix: &str) -> Event {
    (
        recorded.into(),
        path.into(),
        Row {
            kind: "plan",
            id: id.into(),
            status: Some(status.into()),
            text: format!("{recorded} {status} {id}{suffix}"),
            path: Some(path.into()),
            line: Some(1),
            ..Row::default()
        },
    )
}

pub(crate) fn rows(graph: &Graph) -> Vec<Event> {
    let mut events = Vec::new();
    for (path, splice, _) in &graph.splices {
        if graph.invalid.iter().any(|(invalid, _)| invalid == path) {
            continue;
        }
        let id = event::prefix(&splice.parent, &splice.template, splice.ordinal);
        events.push(row(path, &splice.recorded, "spliced", &id, ""));
    }
    for step in &graph.steps {
        if let Some((path, done)) = &step.done {
            events.push(row(path, &done.recorded, "done", &done.node, ""));
        }
        if let Some((path, skip)) = &step.skipped {
            events.push(row(
                path,
                &skip.recorded,
                "skipped",
                &skip.node,
                &format!(" (by {}) {}", skip.by, skip.reason),
            ));
        }
    }
    events
}
