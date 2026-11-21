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
    policy(&mut answer, session, loaded, judged.as_deref());
    if gates {
        detail::add(&mut answer, graph, judged.as_deref());
    }
    if disorder.count() > 0 {
        answer.counts.push(("disorder", disorder.count()));
    }
    super::show_loop::counts(&mut answer, graph);
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

/// Flag the steps whose pinned gate moved (`!policy-moved`, with why) and
/// the open steps the pin locks (`!locked`).
fn policy_rows(answer: &mut Answer, loaded: &Loaded, drift: &super::drift::Drift) {
    for (i, why) in &drift.steps {
        if let Some(row) = answer.rows.get_mut(*i) {
            if !row.flags.iter().any(|f| f == "policy-moved") {
                row.flags.push("policy-moved".into());
            }
            row.more.push(crate::emit::More {
                label: "why",
                text: why.clone(),
                ..crate::emit::More::default()
            });
        }
    }
    for (i, step) in loaded.graph.steps.iter().enumerate() {
        if let (true, false, Some(row)) = (step.locked, step.closed(), answer.rows.get_mut(i)) {
            row.flags.push("locked".into());
            row.extra.push(("locked", serde_json::json!(true)));
        }
    }
}

/// The plan's summary, from its steps, the run's state and its drift.
pub(crate) fn ended(
    session: &Session<'_>,
    loaded: &Loaded,
    judged: Option<&[Vec<judge::Gate>]>,
    drift: &super::drift::Drift,
) -> super::summary::Ended {
    decided(session, loaded, judged, drift).0
}

/// [`ended`], with the kernel's word and how each step closed.
pub(crate) fn decided(
    session: &Session<'_>,
    loaded: &Loaded,
    judged: Option<&[Vec<judge::Gate>]>,
    drift: &super::drift::Drift,
) -> (
    super::summary::Ended,
    writfold_core::policy::Summary,
    Vec<writfold_core::policy::Close>,
) {
    let repo = (loaded.subject.as_ref()).map(|s| (s.top.as_path(), s.name.as_str()));
    let mut names: Vec<String> = Vec::new();
    if let Ok(r) = super::required::find(&session.ws.root, repo) {
        names.extend(r.start.into_iter().chain(r.end));
    }
    let pinned = super::pin::required_in_force(&drift.pins);
    for name in pinned.iter().flat_map(|(_, r)| &r.unskippable) {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }
    let present = super::summary::required_present(&loaded.graph, &names);
    let closed = runs::state(session, &loaded.rec) == "closed";
    super::summary::decide(&loaded.graph, judged, (drift, closed), &present)
}

/// The final `plan:` row of `plan status`.
fn summary_row(
    loaded: &Loaded,
    ended: &super::summary::Ended,
    (drift, closed): (&super::drift::Drift, Option<String>),
) -> crate::emit::Row {
    let text = match ended.word {
        "incomplete" | "closed-incomplete" => {
            let at = loaded.graph.current().map_or(0, |i| i + 1);
            let closed = match (ended.word, closed) {
                ("closed-incomplete", Some(c)) => format!("; closed {c}"),
                _ => String::new(),
            };
            format!("step {at}/{} open; {}{closed}", ended.steps, ended.counts())
        }
        _ => format!("{}; policy={}", ended.counts(), drift.word),
    };
    crate::emit::Row {
        kind: "plan",
        id: ended.shown(),
        text,
        path: Some(format!("{}/plan/", runs::dir(&loaded.rec))),
        section: Some("plan"),
        extra: ended.keys(),
        more: super::repin::lines(&drift.pins),
        ..crate::emit::Row::default()
    }
}

/// The policy parts of `plan status`: drift and lock flags, the summary
/// row, and the trailer's `policy=`, `summary=` and `repinned=`.
fn policy(
    answer: &mut Answer,
    session: &Session<'_>,
    loaded: &Loaded,
    judged: Option<&[Vec<judge::Gate>]>,
) {
    let drift = super::drift::judge(&session.ws.root, loaded);
    policy_rows(answer, loaded, &drift);
    if !loaded.graph.planned() {
        return;
    }
    let ended = ended(session, loaded, judged, &drift);
    let closed = super::closed_note(session, &loaded.rec);
    answer
        .rows
        .push(summary_row(loaded, &ended, (&drift, closed)));
    answer.words.push(("policy", drift.word.into()));
    answer.words.push(("summary", ended.word.into()));
    answer.trailer_json.extend(ended.keys());
    let repins = drift.pins.iter().filter(|p| p.kind == "repin").count();
    answer.counts.push(("repinned", repins));
    answer.flagged |= drift.moved() || ended.flagged();
    answer.notes.extend(drift.note());
}
