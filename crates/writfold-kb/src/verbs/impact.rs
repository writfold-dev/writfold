//! `impact <id> [--depth N] [--direction in|out|both] [--include inferred]`:
//! what does changing X touch. A bounded breadth-first walk ([`crate::walk`])
//! over authored, field and citation edges whose status is explicit or
//! confirmed; mentions and inferred links only with `--include inferred`;
//! retired and disputed edges never, but counted. Project records are homes,
//! not dependencies: they are reached but not walked through.
use super::{BOUNDARY, ambiguous, edge_flags, node_row, unknown};
use crate::emit::{Answer, Walked};
use crate::error::Result;
use crate::index::query::{self, Edge, Resolved};
use crate::session::Session;
use writfold_core::walk::{Arc, Follow, walk};

/// Which way edges are followed from a node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    /// From source to destination: what X points at.
    Out,
    /// From destination to source: what points at X.
    In,
    Both,
}

impl Direction {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "out" => Some(Self::Out),
            "in" => Some(Self::In),
            "both" => Some(Self::Both),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Out => "out",
            Self::In => "in",
            Self::Both => "both",
        }
    }
}

/// The skip reasons the trailer always states.
pub const SKIPS: [&str; 4] = ["retired", "disputed", "inferred", "other"];

/// Whether the walk crosses an edge.
pub fn follow(edge: &Edge, inferred: bool) -> Follow {
    match (edge.status.as_str(), edge.origin.as_str()) {
        ("retired", _) => Follow::Skip("retired"),
        ("disputed", _) => Follow::Skip("disputed"),
        ("inferred", _) | (_, "mention") if inferred => Follow::Yes,
        ("inferred", _) | (_, "mention") => Follow::Skip("inferred"),
        ("explicit" | "confirmed", _) => Follow::Yes,
        _ => Follow::Skip("other"),
    }
}

/// The edges a graph answer may read before it stops and says so (#60):
/// `WRITFOLD_KB_WORK`, else 100,000. Depth and the output budget do not
/// bound the work a hub costs; this does.
pub fn work() -> usize {
    std::env::var("WRITFOLD_KB_WORK")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n: &usize| *n > 0)
        .unwrap_or(100_000)
}

/// Up to `room` of a node's edges in the walk's direction, in key order,
/// and whether more were left unread.
pub fn arcs(
    session: &Session<'_>,
    node: &str,
    direction: Direction,
    room: usize,
) -> Result<(Vec<Arc<Edge>>, bool)> {
    let mut out = Vec::new();
    let mut more = false;
    for outgoing in [true, false] {
        if (outgoing && direction == Direction::In) || (!outgoing && direction == Direction::Out) {
            continue;
        }
        let (edges, left) = query::edges_within(
            &session.conn,
            node,
            outgoing,
            room.saturating_sub(out.len()),
        )?;
        more = more || left;
        for edge in edges {
            let other = if outgoing { &edge.dst } else { &edge.src };
            out.push(Arc {
                other: other.clone(),
                key: format!(
                    "{}\0{}\0{}\0{}\0{}\0{}",
                    edge.src, edge.rel, edge.dst, edge.origin, edge.path, edge.line
                ),
                edge,
            });
        }
    }
    Ok((out, more))
}

/// A node the walk reaches but does not expand.
pub fn terminal(node: &str) -> bool {
    node.starts_with("project:")
}

pub fn impact(
    session: &Session<'_>,
    target: &str,
    depth: usize,
    direction: Direction,
    inferred: bool,
) -> Result<Answer> {
    let start = if target.starts_with("code:")
        && !query::edges_within(&session.conn, target, false, 1)?
            .0
            .is_empty()
    {
        target.to_owned()
    } else {
        match query::resolve(&session.conn, target)? {
            Resolved::One(rec) => rec.id,
            Resolved::Many(candidates) => return Ok(ambiguous(session, target, &candidates)),
            Resolved::None => return unknown(session, "unresolved-id", target, &[]),
        }
    };
    // The walk takes an infallible neighbour function; the first index
    // error is kept and returned after it.
    let mut failure = None;
    let limit = work();
    let result = walk(
        &start,
        (depth, limit),
        |node, room| {
            arcs(session, node, direction, room).unwrap_or_else(|error| {
                failure.get_or_insert(error);
                (Vec::new(), true)
            })
        },
        |edge| follow(edge, inferred),
        terminal,
    );
    if let Some(error) = failure {
        return Err(error);
    }
    let mut answer = Answer::default();
    let mut stale = 0;
    for reached in &result.reached {
        let edge = &reached.edge;
        let mut row = node_row(session, &reached.node)?;
        // The title the row would print elsewhere; a record without one
        // labels itself by its ID, which is not a title.
        row.title = Some(std::mem::take(&mut row.text)).filter(|t| !t.is_empty() && *t != row.id);
        row.edge = Some(Walked {
            from: edge.src.clone(),
            rel: edge.rel.clone(),
            to: edge.dst.clone(),
            origin: edge.origin.clone(),
            depth: reached.depth,
        });
        row.text = format!(
            "depth={} {} {} {} ({})",
            reached.depth, edge.src, edge.rel, edge.dst, edge.origin
        );
        row.path = Some(edge.path.clone());
        row.line = Some(edge.line);
        let freshness = edge_flags(session, edge)?;
        stale += usize::from(!freshness.is_empty());
        row.flags.extend(freshness);
        if follow(edge, false) != Follow::Yes {
            row.flags.push("inferred".into());
        }
        answer.rows.push(row);
    }
    answer.counts.push(("frontier", result.frontier));
    for reason in SKIPS {
        let count = result.skipped.get(reason).copied().unwrap_or(0);
        if reason != "other" || count > 0 {
            answer.counts.push((skip_label(reason), count));
        }
    }
    if stale > 0 {
        answer.counts.push(("stale", stale));
    }
    answer.counts.push(("examined", result.examined));
    answer.partial = !result.complete();
    if result.truncated {
        answer.notes.push(format!(
            "the walk stopped at its work limit of {limit} edges (WRITFOLD_KB_WORK): what is shown is reached, but not everything reachable is shown; narrow --depth or --direction"
        ));
    }
    answer.notes.push(format!(
        "walked from {start}: depth {depth}, direction {}{}; project records are homes and are not walked through",
        direction.name(),
        if inferred { ", inferred edges included" } else { "" }
    ));
    if answer.rows.is_empty() {
        answer.notes.push(BOUNDARY.to_owned());
    }
    Ok(answer)
}

fn skip_label(reason: &str) -> &'static str {
    match reason {
        "retired" => "skipped_retired",
        "disputed" => "skipped_disputed",
        "inferred" => "skipped_inferred",
        _ => "skipped_other",
    }
}
