//! Portable storage identity, never a path, reader handle or approval capability.
//!
//! Native and historical commit namespaces are not interchangeable:
//! ```compile_fail
//! use writfold_domain::{DoltCommitId, GraphCommitId};
//! fn native_snapshot(_: GraphCommitId) {}
//! let legacy: DoltCommitId = "00000000000000000000000000000000".parse().unwrap();
//! native_snapshot(legacy);
//! ```
use crate::{
    DoltCommitId, GraphCommitId, GraphId, KbId, NativeFormatVersion, OmnigraphToolGeneration,
    QueryContractDigest, SchemaDigest, SchemaEpochId, StorageProfileId, records::record,
};

record!(
    /// Immutable physical schema epoch and its frozen historical query contract.
    SchemaEpoch, SchemaEpochView {
        id: SchemaEpochId,
        schema: SchemaDigest,
        query: QueryContractDigest,
    }
);
record!(
    /// A structurally checked native pin, not evidence that its objects exist.
    OmnigraphPin, OmnigraphPinView {
        kb: KbId,
        graph: GraphId,
        commit: GraphCommitId,
        epoch: SchemaEpoch,
        profile: StorageProfileId,
        native_format: NativeFormatVersion,
        producer: OmnigraphToolGeneration,
    }
);
record!(
    /// Historical representation only; this does not enable a second normal backend.
    LegacyDoltPin, LegacyDoltPinView {
        kb: KbId,
        commit: DoltCommitId,
    }
);

::vstd::prelude::verus! {
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoragePin {
    Omnigraph(OmnigraphPin),
    LegacyDolt(LegacyDoltPin),
}
}
