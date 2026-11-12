//! The bounded breadth-first walk behind `impact`, as a pure function over
//! a neighbour function: no IO, no index. It decides which nodes are
//! reached (and by which edge first), which edges were skipped and why, and
//! the frontier: followable edges from the last level that lead to nodes
//! not visited. `coverage=complete` may be claimed only when both the
//! frontier and the skipped counts are zero.
use std::collections::{BTreeMap, BTreeSet};

/// Whether the walk may cross an edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Follow {
    Yes,
    /// Not followed; counted under this reason.
    Skip(&'static str),
}

/// One edge seen from a node: the node at its other end, a key naming the
/// edge (so an edge seen from both ends counts once), and the caller's
/// payload.
#[derive(Clone, Debug)]
pub struct Arc<E> {
    pub other: String,
    pub key: String,
    pub edge: E,
}

/// A node reached for the first time, at `depth`, from `from` over `edge`.
#[derive(Clone, Debug)]
pub struct Reached<E> {
    pub node: String,
    pub depth: usize,
    pub from: String,
    pub edge: E,
}

#[derive(Clone, Debug, Default)]
pub struct Walk<E> {
    /// In breadth-first order; within a level, by node.
    pub reached: Vec<Reached<E>>,
    /// Distinct followable edges from the last level to unvisited nodes.
    pub frontier: usize,
    /// Distinct skipped edges per reason, among the nodes expanded.
    pub skipped: BTreeMap<&'static str, usize>,
}

impl<E> Walk<E> {
    pub fn complete(&self) -> bool {
        self.frontier == 0 && self.skipped.values().all(|n| *n == 0)
    }
}

/// Walk from `start` to `depth` levels. `arcs` lists a node's edges (in a
/// deterministic order), `follow` classifies one, and a `terminal` node
/// (other than the start) is reached but never expanded.
pub fn walk<E: Clone>(
    start: &str,
    depth: usize,
    mut arcs: impl FnMut(&str) -> Vec<Arc<E>>,
    follow: impl Fn(&E) -> Follow,
    terminal: impl Fn(&str) -> bool,
) -> Walk<E> {
    let mut out = Walk {
        reached: Vec::new(),
        frontier: 0,
        skipped: BTreeMap::new(),
    };
    let mut visited: BTreeSet<String> = BTreeSet::from([start.to_owned()]);
    let mut skipped: BTreeSet<(&'static str, String)> = BTreeSet::new();
    let mut level = vec![start.to_owned()];
    for step in 1..=depth {
        let mut next: BTreeMap<String, Reached<E>> = BTreeMap::new();
        for node in level.iter().filter(|n| *n == start || !terminal(n)) {
            for arc in arcs(node) {
                match follow(&arc.edge) {
                    Follow::Skip(reason) => {
                        skipped.insert((reason, arc.key));
                    }
                    Follow::Yes if visited.contains(&arc.other) => {}
                    Follow::Yes => {
                        next.entry(arc.other.clone()).or_insert(Reached {
                            node: arc.other,
                            depth: step,
                            from: node.clone(),
                            edge: arc.edge,
                        });
                    }
                }
            }
        }
        level = next.keys().cloned().collect();
        visited.extend(level.iter().cloned());
        out.reached.extend(next.into_values());
    }
    let mut frontier = BTreeSet::new();
    for node in level.iter().filter(|n| *n == start || !terminal(n)) {
        for arc in arcs(node) {
            if follow(&arc.edge) == Follow::Yes && !visited.contains(&arc.other) {
                frontier.insert(arc.key);
            }
        }
    }
    out.frontier = frontier.len();
    for (reason, _) in skipped {
        *out.skipped.entry(reason).or_insert(0) += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Arc, Follow, walk};
    use std::collections::{BTreeMap, BTreeSet};

    /// A deterministic generator (xorshift64*).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self, bound: u64) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) % bound
        }
    }

    type Graph = Vec<(usize, usize, bool)>;

    fn arcs(graph: &Graph, node: &str) -> Vec<Arc<bool>> {
        let mut out = Vec::new();
        for (index, (a, b, ok)) in graph.iter().enumerate() {
            for (here, there) in [(a, b), (b, a)] {
                if format!("n{here}") == node && a != b {
                    out.push(Arc {
                        other: format!("n{there}"),
                        key: format!("e{index}"),
                        edge: *ok,
                    });
                }
            }
        }
        out
    }

    /// The reference: plain level sets over an undirected edge list.
    fn reference(
        graph: &Graph,
        depth: usize,
        terminal: &BTreeSet<String>,
    ) -> (BTreeMap<String, usize>, usize, usize) {
        let mut dist = BTreeMap::from([("n0".to_owned(), 0)]);
        let expands = |n: &String| n == "n0" || !terminal.contains(n);
        for d in 0..depth {
            let level: Vec<String> = dist
                .iter()
                .filter(|(_, v)| **v == d)
                .map(|(k, _)| k.clone())
                .collect();
            for n in level.iter().filter(|n| expands(n)) {
                for arc in arcs(graph, n).into_iter().filter(|a| a.edge) {
                    dist.entry(arc.other).or_insert(d + 1);
                }
            }
        }
        let expanded = |n: &String| dist.get(n).is_some_and(|d| *d < depth) && expands(n);
        let last = |n: &String| dist.get(n) == Some(&depth) && expands(n);
        let (mut frontier, mut skipped) = (0, 0);
        for (index, (a, b, ok)) in graph.iter().enumerate() {
            let _ = index;
            let (a, b) = (format!("n{a}"), format!("n{b}"));
            if a == b {
                continue;
            }
            if !ok && (expanded(&a) || expanded(&b)) {
                skipped += 1;
            }
            if *ok && ((last(&a) && !dist.contains_key(&b)) || (last(&b) && !dist.contains_key(&a)))
            {
                frontier += 1;
            }
        }
        dist.remove("n0");
        (dist, frontier, skipped)
    }

    #[test]
    fn walk_matches_the_reference_on_random_graphs() {
        let mut rng = Rng(0x5eed_cafe_f00d_0001);
        for _ in 0..400 {
            let nodes = 2 + rng.next(14) as usize;
            let graph: Graph = (0..rng.next(30))
                .map(|_| {
                    (
                        rng.next(nodes as u64) as usize,
                        rng.next(nodes as u64) as usize,
                        rng.next(5) != 0,
                    )
                })
                .collect();
            let terminal: BTreeSet<String> = (1..nodes)
                .filter(|_| rng.next(6) == 0)
                .map(|n| format!("n{n}"))
                .collect();
            let depth = 1 + rng.next(4) as usize;
            let result = walk(
                "n0",
                depth,
                |n| arcs(&graph, n),
                |ok| {
                    if *ok {
                        Follow::Yes
                    } else {
                        Follow::Skip("disputed")
                    }
                },
                |n| terminal.contains(n),
            );
            let reached: BTreeMap<String, usize> = result
                .reached
                .iter()
                .map(|r| (r.node.clone(), r.depth))
                .collect();
            let (expected, frontier, skipped) = reference(&graph, depth, &terminal);
            assert_eq!(reached, expected, "{graph:?} depth {depth}");
            assert_eq!(result.frontier, frontier, "{graph:?} depth {depth}");
            assert_eq!(
                result.skipped.get("disputed").copied().unwrap_or(0),
                skipped,
                "{graph:?}"
            );
            assert_eq!(result.complete(), frontier == 0 && skipped == 0);
        }
    }
}
