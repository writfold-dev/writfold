//! The `step:` lines `deliver status` adds under its candidates: the
//! candidate a current step is judged on, each one a done record binds,
//! and each older draft that decides no step because a newer candidate of
//! its mode is bound.
use super::delivery::{Deliveries, line};
use crate::deliver::step::{self, PREFIX};
use crate::emit::Row;
use crate::index::query::Rec;

/// The `step:` line `deliver status` adds under a candidate a step is
/// judged on: the current step's deciding candidate, and each candidate a
/// done record binds. Other candidates are unchanged.
pub fn mark(root: &std::path::Path, rec: &Rec, rows: &mut [Row]) {
    let run_dir = crate::runs::dir(rec);
    let graph = super::fold::read(root, &run_dir, &rec.id);
    let d = Deliveries::read(root, &run_dir, &rec.id, &graph);
    let current = graph.current();
    let mut said: Vec<(String, String)> = Vec::new();
    for (i, s) in graph.steps.iter().enumerate() {
        let done = s.done.is_some();
        if !done && current != Some(i) {
            continue;
        }
        for mode in s.gates.iter().filter_map(|g| step::mode_of(g)) {
            let Some(c) = d.decides(s, mode) else {
                continue;
            };
            let text = match done {
                true => format!("bound to {} [done]", s.path),
                false => format!("decides {PREFIX}{mode} at {} [current]", s.path),
            };
            said.push((c.id.clone(), text));
        }
    }
    said.extend(shadowed(&d, &bound_steps(root, &run_dir, &rec.id)));
    for row in rows.iter_mut().filter(|r| r.kind == "delivery") {
        for (_, text) in said.iter().filter(|(id, _)| *id == row.id) {
            row.more.push(line("step", text.clone(), None));
        }
    }
}

/// Each candidate a done record of the run binds, with the step's path.
pub fn bound_steps(root: &std::path::Path, run_dir: &str, run: &str) -> Vec<(String, String)> {
    let graph = super::fold::read(root, run_dir, run);
    let mut out = Vec::new();
    for s in &graph.steps {
        let Some((_, done)) = &s.done else { continue };
        for m in done.met.iter().filter(|m| step::mode_of(&m.gate).is_some()) {
            if let Some(id) = m
                .evidence
                .as_deref()
                .and_then(|e| step::bound_id(run_dir, e))
            {
                out.push((id.to_owned(), s.path.clone()));
            }
        }
    }
    out
}

/// Each live, unbound candidate older than a bound one of its mode: it
/// decides no step ([`writfold_core::delivery::deciding`]).
fn shadowed(d: &Deliveries, steps: &[(String, String)]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut modes: Vec<&str> = Vec::new();
    for c in &d.cands {
        if let Ok(m) = &c.manifest
            && !modes.contains(&m.mode.as_str())
        {
            modes.push(&m.mode);
        }
    }
    for mode in modes {
        let mine = step::of_mode(&d.cands, mode);
        let places = step::places(&mine, &d.bound, &d.withdrawn);
        for (i, c) in mine.iter().enumerate() {
            if places[i] != writfold_core::delivery::Place::Open {
                continue;
            }
            let newer = (i + 1..mine.len())
                .rev()
                .find(|&j| places[j] == writfold_core::delivery::Place::Bound);
            let Some(j) = newer else { continue };
            let Some((_, at)) = steps.iter().find(|(id, _)| *id == mine[j].id) else {
                continue;
            };
            out.push((
                c.id.clone(),
                format!(
                    "decides no step: {}, a newer {mode} candidate, is bound to {at} [done]",
                    mine[j].id
                ),
            ));
        }
    }
    out
}
