//! Supplied ledger-head structure, never a validated prefix or current authority.
//!
//! Logical record IDs, event fingerprints and retained-byte fingerprints remain
//! distinct. A consumer must load the complete record/closure, validate origin
//! and historical replay, and qualify current ownership before selecting a head.
use super::{BlobDigest, LedgerEventDigest};
use crate::{
    ByteCount, InstanceId, LedgerRecordId, LedgerSequence, OwnershipGeneration, RealmId, StreamId,
    records::record,
};

vstd::prelude::verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamKind {
    Control,
    Run,
    Learning,
}

/// Identifies the existing mathematical reducer profile, not its implementation
/// or qualification. A head cannot make an unavailable reducer available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReducerProfile {
    EngineeringV1,
}
}

crate::records::scalar_views!(StreamKind, ReducerProfile, BlobDigest, LedgerEventDigest);

record!(
    /// Realm-qualified logical stream position. Sequence zero is represented by
    /// absence of a head, not an invented empty record.
    LedgerPosition, LedgerPositionView {
        realm: RealmId,
        stream: StreamId,
        kind: StreamKind,
        sequence: LedgerSequence,
    }
);
record!(
    /// Material assertions within an enclosing realm-qualified head. These
    /// fingerprints do not replace the logical record ID or prove availability.
    LedgerMaterial, LedgerMaterialView {
        record: LedgerRecordId,
        event: LedgerEventDigest,
        blob: BlobDigest,
        bytes: ByteCount,
    }
);
record!(
    /// Recorded writer stamp, not a live ownership lease or a request generation.
    LedgerOwner, LedgerOwnerView {
        instance: InstanceId,
        generation: OwnershipGeneration,
    }
);
record!(
    /// Structurally typed candidate. Equality is neither historical validation,
    /// origin authentication, retention evidence nor permission to advance.
    LedgerHead, LedgerHeadView {
        position: LedgerPosition,
        material: LedgerMaterial,
        profile: ReducerProfile,
        owner: LedgerOwner,
    }
);
