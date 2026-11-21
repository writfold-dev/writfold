//! Loops in the folded plan (#76): how many rounds a loop step has, and
//! which steps sit in a round that is no longer the newest (history: never
//! re-judged, never stale). Counting is tested IO over the fold; what the
//! counts allow is [`writfold_core::plan::round`].
use super::fold::{Graph, Step};
use writfold_core::plan::round::{Bounds, Place};

impl Graph {
    /// The rounds of loop step `step`: the uses of its body under it.
    pub fn rounds(&self, step: &Step) -> usize {
        let Some((body, _)) = &step.repeat else {
            return 0;
        };
        let under = self
            .uses
            .iter()
            .filter(|u| u.parent == step.path && &u.template == body);
        under.count()
    }

    /// Where `path` sits in each loop on it: (round, rounds) per loop.
    pub fn places(&self, path: &str) -> Vec<Place> {
        let mut out = Vec::new();
        for step in self.steps.iter().filter(|s| s.repeat.is_some()) {
            let Some((body, _)) = &step.repeat else {
                continue;
            };
            let head = format!("{}/{body}#", step.path);
            let Some(rest) = path.strip_prefix(&head) else {
                continue;
            };
            let round = rest.split('/').next().and_then(|n| n.parse::<usize>().ok());
            if let Some(round) = round {
                out.push(Place {
                    round,
                    rounds: self.rounds(step),
                });
            }
        }
        out
    }

    /// Whether `path` is in a round that is not its loop's newest.
    pub fn superseded(&self, path: &str) -> bool {
        writfold_core::plan::round::superseded_any(&self.places(path))
    }
}

/// The kernel's bounds of a loop step.
pub fn bounds(step: &Step) -> Option<Bounds> {
    step.repeat.as_ref().map(|(_, r)| Bounds {
        min: r.min,
        max: r.max,
    })
}
