//! Pure decisions of the Dolt-backed publication (`writfold-dolt-integration/1`,
//! DINT02 first slice): which step comes next before any write, and how an
//! interrupted publication is reconciled from observed native facts alone.
//!
//! A publication runs in a fixed order under the realm's Git and Dolt writer
//! leases: Git expected-old check, retained candidate blob, Dolt conditional
//! commit, Git prepared commit naming the new Dolt pin, Git expected-old CAS.
//! The Git CAS is the only visibility point. Nothing here does IO.
//!
//! A bootstrap publication creates a realm's first release. Its approval names
//! no predecessor, so its Git expected-old state is an absent completed ref
//! and its storage base is the realm's origin: a head with no Writfold write
//! on its history. Every other step, and recovery, is the same.
//!
//! Proof candidate: not yet in the Charon `start_from` list, so the `proofs`
//! gate does not translate it and no theorem is claimed about it.

/// What the publisher observed before writing anything.
pub struct Before {
    /// The approval names no predecessor: the first release of a realm.
    pub bootstrap: bool,
    /// The Git completed ref is exactly the approval's base release; for a
    /// bootstrap, the completed ref is absent.
    pub completed_is_base: bool,
    /// The base release's retained manifest names exactly the approval's base
    /// pin; for a bootstrap, no Writfold write is on the storage head's history.
    pub base_pin_matches: bool,
    /// A prepared commit for this operation already exists.
    pub prepared: bool,
}

/// The decision before any write.
pub enum Admission {
    /// Every guard holds: retain the candidate, then the Dolt write.
    Proceed,
    /// The Git completed ref moved: a typed conflict, nothing written.
    Conflict,
    /// The base release does not name the approval's base pin.
    Misinterpreted,
    /// A bootstrap over storage that already carries a Writfold write (an
    /// earlier bootstrap, completed or not): refused, nothing written.
    Initialized,
    /// This operation was already prepared: reconcile with observation.
    AlreadyPrepared,
}

/// Guards in this order: Git conflict, historical interpretation (for a
/// bootstrap, an uninitialized storage origin), reuse.
pub fn admit(before: &Before) -> Admission {
    if !before.completed_is_base {
        Admission::Conflict
    } else if !before.base_pin_matches {
        if before.bootstrap {
            Admission::Initialized
        } else {
            Admission::Misinterpreted
        }
    } else if before.prepared {
        Admission::AlreadyPrepared
    } else {
        Admission::Proceed
    }
}

/// Native facts observed, read-only, after an interrupted publication.
pub struct Facts {
    /// The completed ref has this operation's prepared commit on its history.
    pub published: bool,
    /// A prepared commit for this operation exists.
    pub prepared: bool,
    /// The prepared commit names exactly the Dolt commit carrying the request.
    pub prepared_names_applied: bool,
    /// A Dolt commit carrying exactly this request exists on the head history.
    pub applied: bool,
    /// The Dolt head is exactly that commit.
    pub head_is_applied: bool,
    /// The Git completed ref is still the approval's base release (for a
    /// bootstrap, still absent).
    pub completed_is_base: bool,
}

/// How an interrupted publication resolves.
pub enum Resolution {
    /// Visible: the completed ref carries this operation's release.
    Published,
    /// Nothing durable was written for this request; it never becomes visible.
    NotPublished,
    /// The Dolt commit exists and nothing else moved; the remaining Git steps
    /// are completable. `true` when the prepared commit already exists.
    Interrupted(bool),
    /// Facts that the ordering and the leases exclude. Refuse; a person decides.
    Diverged,
}

/// Decide from observation alone; never retries the Dolt write.
pub fn resolve(facts: &Facts) -> Resolution {
    if facts.published {
        if facts.applied && facts.prepared && facts.prepared_names_applied {
            Resolution::Published
        } else {
            Resolution::Diverged
        }
    } else if !facts.applied {
        if facts.prepared {
            Resolution::Diverged
        } else {
            Resolution::NotPublished
        }
    } else if facts.completed_is_base && facts.head_is_applied {
        if facts.prepared {
            if facts.prepared_names_applied {
                Resolution::Interrupted(true)
            } else {
                Resolution::Diverged
            }
        } else {
            Resolution::Interrupted(false)
        }
    } else {
        Resolution::Diverged
    }
}
