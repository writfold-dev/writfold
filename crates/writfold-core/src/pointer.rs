//! Which run is current for a seat: one agent session working in one work
//! tree. A KB clone keeps up to three local pointers a seat can read: the
//! session's own, the work tree's, and the clone's shared one. The most
//! specific pointer that is set decides, whatever state its run is in: a
//! closed or missing run there is reported, never skipped, so a seat is
//! never judged against another seat's run because its own run ended.
//! Making a run current writes every pointer the seat has, and the shared
//! one always. Translated and proved (`proofs/Writfold/Pointer.lean`).
//! Reading the pointer files and the run each names is `writfold-kb`'s
//! (`runs.rs`), tested there.

/// How one pointer reads.
pub enum Pointer {
    /// No pointer (no file, an empty one, or a level the seat lacks).
    Unset,
    /// It names an open run.
    Open,
    /// It names a closed run.
    Closed,
    /// It names no run of the KB.
    Gone,
}

/// The three pointers, most specific first.
pub enum Level {
    Session,
    Tree,
    Shared,
}

/// Whether a pointer is set.
pub fn is_set(one: &Pointer) -> bool {
    !matches!(one, Pointer::Unset)
}

/// The level that decides: the most specific pointer that is set.
pub fn level(session: &Pointer, tree: &Pointer, shared: &Pointer) -> Option<Level> {
    if is_set(session) {
        return Some(Level::Session);
    }
    if is_set(tree) {
        return Some(Level::Tree);
    }
    if is_set(shared) {
        return Some(Level::Shared);
    }
    None
}

/// Whether the seat's current run is open: the deciding pointer names an
/// open run.
pub fn open(session: &Pointer, tree: &Pointer, shared: &Pointer) -> bool {
    match level(session, tree, shared) {
        None => false,
        Some(Level::Session) => matches!(session, Pointer::Open),
        Some(Level::Tree) => matches!(tree, Pointer::Open),
        Some(Level::Shared) => matches!(shared, Pointer::Open),
    }
}

/// The pointers a seat writes when it makes a run current.
pub struct Writes {
    pub session: bool,
    pub tree: bool,
    pub shared: bool,
}

/// A seat writes the pointer of each level it has, and the shared one
/// always.
pub fn writes(has_session: bool, has_tree: bool) -> Writes {
    Writes {
        session: has_session,
        tree: has_tree,
        shared: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{Level, Pointer, level, open, writes};

    fn of(code: u8) -> Pointer {
        match code {
            0 => Pointer::Unset,
            1 => Pointer::Open,
            2 => Pointer::Closed,
            _ => Pointer::Gone,
        }
    }

    /// Every combination: the first set pointer decides, and only an open
    /// run there is open.
    #[test]
    fn the_most_specific_set_pointer_decides() {
        for s in 0..4u8 {
            for t in 0..4u8 {
                for h in 0..4u8 {
                    let found = level(&of(s), &of(t), &of(h));
                    let want = if s != 0 {
                        (1, s)
                    } else if t != 0 {
                        (2, t)
                    } else if h != 0 {
                        (3, h)
                    } else {
                        (0, 0)
                    };
                    let got = match found {
                        None => 0,
                        Some(Level::Session) => 1,
                        Some(Level::Tree) => 2,
                        Some(Level::Shared) => 3,
                    };
                    assert_eq!(got, want.0, "{s} {t} {h}");
                    assert_eq!(open(&of(s), &of(t), &of(h)), want.1 == 1, "{s} {t} {h}");
                }
            }
        }
    }

    /// A closed session pointer is not skipped for an open tree pointer.
    #[test]
    fn a_closed_run_is_reported_not_skipped() {
        assert!(!open(&Pointer::Closed, &Pointer::Open, &Pointer::Open));
        assert!(!open(&Pointer::Unset, &Pointer::Gone, &Pointer::Open));
    }

    /// The writer reads its run back at every seat shape, and the shared
    /// pointer is always written.
    #[test]
    fn a_seat_reads_back_the_run_it_made_current() {
        for (has_session, has_tree) in [(false, false), (false, true), (true, false), (true, true)]
        {
            let w = writes(has_session, has_tree);
            assert!(w.shared);
            assert_eq!((w.session, w.tree), (has_session, has_tree));
            // A written pointer names the open run; a level the seat
            // lacks reads as unset; one it has but did not write is as it was.
            let read = |has: bool, written: bool, old: Pointer| match (has, written) {
                (false, _) => Pointer::Unset,
                (true, true) => Pointer::Open,
                (true, false) => old,
            };
            let session = read(has_session, w.session, Pointer::Closed);
            let tree = read(has_tree, w.tree, Pointer::Closed);
            let shared = read(true, w.shared, Pointer::Gone);
            assert!(open(&session, &tree, &shared));
        }
    }
}
