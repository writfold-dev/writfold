//! The fold: a run's plan events into one ordered list of steps. The
//! result is a function of the set of event files alone, never of the
//! order a directory lists them in, nor of any clock:
//!
//! 1. every event is parsed and checked; an invalid one is listed, never
//!    used;
//! 2. splices are sorted by a total key (parent, ordinal, template name,
//!    digest) and each is expanded from its template snapshots (the
//!    template's, and those of the templates it includes);
//! 3. the tree is flattened post-order: a splice's steps go immediately
//!    before the step they were spliced under (the root's go in order), so
//!    a step's children come before it. A step that includes a template
//!    (`use=`) has that template's steps first, as `<step>/<name>#1/…`,
//!    then the splices made under it, in key order;
//! 4. `done-` and `skip-` events close the steps they name.
//!
//! A step's identity is its path, `<template>#<n>/<step>` under the root
//! (`change#1/implement`), longer under a spliced step
//! (`change#1/review/review#1/read-diff`). Each step's
//! window for run-record checks begins at the event that began it: its
//! splice for the first step of a splice, else the event that closed the
//! step before it.
use super::event::{self, Baseline, Done, Event, Skip, Splice};
use super::template::Template;
mod snapshots;
mod tree;
mod windows;
use tree::{Tree, expand};
use windows::windows;

/// A splice nested deeper than this is not expanded.
pub const MAX_DEPTH: usize = writfold_core::plan::MAX_DEPTH;

/// One use of a template under a parent: a splice or a static include.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Use {
    pub parent: String,
    pub template: String,
    pub digest: String,
    pub ordinal: u64,
}

/// How many splices deep a node path is: one per `#`.
pub fn depth(path: &str) -> usize {
    path.matches('#').count()
}

/// One step of the folded list.
#[derive(Clone, Debug)]
pub struct Step {
    pub path: String,
    pub id: String,
    pub gates: Vec<String>,
    /// The template it came from (name, digest and where it was read
    /// from) and the splice file that added it.
    pub template: String,
    pub digest: String,
    pub source: String,
    pub splice: String,
    /// The expanded template use, including static includes.
    pub usage: usize,
    /// Whether that splice is at the root (rather than under a step).
    pub root: bool,
    pub done: Option<(String, Done)>,
    pub skipped: Option<(String, Skip)>,
    /// Where its window begins.
    pub begin: Baseline,
}

impl Step {
    pub fn closed(&self) -> bool {
        self.done.is_some() || self.skipped.is_some()
    }

    /// Where its window ends: the event that closed it, if any.
    pub fn end(&self) -> Option<&Baseline> {
        match (&self.done, &self.skipped) {
            (Some((_, d)), _) => Some(&d.records),
            (None, Some((_, s))) => Some(&s.records),
            (None, None) => None,
        }
    }
}

/// A run's plan, folded.
#[derive(Clone, Debug, Default)]
pub struct Graph {
    /// Valid splices in key order, with their templates.
    pub splices: Vec<(String, Splice, Template)>,
    pub steps: Vec<Step>,
    /// Every template use that was expanded, splices and includes.
    pub uses: Vec<Use>,
    /// Event files that are unreadable, invalid or name no step: (path, why).
    pub invalid: Vec<(String, String)>,
}

impl Graph {
    /// Whether the run has a plan at all.
    pub fn planned(&self) -> bool {
        !self.splices.is_empty()
    }

    /// The kernel's view of the list.
    pub fn nodes(&self) -> Vec<writfold_core::plan::Node> {
        self.steps
            .iter()
            .map(|s| writfold_core::plan::Node {
                done: s.done.is_some(),
                skipped: s.skipped.is_some(),
            })
            .collect()
    }

    /// The current step's index.
    pub fn current(&self) -> Option<usize> {
        writfold_core::plan::current(&self.nodes())
    }

