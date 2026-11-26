//! `runs [--open] [--project P] [--plan]` and `resume [<run>]`.
//!
//! `runs` lists every run, newest activity first, with its effective state
//! (the Writfold sidecar's `closed`, else the manifest's), where the work
//! happens and its last checkpoint. `resume` shows one run: the run, its
//! coverage counts, the items needing a recheck, the state of each defined gate
//! (`writfold kb gate`), the newest checkpoint entry, and the local
//! drafts and pending proposals of the run (or of no run). With no run
//! named it takes the current-run pointer, then the newest run of the
//! project inferred from the working directory, then the newest open run.
//! No liveness is claimed: there is no daemon.
mod idle;
mod packet;
mod plan;

pub use idle::{age as idle_age, idle};
pub use packet::packet;

use super::{BOUNDARY, brief, record_row};
use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::index::query::{self, Rec, Resolved};
use crate::runs;
use crate::session::Session;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use writfold_core::runs::{Shown, shown};

pub fn list(
    session: &Session<'_>,
    open: bool,
    project: Option<&str>,
    plan: bool,
) -> Result<Answer> {
    let mut answer = Answer::default();
    let current = runs::current_of(session);
    let (mut planned, mut unknown) = (0, 0);
    for (rec, at) in runs::all(session)? {
        let state = runs::state(session, &rec);
        let shown = shown(runs::classify(&state), open);
        if shown == Shown::Hidden || project.is_some_and(|p| rec.project.as_deref() != Some(p)) {
            continue;
        }
        let (row, has_plan) = run_row(session, (&rec, &at, state), current.as_deref(), plan);
        planned += usize::from(has_plan);
        unknown += usize::from(shown == Shown::Flagged);
        answer.rows.push(row);
    }
    if plan {
        answer.counts.push(("planned", planned));
    }
    if unknown > 0 {
        answer.counts.push(("vocab_unknown", unknown));
        answer.notes.push(format!(
            "{unknown} run(s) with a state other than open or closed; only closed is left out by --open"
        ));
    }
    runs::inventory(session, &mut answer)?;
    if answer.rows.is_empty() {
        answer.unknown = Some("no-record");
        answer.notes.push(BOUNDARY.into());
        answer.notes.push(start_note());
    }
    Ok(answer)
}

/// One run's `runs` row: state, place, plan position (with `plan`) and
/// last checkpoint; and whether it has a plan position.
pub(crate) fn run_row(
    session: &Session<'_>,
    (rec, at, state): (&Rec, &str, String),
    current: Option<&str>,
    plan: bool,
) -> (Row, bool) {
    let mut row = record_row(session, rec);
    row.kind = "run";
    // A closed run whose plan still has an open step (policy spec 2.6).
    if state == "closed" && plan::open(&session.ws.root, rec) {
        row.flags.push("incomplete".into());
    }
    if shown(runs::classify(&state), false) == Shown::Flagged {
        row.flags.push("vocab=unknown".into());
    }
    row.status = Some(state);
    if current == Some(rec.id.as_str()) {
        row.flags.push("current".into());
    }
    let mut place = Vec::new();
    if let Some(p) = &rec.project {
        place.push(format!("project={p}"));
    }
    if let Some((_, map)) = runs::sidecar(session, rec) {
        for key in runs::PLACE {
            if let Some(v) = map.get(key).and_then(Value::as_str) {
                let v = if key == "base" {
                    v.get(..12).unwrap_or(v)
                } else {
                    v
                };
                place.push(format!("{key}={v}"));
            }
        }
    }
    // A run with no project, place or base has no field line (#97).
    if !place.is_empty() {
        row.more.push(More {
            label: "field",
            text: place.join(" "),
            ..More::default()
        });
    }
    let mut planned = false;
    if plan && let Some(position) = plan::position(&session.ws.root, rec) {
        row.more.push(position);
        planned = true;
    }
    let (path, entries) = runs::entries(session, rec);
    if let Some(entry) = entries.last() {
        let first = session
            .lines(&path, entry.first + 1, entry.last)
            .into_iter()
            .find(|l| !l.trim().is_empty())
            .unwrap_or_default();
        row.more.push(More {
            label: "checkpoint",
            text: format!("{at} {}", first.trim()),
            path: Some(path),
            line: Some(entry.first),
            ..More::default()
        });
    }
    (row, planned)
}

