//! Read-only Dolt projection identities and navigation data, never knowledge authority.
//!
//! Current integration commits cannot be substituted with historical Dolt pins:
//! ```compile_fail
//! use writfold_domain::{DoltCommitId, DoltIntegrationCommitId};
//! fn current(_: DoltIntegrationCommitId) {}
//! let historical: DoltCommitId = "00000000000000000000000000000000".parse().unwrap();
//! current(historical);
//! ```
use crate::{
    DoltDatabaseId, DoltIntegrationCommitId, DoltToolGeneration, GraphEdgeId, GraphNodeId,
    GraphNodeKind, KbId, RealmId, RelationshipId, SchemaEpoch, StorageProfileId, records::record,
};

::vstd::prelude::verus! {
#[allow(clippy::redundant_static_lifetimes, reason = "pinned Verus requires this explicit reference lifetime")]
pub const DOLT_READ_PROFILE: &'static str = "dolt-corpus-read-v1";
}

record!(
    /// Complete exact read identity; construction does not admit a database or tool.
    DoltReadPin, DoltReadPinView {
        realm: RealmId,
        kb: KbId,
        database: DoltDatabaseId,
        commit: DoltIntegrationCommitId,
        epoch: SchemaEpoch,
        profile: StorageProfileId,
        producer: DoltToolGeneration,
    }
);

/// Navigation data; kind and label are not inference rules or evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphNode {
    id: GraphNodeId,
    kind: GraphNodeKind,
    label: String,
    home_path: Option<String>,
}

impl GraphNode {
    pub fn new(
        id: GraphNodeId,
        kind: GraphNodeKind,
        label: String,
        home_path: Option<String>,
    ) -> Self {
        Self {
            id,
            kind,
            label,
            home_path,
        }
    }
    pub fn id(&self) -> &GraphNodeId {
        &self.id
    }
    pub fn kind(&self) -> &GraphNodeKind {
        &self.kind
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn home_path(&self) -> Option<&str> {
        self.home_path.as_deref()
    }
}

record!(
    /// A navigation relationship, not a claim of knowledge truth or citation lineage.
    GraphEdge, GraphEdgeView {
        id: GraphEdgeId,
        source: GraphNodeId,
        target: GraphNodeId,
        kind: RelationshipId,
    }
);

/// Structurally typed projection. The store separately checks endpoint closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoltSnapshot {
    pin: DoltReadPin,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

impl DoltSnapshot {
    pub fn new(pin: DoltReadPin, nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> Self {
        Self { pin, nodes, edges }
    }
    pub fn pin(&self) -> &DoltReadPin {
        &self.pin
    }
    pub fn nodes(&self) -> &[GraphNode] {
        &self.nodes
    }
    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }
}
