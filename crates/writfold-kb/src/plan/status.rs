//! The ordinary and detailed plan status projections.
use super::{Loaded, detail, disorder, judge, moved, show, template};
use crate::emit::Answer;
use crate::error::Result;
use crate::runs;
use crate::session::Session;

/// The `plan status` answer.
pub(super) fn status(session: &Session<'_>, loaded: &Loaded, gates: bool) -> Result<Answer> {
    let mut answer = Answer::default();
    let graph = &loaded.graph;
    if !graph.planned() {
        answer.unknown = Some("no-plan");
        answer.notes.push(format!(
            "run {} has no plan: writfold kb plan set <template> (templates in {}/)",
            loaded.rec.id,
            template::DIR
        ));
    }
    let judged = match &loaded.subject {
        Some(subject) if gates => Some(judge::steps_detailed(
            session,
            &runs::dir(&loaded.rec),
            &loaded.rec.id,
            graph,
            subject,
            &loaded.records,
        )?),
        _ => loaded.judged(session)?,
    };
    if judged.is_none() && graph.planned() {
        answer
            .notes
            .push("no subject repository here: gates are not judged; pass --repo DIR".into());
    }
    answer.rows = show::rows(graph, judged.as_deref(), &loaded.moved);
    let disorder = disorder::find(graph);
    disorder.mark(&mut answer.rows);
    let count = |status: &str| {
        answer
            .rows
            .iter()
            .filter(|r| r.status.as_deref() == Some(status))
            .count()
    };
    let stale = answer
        .rows
        .iter()
        .filter(|r| r.flags.iter().any(|f| f == "stale"))
        .count();
    let flagged = |flag: &str| {
        answer
            .rows
            .iter()
            .filter(|r| r.flags.iter().any(|f| f == flag))
            .count()
    };
    let moved = flagged("template-moved");
    answer.counts = vec![
        ("steps", graph.steps.len()),
        ("done", count("done")),
        ("skipped", count("skipped")),
        ("stale", stale),
        ("moved", moved),
        ("invalid", graph.invalid.len()),
    ];
    if gates {
        detail::add(&mut answer, graph, judged.as_deref());
    }
    if disorder.count() > 0 {
        answer.counts.push(("disorder", disorder.count()));
    }
    notes(&mut answer, loaded);
    answer.notes.extend(disorder.notes);
    Ok(answer)
}

fn notes(answer: &mut Answer, loaded: &Loaded) {
    let graph = &loaded.graph;
    let spliced: Vec<String> = graph
        .splices
        .iter()
        .map(|(_, s, t)| {
            let under = match s.parent.as_str() {
                "" => String::new(),
                parent => format!(" under {parent}"),
            };
            let short = moved::short(&s.digest);
            format!("{} @{short}{under} from {}", t.name, s.source)
        })
        .collect();
    if !spliced.is_empty() {
        answer.notes.push(format!(
            "plan {}; steps are derived from runs/{}/plan/ on every read, never stored",
            spliced.join(", "),
            loaded.rec.id
        ));
    }
    answer
        .notes
        .extend(loaded.moved.iter().map(moved::Moved::note));
}