fn start_note() -> String {
    "start a run: writfold kb checkpoint --new-run --project P --task T --note TEXT".into()
}

/// The run `resume` shows, or why there is none.
fn pick(
    session: &Session<'_>,
    named: Option<&str>,
    cwd: &Path,
) -> Result<std::result::Result<Rec, Answer>> {
    if let Some(name) = named {
        return Ok(match query::resolve(&session.conn, name)? {
            Resolved::One(rec) if rec.kind == "run" => Ok(*rec),
            Resolved::Many(found) => Err(super::ambiguous(session, name, &found)),
            _ => Err(super::unknown(session, "no-record", name, &["run"])?),
        });
    }
    if let Some(id) = runs::current_of(session)
        && let Some(rec) = query::record(&session.conn, &id)?
    {
        return Ok(Ok(rec));
    }
    let project = brief::infer(session, cwd)?;
    let all = runs::all(session)?;
    let by_project = all
        .iter()
        .find(|(r, _)| project.is_some() && r.project == project);
    let open = all
        .iter()
        .find(|(r, _)| runs::state(session, r) != "closed");
    match by_project.or(open) {
        Some((rec, _)) => Ok(Ok(rec.clone())),
        None => {
            let mut answer = Answer {
                unknown: Some("no-record"),
                ..Answer::default()
            };
            answer.notes.push(BOUNDARY.into());
            answer.notes.push(start_note());
            Ok(Err(answer))
        }
    }
}

/// `resume <run> --here`: point the seat's pointers at an open run, then
/// answer as `resume`. A run that is closed or not in the KB is refused:
/// the seat may take a run exactly when `run.current` would then be met
/// ([`writfold_core::guard::require::run_current`]).
pub fn attach(session: &Session<'_>, named: &str, cwd: &Path) -> Result<(Answer, String)> {
    use crate::error::KbError;
    use writfold_core::guard::{Fact, require::run_current};
    let rec = match pick(session, Some(named), cwd)? {
        Ok(rec) => rec,
        Err(answer) => return Ok((answer, named.to_owned())),
    };
    let open = runs::state(session, &rec) != "closed";
    if !matches!(run_current(true, open), Fact::Met) {
        return Err(KbError::Refused(format!(
            "run {} is closed; reopen it with status {} --set open",
            rec.id, rec.id
        )));
    }
    let Some(repo) = &session.ws.repo else {
        return Err(KbError::Refused(
            "the current-run pointers need a KB under Git".into(),
        ));
    };
    let seat = &session.ws.seat;
    crate::seat::make_current(&repo.git_dir.join("writfold"), seat, &rec.id)?;
    let (mut answer, id) = resume(session, Some(&rec.id), cwd)?;
    answer.notes.push(format!(
        "{} is now the current run here ({}); local pointers only, nothing to commit",
        rec.id,
        seat.describe()
    ));
    Ok((answer, id))
}