    /// The ordinal the next splice under `parent` takes.
    pub fn next_ordinal(&self, parent: &str) -> u64 {
        let used = self.uses.iter().filter(|u| u.parent == parent);
        let ordinals = self.splices.iter().filter(|(_, s, _)| s.parent == parent);
        used.map(|u| u.ordinal)
            .chain(ordinals.map(|(_, s, _)| s.ordinal))
            .max()
            .unwrap_or(0)
            + 1
    }

    /// Every gate any step names, in first-named order.
    pub fn gates(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for gate in self.steps.iter().flat_map(|s| &s.gates) {
            if !out.contains(gate) {
                out.push(gate.clone());
            }
        }
        out
    }
}

/// Read and fold the events of the run whose directory is `run_dir`.
pub fn read(root: &std::path::Path, run_dir: &str, run: &str) -> Graph {
    let dir = root.join(run_dir).join(event::DIR);
    let files: Vec<(String, String)> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json") && !n.starts_with('.'))
        .map(|n| {
            let text = std::fs::read_to_string(dir.join(&n)).unwrap_or_default();
            (n, text)
        })
        .collect();
    let snapshot = |digest: &str| {
        let hex = digest.strip_prefix("sha256:")?;
        std::fs::read(dir.join(event::TEMPLATES).join(format!("{hex}.toml"))).ok()
    };
    fold(run, &format!("{run_dir}/{}", event::DIR), files, &snapshot)
}

type Snapshot<'a> = &'a dyn Fn(&str) -> Option<Vec<u8>>;

/// Fold event files `(name, text)` filed under `dir` (KB-relative).
/// `snapshot` returns a template snapshot's bytes by digest.
pub fn fold(run: &str, dir: &str, files: Vec<(String, String)>, snapshot: Snapshot<'_>) -> Graph {
    let mut graph = Graph::default();
    let mut splices = Vec::new();
    let mut closes: Vec<(String, Event)> = Vec::new();
    let mut library: Vec<Template> = Vec::new();
    for (name, text) in files {
        let path = format!("{dir}/{name}");
        match event::parse(&text, run, &name) {
            Err(why) => graph.invalid.push((path, why)),
            Ok(Event::Splice(s)) => match snapshots::load(&s, snapshot, &mut library) {
                Ok(t) => splices.push((path, s, t)),
                Err(why) => graph.invalid.push((path, why)),
            },
            Ok(other) => closes.push((path, other)),
        }
    }
    splices.sort_by(|a, b| {
        let key = |s: &Splice| {
            (
                s.parent.clone(),
                s.ordinal,
                s.template.clone(),
                s.digest.clone(),
            )
        };
        (key(&a.1), &a.0).cmp(&(key(&b.1), &b.0))
    });
    let mut tree = Tree {
        splices: &splices,
        library: &library,
        used: vec![false; splices.len()],
        steps: Vec::new(),
        uses: Vec::new(),
    };
    expand(&mut tree, "", 0);
    let (used, steps, uses) = (tree.used, tree.steps, tree.uses);
    graph.steps = steps;
    graph.uses = uses;
    for (i, (path, s, _)) in splices.iter().enumerate() {
        if !used[i] {
            let why = format!(
                "splices under {}, which is not a step of this plan",
                s.parent
            );
            graph.invalid.push((path.clone(), why));
        }
    }
    graph.splices = splices;
    close(&mut graph, closes);
    windows(&mut graph);
    graph.invalid.sort();
    graph
}

fn close(graph: &mut Graph, closes: Vec<(String, Event)>) {
    for (path, e) in closes {
        let node = match &e {
            Event::Done(d) => &d.node,
            Event::Skip(s) => &s.node,
            Event::Splice(_) => continue,
        };
        let Some(step) = graph.steps.iter_mut().find(|s| &s.path == node) else {
            let why = format!("names {node}, which is not a step of this plan");
            graph.invalid.push((path, why));
            continue;
        };
        match e {
            Event::Done(d) => step.done = Some((path, d)),
            Event::Skip(s) => step.skipped = Some((path, s)),
            Event::Splice(_) => {}
        }
    }
}
