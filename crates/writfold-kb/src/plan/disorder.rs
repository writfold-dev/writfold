//! Disorder a merge of two clones can leave in a plan. No lock spans
//! clones, so the re-check ([`super::recheck`]) cannot prevent it; the
//! fold stays a function of the files and the current step stays the
//! first open one, so nothing here changes either. It is only shown:
//!
//! - `!out-of-order`: a done step with an open step before it (a late
//!   splice under a step another clone closed meanwhile);
//! - `!two-plans`: two root splices at one ordinal (a plan set in each of
//!   two clones);
//! - `!repeated`: a template spliced again, at the same digest and without
//!   `--again`, under a step whose ancestor chain already holds it in a way
//!   one clone's idempotence lookup would have seen. An `--again` splice
//!   is an intended repeat and never flagged.
use super::event::{self, Splice};
use super::fold::Graph;
use super::place::ancestors;
use crate::emit::{More, Row};

/// The flags of each step (by index in the fold) and the notes that
/// explain them.
#[derive(Debug, Default)]
pub struct Disorder {
    pub flags: Vec<Vec<&'static str>>,
    pub notes: Vec<String>,
    pub why: Vec<Option<String>>,
}

impl Disorder {
    /// Every flag raised, once each, in a fixed order.
    pub fn kinds(&self) -> Vec<&'static str> {
        ["out-of-order", "two-plans", "repeated"]
            .into_iter()
            .filter(|k| self.flags.iter().any(|f| f.contains(k)))
            .collect()
    }

    /// How many steps carry a flag.
    pub fn count(&self) -> usize {
        self.flags.iter().filter(|f| !f.is_empty()).count()
    }

    /// Add the flags (and a `why` line) to the step rows, which come first
    /// and in fold order.
    pub fn mark(&self, rows: &mut [Row]) {
        for ((row, flags), why) in rows.iter_mut().zip(&self.flags).zip(&self.why) {
            row.flags.extend(flags.iter().map(|f| (*f).to_owned()));
            if let Some(why) = why {
                row.more.push(More {
                    label: "why",
                    text: why.clone(),
                    ..More::default()
                });
            }
        }
    }
}

/// Find every disorder in `graph`.
pub fn find(graph: &Graph) -> Disorder {
    let n = graph.steps.len();
    let mut out = Disorder {
        flags: vec![Vec::new(); n],
        notes: Vec::new(),
        why: vec![None; n],
    };
    out_of_order(graph, &mut out);
    two_plans(graph, &mut out);
    repeated(graph, &mut out);
    out
}

/// At most three names, then how many more.
fn named(paths: &[&str]) -> String {
    let mut shown = paths.iter().take(3).copied().collect::<Vec<_>>().join(", ");
    if paths.len() > 3 {
        shown.push_str(&format!(" and {} more", paths.len() - 3));
    }
    shown
}

fn out_of_order(graph: &Graph, out: &mut Disorder) {
    let mut open: Vec<&str> = Vec::new();
    let mut flagged = 0;
    for (i, step) in graph.steps.iter().enumerate() {
        if !step.closed() {
            open.push(&step.path);
            continue;
        }
        if step.done.is_none() || open.is_empty() {
            continue;
        }
        flagged += 1;
        out.flags[i].push("out-of-order");
        out.why[i] = Some(format!(
            "done, but {} before it {} open: do or skip {} (plan advance, or plan skip --reason --by)",
            named(&open),
            if open.len() == 1 { "is" } else { "are" },
            if open.len() == 1 { "it" } else { "them" },
        ));
    }
    if flagged > 0 {
        out.notes.push(format!(
            "{flagged} done step(s) follow open steps, as a merge of two clones leaves when one spliced under a step the other closed: the plan is still usable (the current step is the first open one); do or skip the open steps"
        ));
    }
}

/// Mark every step that came from one of `splices` (event paths).
fn mark_from(graph: &Graph, out: &mut Disorder, splices: &[&str], flag: &'static str) {
    for (i, step) in graph.steps.iter().enumerate() {
        if splices.contains(&step.splice.as_str()) && !out.flags[i].contains(&flag) {
            out.flags[i].push(flag);
        }
    }
}

fn two_plans(graph: &Graph, out: &mut Disorder) {
    let roots: Vec<&(String, Splice, _)> = graph
        .splices
        .iter()
        .filter(|(_, s, _)| s.parent.is_empty())
        .collect();
    let mut ordinals: Vec<u64> = roots.iter().map(|(_, s, _)| s.ordinal).collect();
    ordinals.sort_unstable();
    ordinals.dedup();
    for ordinal in ordinals {
        let at: Vec<&(String, Splice, _)> = roots
            .iter()
            .copied()
            .filter(|(_, s, _)| s.ordinal == ordinal)
            .collect();
        if at.len() < 2 {
            continue;
        }
        let paths: Vec<&str> = at.iter().map(|(p, _, _)| p.as_str()).collect();
        mark_from(graph, out, &paths, "two-plans");
        let names: Vec<&str> = at.iter().map(|(_, s, _)| s.template.as_str()).collect();
        out.notes.push(format!(
            "two plans at root ordinal {ordinal} ({}), as a merge of two clones that each set a plan leaves: their steps fold by template name; do the steps you want and skip the others (plan skip --reason --by)",
            names.join(", ")
        ));
    }
}

/// The ordinal of the branch under `ancestor` that holds `path`: the
/// `<template>#<n>` segment right below it.
fn branch(ancestor: &str, path: &str) -> Option<u64> {
    let rel = match ancestor {
        "" => path,
        _ => path.strip_prefix(ancestor)?.strip_prefix('/')?,
    };
    let segment = rel.split('/').next()?;
    segment.rsplit_once('#')?.1.parse().ok()
}

/// Whether one clone writing `later` without `--again` would have found
/// `earlier` already spliced (so only a merge put both there).
fn seen_before(earlier: &Splice, later: &Splice) -> bool {
    if earlier.template != later.template || earlier.digest != later.digest {
        return false;
    }
    if !ancestors(&later.parent).contains(&earlier.parent.as_str()) {
        return false;
    }
    match earlier.parent == later.parent {
        true => earlier.ordinal < later.ordinal,
        false => branch(&earlier.parent, &later.parent).is_some_and(|b| b >= earlier.ordinal),
    }
}

fn repeated(graph: &Graph, out: &mut Disorder) {
    for (path, later, _) in graph.splices.iter().filter(|(_, s, _)| !s.again) {
        let Some((_, earlier, _)) = graph
            .splices
            .iter()
            .find(|(p, s, _)| p != path && seen_before(s, later))
        else {
            continue;
        };
        mark_from(graph, out, &[path.as_str()], "repeated");
        out.notes.push(format!(
            "{} is spliced as {} and again as {} without --again, as a merge of two clones leaves: skip the steps of the copy you do not need (plan skip --reason --by)",
            later.template,
            event::prefix(&earlier.parent, &earlier.template, earlier.ordinal),
            event::prefix(&later.parent, &later.template, later.ordinal),
        ));
    }
}