pub fn resume(session: &Session<'_>, named: Option<&str>, cwd: &Path) -> Result<(Answer, String)> {
    let rec = match pick(session, named, cwd)? {
        Ok(rec) => rec,
        Err(answer) => return Ok((answer, named.unwrap_or_default().to_owned())),
    };
    let mut answer = Answer::default();
    let mut rows = runs::rows(session, &rec)?;
    let tail = (rows.len() > 1).then(|| rows.remove(1));
    answer.rows.extend(rows);
    answer
        .rows
        .extend(crate::plan::resume_row(session, &rec, cwd));
    answer.rows.extend(coverage(session, &rec));
    let observed = crate::observe::summary(&session.ws.root, &runs::dir(&rec));
    answer.notes.extend(observed);
    let (gates, notes) = crate::gate::resume_rows(session, &rec, cwd);
    answer.rows.extend(gates);
    answer.notes.extend(notes);
    let (deliveries, notes) = crate::deliver::resume_rows(session, &rec);
    answer.rows.extend(deliveries);
    answer.notes.extend(notes);
    (answer.rows).extend(crate::agents::shown::rows(&session.ws.root, &rec));
    answer
        .rows
        .extend(crate::write::artifact::rows(session, &rec)?);
    answer.rows.extend(tail);
    answer.rows.extend(drafts(session, &rec));
    answer.rows.extend(crate::proposal::rows(session, &rec.id));
    if runs::state(session, &rec) == "closed" {
        answer.notes.push(format!("{} is closed", rec.id));
    }
    let top = crate::defs::lint::enclosing(&session.ws.root, cwd);
    if !crate::gate::policy::configured(&session.ws.root, top.as_deref()) {
        answer
            .notes
            .push(crate::gate::policy::NONE_CONFIGURED.into());
    }
    Ok((answer, rec.id))
}

/// The unconfirmed local drafts of this run, or of no run, marked `!draft`.
fn drafts(session: &Session<'_>, rec: &Rec) -> Vec<Row> {
    let Some(repo) = &session.ws.repo else {
        return Vec::new();
    };
    let state = repo.git_dir.join("writfold");
    crate::draft::all(&state)
        .into_iter()
        .filter(|(_, d)| crate::draft::text(d, "run").is_none_or(|run| run == rec.id))
        .map(|(id, d)| {
            let first = crate::draft::body(&d)
                .and_then(|b| b.lines().find(|l| !l.trim().is_empty()).map(str::to_owned))
                .unwrap_or_else(|| "(no text yet)".into());
            Row {
                kind: "draft",
                status: Some(
                    if d.contains_key("ended") {
                        "ended"
                    } else {
                        "open"
                    }
                    .into(),
                ),
                text: first.trim().chars().take(120).collect(),
                path: crate::draft::path(&state, &id)
                    .ok()
                    .map(|p| p.display().to_string()),
                flags: vec!["draft".into()],
                more: vec![More {
                    label: "confirm",
                    text: format!("writfold kb checkpoint --from-draft {id}"),
                    ..More::default()
                }],
                id,
                ..Row::default()
            }
        })
        .collect()
}

/// Outcome counts from `coverage.json`, then one row per item whose
/// outcome is `needs-recheck`.
fn coverage(session: &Session<'_>, rec: &Rec) -> Vec<Row> {
    let path = format!("{}/coverage.json", runs::dir(rec));
    let Some(text) = session.text(&path) else {
        return Vec::new();
    };
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&text) else {
        return vec![Row {
            kind: "coverage",
            id: rec.id.clone(),
            text: "unreadable coverage.json".into(),
            path: Some(path),
            flags: vec!["unverifiable".into()],
            ..Row::default()
        }];
    };
    let entries = map
        .get("entries")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut recheck = Vec::new();
    for (item, entry) in &entries {
        let outcome = entry
            .get("outcome")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        *counts.entry(outcome.to_owned()).or_default() += 1;
        if outcome == "needs-recheck" {
            recheck.push(item.clone());
        }
    }
    let summary: Vec<String> = counts.iter().map(|(k, n)| format!("{k}={n}")).collect();
    let mut rows = vec![Row {
        kind: "coverage",
        id: rec.id.clone(),
        text: if summary.is_empty() {
            "no entries".into()
        } else {
            summary.join(" ")
        },
        path: Some(path.clone()),
        line: Some(1),
        ..Row::default()
    }];
    for item in recheck {
        rows.push(Row {
            kind: "recheck",
            line: Some(crate::parse::json::line_of(&text, &item)),
            id: item,
            text: "needs-recheck".into(),
            path: Some(path.clone()),
            flags: vec!["stale".into()],
            ..Row::default()
        });
    }
    rows
}
