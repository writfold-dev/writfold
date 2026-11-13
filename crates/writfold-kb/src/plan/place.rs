//! Where `plan use` splices a template on a planned run: before the
//! current step as its children, at the next ordinal under it, or nowhere
//! when the template is already spliced at that digest under the step,
//! one of its ancestors or the root (idempotence; `--again` skips the
//! lookup). A required end template the plan lacks goes at the root's end.
use super::fold::{self, Graph};
use super::recheck::Decided;
use super::required::{self, Required};
use super::splice::{Planned, shown, too_deep};
use super::template::Closure;
use crate::error::{KbError, Result};
use writfold_core::plan::{SpliceRefusal, Spliced, splice_check};

/// The node paths a splice at `at` is idempotent against: `at`, each of
/// its ancestors, and the root.
pub fn ancestors(at: &str) -> Vec<&str> {
    let mut out = vec![""];
    for (i, c) in at.char_indices() {
        if c == '/' {
            out.push(&at[..i]);
        }
    }
    if !at.is_empty() {
        out.push(at);
    }
    out
}

/// The kernel's key of a template use: `<name>@<digest>`.
fn key(name: &str, digest: &str) -> Vec<u8> {
    format!("{name}@{digest}").into_bytes()
}

/// What the write re-checks under the lock for a splice placed on `graph`.
pub fn decided(graph: &Graph, planned: &Planned) -> Decided {
    let current = graph.current().and_then(|i| graph.steps.get(i));
    Decided::Splice {
        parent: planned.parent.clone(),
        ordinal: planned.ordinal,
        current: current.is_some_and(|s| s.path == planned.parent),
    }
}

/// Where `plan use` splices `closure` on a planned run, or the note of an
/// idempotent repeat.
pub fn place(
    graph: &Graph,
    required: &Required,
    closure: Closure,
    again: bool,
) -> Result<std::result::Result<Planned, String>> {
    let name = closure.top.name.clone();
    if required::missing_end(required, graph).contains(&name) {
        // A required end template the plan lacks goes at the root's end.
        let ordinal = graph.next_ordinal("");
        return Ok(Ok(Planned {
            parent: String::new(),
            ordinal,
            closure,
            again,
        }));
    }
    let parent = graph
        .current()
        .map_or_else(String::new, |i| graph.steps[i].path.clone());
    let under = ancestors(&parent);
    let spliced: Vec<(&fold::Use, Vec<u8>)> = graph
        .uses
        .iter()
        .filter(|u| !again && under.contains(&u.parent.as_str()))
        .map(|u| (u, key(&u.template, &u.digest)))
        .collect();
    let keys: Vec<Vec<u8>> = spliced.iter().map(|(_, k)| k.clone()).collect();
    let mine = key(&name, &closure.top.digest);
    let depth = fold::depth(&parent) + closure.height;
    match splice_check(&graph.nodes(), &keys, &mine, depth) {
        Ok(Spliced::Already) => {
            let at = spliced
                .iter()
                .find(|(_, k)| *k == mine)
                .map_or("", |(u, _)| u.parent.as_str());
            Ok(Err(format!(
                "{name} is already spliced under {} at this digest; nothing written (--again adds another)",
                shown(at)
            )))
        }
        Ok(Spliced::Insert(_)) => Ok(Ok(Planned {
            ordinal: graph.next_ordinal(&parent),
            parent,
            closure,
            again,
        })),
        Err(SpliceRefusal::TooDeep) => Err(too_deep(&name, &parent)),
        Err(SpliceRefusal::Complete) => Err(KbError::Refused(
            "every step is done or skipped: there is no current step to splice under".into(),
        )),
    }
}
