//! Ordered step projections. Only `under` links determine nesting.
use crate::cells::State;
use crate::lines::more;
use crate::source::{Reply, field, flags};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Node<'a> {
    pub id: &'a str,
    pub depth: usize,
    pub group: &'a str,
    pub parent: Option<&'a str>,
    pub state: State,
    pub chips: Vec<&'a str>,
    pub rail_text: &'a str,
    pub flags: Vec<&'a str>,
    pub row: &'a Value,
}

pub fn nodes(answer: &Reply) -> Vec<Node<'_>> {
    let mut nodes: Vec<_> = answer
        .rows
        .iter()
        .filter(|row| field(row, "kind") == "step")
        .map(node)
        .collect();
    let parents: HashMap<_, _> = nodes.iter().map(|n| (n.id, n.parent)).collect();
    for node in &mut nodes {
        node.depth = depth(node.parent, &parents);
    }
    nodes
}

fn depth<'a>(mut parent: Option<&'a str>, parents: &HashMap<&str, Option<&'a str>>) -> usize {
    let mut depth = 0;
    // Malformed links cannot recurse or walk beyond the number of rows.
    while depth < parents.len() {
        let Some(next) = parent.and_then(|id| parents.get(id)) else {
            break;
        };
        depth += 1;
        parent = *next;
    }
    depth
}

fn node(row: &Value) -> Node<'_> {
    let flags = flags(row);
    let state = state(field(row, "status"), &flags);
    Node {
        id: field(row, "id"),
        depth: 0,
        group: labelled(row, "splice").map_or("", |value| field(value, "text")),
        parent: labelled(row, "under").map(|value| field(value, "text")),
        state,
        chips: more(row)
            .filter(|value| field(value, "label") == "gate")
            .map(|value| field(value, "status"))
            .collect(),
        rail_text: labelled(row, "done")
            .map_or_else(|| field(row, "text"), |value| field(value, "text")),
        flags,
        row,
    }
}

fn labelled<'a>(row: &'a Value, label: &str) -> Option<&'a Value> {
    more(row).find(|value| field(value, "label") == label)
}

fn state(status: &str, flags: &[&str]) -> State {
    if flags.contains(&"invalid") {
        return State::Invalid;
    }
    match status {
        "done" if flags.contains(&"stale") => State::Stale,
        "done" => State::Done,
        "skipped" => State::Skipped,
        "current" => State::Current,
        "later" => State::Later,
        _ => State::Invalid,
    }
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
