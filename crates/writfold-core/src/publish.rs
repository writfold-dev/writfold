//! Whether a freshly derived index may replace the stored one (#58). A
//! repair (`index verify`, `kb check`) replaces the index only on a
//! confirmed mismatch, while the KB is still at the pin the shadow was
//! built for and the live index is still the very generation that was
//! compared at that pin: drift alone never authorises a replacement. A
//! full rebuild or a refresh publishes what it derived unless the KB moved
//! while it read. Translated and proved (`proofs/Writfold/Publish.lean`);
//! the protocol around it is `spec/KbRepair.tla` and `spec/KbWrite.tla`;
//! observing, building, comparing and renaming are tested IO in
//! `writfold-kb`.

/// What to do with a derived index.
pub enum Publish {
    /// Replace the stored index with it.
    Replace,
    /// Keep the stored index; discard the derivation.
    Keep,
    /// Discard the derivation and observe again: something moved.
    Retry,
}

/// What a repair found.
pub struct Repair {
    /// The live index differs from the shadow rebuild.
    pub mismatch: bool,
    /// The KB is no longer at the pin the shadow was built for.
    pub source_moved: bool,
    /// The live index is not the generation that was compared, or not at
    /// that pin.
    pub live_moved: bool,
}

/// Replace only a mismatching index, and only when nothing moved.
pub fn repair(found: &Repair) -> Publish {
    if !found.mismatch {
        return Publish::Keep;
    }
    if found.source_moved || found.live_moved {
        return Publish::Retry;
    }
    Publish::Replace
}

/// A full rebuild or a refresh: publish unless the KB moved while it read.
pub fn derived(source_moved: bool) -> Publish {
    if source_moved {
        return Publish::Retry;
    }
    Publish::Replace
}

#[cfg(test)]
mod tests {
    use super::{Publish, Repair, derived, repair};

    #[test]
    fn a_repair_replaces_only_a_confirmed_mismatch() {
        let found = |mismatch, source_moved, live_moved| Repair {
            mismatch,
            source_moved,
            live_moved,
        };
        assert!(matches!(
            repair(&found(true, false, false)),
            Publish::Replace
        ));
        assert!(matches!(repair(&found(false, false, false)), Publish::Keep));
        assert!(matches!(repair(&found(false, true, true)), Publish::Keep));
        assert!(matches!(repair(&found(true, true, false)), Publish::Retry));
        assert!(matches!(repair(&found(true, false, true)), Publish::Retry));
        assert!(matches!(derived(false), Publish::Replace));
        assert!(matches!(derived(true), Publish::Retry));
    }
}
