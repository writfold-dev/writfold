//! How `resume` and `runs` show a run's invocations (#148): each with its
//! derived state ([`super::invocation::state`]); an invocation with no
//! report reads `prepared`, never done. Rendering only; tested.
use super::invocation::{self, Found};
use crate::deliver::{candidate, withdraw};
use crate::emit::{More, Row};
use crate::index::query::Rec;
use crate::runs;
use std::path::Path;

/// Every invocation of `rec` with its state, oldest first.
fn states(root: &Path, rec: &Rec) -> Vec<(Found, &'static str)> {
    let run_dir = runs::dir(rec);
    let found = invocation::list(root, &run_dir, &rec.id);
    let cands = candidate::list(root, &run_dir, &rec.id);
    let withdrawn = withdraw::ids(root, &cands, &rec.id);
    let words: Vec<&'static str> = (found.iter())
        .map(|f| match &f.record {
            Ok(_) => invocation::state(&found, &cands, &withdrawn, &f.id),
            Err(_) => "unreadable",
        })
        .collect();
    found.into_iter().zip(words).collect()
}

/// The rows `resume` shows: the run's newest three invocations.
pub fn rows(root: &Path, rec: &Rec) -> Vec<Row> {
    let all = states(root, rec);
    let skip = all.len().saturating_sub(3);
    (all.into_iter().skip(skip))
        .map(|(f, state)| {
            let text = match &f.record {
                Ok(r) => format!("{} at {}", r.agent, r.step),
                Err(why) => why.clone(),
            };
            let mut row = Row {
                kind: "invocation",
                id: f.id.clone(),
                status: Some(state.to_owned()),
                text,
                path: Some(f.path.clone()),
                line: Some(1),
                ..Row::default()
            };
            if state == "prepared" {
                row.more.push(More {
                    label: "next",
                    text: format!(
                        "no report yet; record it with writfold kb deliver prepare --mode MODE --from REPORT --invocation {}",
                        f.id
                    ),
                    ..More::default()
                });
            }
            row
        })
        .collect()
}

/// The run row's line: how many invocations are in each state.
pub fn summary(root: &Path, rec: &Rec) -> Option<More> {
    let all = states(root, rec);
    if all.is_empty() {
        return None;
    }
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for (_, state) in &all {
        match counts.iter_mut().find(|(s, _)| s == state) {
            Some((_, n)) => *n += 1,
            None => counts.push((state, 1)),
        }
    }
    let text: Vec<String> = counts.iter().map(|(s, n)| format!("{n} {s}")).collect();
    Some(More {
        label: "invocations",
        text: text.join(", "),
        ..More::default()
    })
}
