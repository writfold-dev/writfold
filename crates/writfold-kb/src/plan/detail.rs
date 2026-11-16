//! Gate and checklist continuations requested by `plan status --gates`.
use super::fold::{Graph, Step};
use super::judge::Gate;
use crate::emit::{Answer, More};

fn gate_line(gate: &Gate) -> More {
    let state = gate.shown();
    let mut text = format!("{} [{state}]", gate.id);
    if let Some(d) = &gate.delivery {
        text.push(' ');
        text.push_str(&d.summary);
    } else if !gate.class.is_empty() {
        text.push(' ');
        text.push_str(&gate.class);
    }
    if let Some((covered, required)) = gate.counts {
        text.push_str(&format!(" {covered}/{required} required covered"));
    }
    More {
        label: "gate",
        text,
        status: Some(state.into()),
        path: gate.evidence.clone(),
        line: gate.evidence.as_ref().map(|_| 1),
        flags: if gate.met() {
            Vec::new()
        } else {
            vec![state.into()]
        },
    }
}

/// Enrich only the requested projection; ordinary plan readers stay unchanged.
pub fn add(answer: &mut Answer, graph: &Graph, judged: Option<&[Vec<Gate>]>) {
    for (i, (step, row)) in graph.steps.iter().zip(&mut answer.rows).enumerate() {
        let gates: Vec<More> = match row.status.as_deref() {
            Some("skipped") => Vec::new(),
            Some("later") => step
                .gates
                .iter()
                .map(|id| More {
                    label: "gate",
                    text: format!("{id} [later]"),
                    status: Some("later".into()),
                    ..More::default()
                })
                .collect(),
            _ => judged
                .and_then(|g| g.get(i))
                .into_iter()
                .flatten()
                .flat_map(|g| {
                    let candidate = g.delivery.as_ref().and_then(|d| d.detail.clone());
                    std::iter::once(gate_line(g))
                        .chain(candidate)
                        .chain(g.items.iter().cloned())
                })
                .collect(),
        };
        let at = row
            .more
            .iter()
            .position(|m| m.label != "why")
            .unwrap_or(row.more.len());
        row.more.splice(at..at, structure(step, graph).chain(gates));
    }
    if let Some(gates) = judged.filter(|_| !graph.steps.is_empty()) {
        match graph.current() {
            Some(i) => {
                answer.counts.push(("current", i + 1));
                answer
                    .counts
                    .push(("current_gates", graph.steps[i].gates.len()));
                answer
                    .counts
                    .push(("current_owed", gates[i].iter().filter(|g| !g.met()).count()));
            }
            None => answer.counts.push(("current", 0)),
        }
    }
}

fn structure<'a>(step: &'a Step, graph: &'a Graph) -> impl Iterator<Item = More> + 'a {
    let usage = &graph.uses[step.usage];
    let digest = step.digest.strip_prefix("sha256:").unwrap_or(&step.digest);
    let splice = More {
        label: "splice",
        text: format!(
            "{}#{} @{} from {}",
            step.template,
            usage.ordinal,
            digest.get(..12).unwrap_or(digest),
            step.source
        ),
        path: Some(step.splice.clone()),
        line: Some(1),
        ..More::default()
    };
    let under = (!usage.parent.is_empty()).then(|| More {
        label: "under",
        text: usage.parent.clone(),
        ..More::default()
    });
    std::iter::once(splice).chain(under)
}
