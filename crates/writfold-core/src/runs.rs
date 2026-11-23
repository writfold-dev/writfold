//! What `kb runs` shows and when its list may claim to be complete (#98).
//! A run's effective state is `open` or `closed` (the vocabulary Writfold
//! and the prototype share) or some other word, kept as authored. `--open`
//! hides only a run known to be closed: a state outside the vocabulary is
//! never taken as closed, and it is never shown as an ordinary run either,
//! but flagged. The list is complete only when every run directory has a
//! readable, unique `manifest.json`: a directory under `runs/` without one
//! is a run the list cannot show. Translated and proved
//! (`proofs/Writfold/Runs.lean`); reading the state word, the sidecar and
//! the index is tested IO in `writfold-kb`.

/// A run's effective state, classified.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Open,
    Closed,
    /// Any other word (`done`, `parked`, …), kept as authored.
    Other,
}

/// How one run appears in a `runs` listing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shown {
    /// Left out (`--open` and the run is closed).
    Hidden,
    /// Listed with its state.
    Listed,
    /// Listed and flagged: its state is outside the vocabulary.
    Flagged,
}

/// How a run in `state` appears, `open_only` for `--open`.
pub fn shown(state: State, open_only: bool) -> Shown {
    match state {
        State::Closed => {
            if open_only {
                Shown::Hidden
            } else {
                Shown::Listed
            }
        }
        State::Open => Shown::Listed,
        State::Other => Shown::Flagged,
    }
}

/// Whether the run list may claim `coverage=complete`: no manifest under
/// `runs/<id>/` is unreadable (unparsable or a duplicate `run_id`) and no
/// run directory lacks a manifest.
pub fn complete(unreadable: usize, no_manifest: usize) -> bool {
    unreadable == 0 && no_manifest == 0
}

#[cfg(test)]
mod tests {
    use super::{Shown, State, complete, shown};

    #[test]
    fn open_hides_only_a_closed_run() {
        assert_eq!(shown(State::Closed, true), Shown::Hidden);
        assert_eq!(shown(State::Closed, false), Shown::Listed);
        assert_eq!(shown(State::Open, true), Shown::Listed);
        assert_eq!(shown(State::Other, true), Shown::Flagged);
        assert_eq!(shown(State::Other, false), Shown::Flagged);
    }

    #[test]
    fn a_directory_without_manifest_makes_the_list_partial() {
        assert!(complete(0, 0));
        assert!(!complete(0, 1));
        assert!(!complete(2, 0));
    }
}
