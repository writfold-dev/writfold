//! Finite supplied procedure declarations, not an authenticated catalog or IO authority.
//!
//! Fields are public because these values are unadmitted input: only the kernel's
//! checked procedure establishes structure. Lists retain their supplied order;
//! declarations and reference sets must be duplicate-free at admission, and no
//! spelling, Unicode or ordering is normalized. `ProcedureMaterial` deliberately
//! has no field for its own revision.
use super::dependency::{ExportName, PackageRef};
use super::execution::{Capability, Gate, ValueType};
use super::{BlobDigest, DependencyLockDigest, ProcedureRevision, SkillRevision};
use crate::{
    AdapterId, ContextId, ExecutionCost, GateId, ProcedureDependencyId, ProcedureId,
    ProcedureNodeId, ProcedureObjectId, ProcedurePredicateId, ProcedureRank, RealmId, ResourceId,
    SkillId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillRef {
    pub id: SkillId,
    pub revision: SkillRevision,
}

/// Exact retained bytes a Read node may name; not proof the bytes are available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectDeclaration {
    pub id: ProcedureObjectId,
    pub blob: BlobDigest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextDeclaration {
    pub id: ContextId,
    pub objects: Vec<ProcedureObjectId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeDeclaration {
    pub id: ProcedureDependencyId,
    pub package: PackageRef,
}

/// A declared ceiling for a named resource, never a current grant or reservation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceDeclaration {
    pub id: ResourceId,
    pub context: ContextId,
    pub ceiling: Vec<Capability>,
}

/// Exact implementation bytes and package export; this does not admit an adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterDeclaration {
    pub id: AdapterId,
    pub package: PackageRef,
    pub export: ExportName,
    pub implementation: BlobDigest,
    pub input: ValueType,
    pub output: ValueType,
    pub capability: Capability,
}

/// A gate definition is a pinned requirement, never a successful gate result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateDefinition {
    pub gate: Gate,
    pub id: GateId,
    pub definition: BlobDigest,
    pub predicate: ProcedurePredicateId,
}

/// A registered signature and pinned definition say nothing about semantic truth.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateSignature {
    pub id: ProcedurePredicateId,
    pub input: ValueType,
    pub output: ValueType,
    pub definition: BlobDigest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dependency {
    pub node: ProcedureNodeId,
    pub value_type: ValueType,
}

/// Pinned by logical identity and exact revision, never a mutable name lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallTarget {
    pub procedure: ProcedureId,
    pub revision: ProcedureRevision,
    pub input: ValueType,
    pub output: ValueType,
    pub delegation: Vec<Capability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeForm {
    Read(ProcedureObjectId),
    Select(Vec<ProcedureNodeId>),
    Judge(ProcedurePredicateId),
    Call(CallTarget),
    Effect(Capability),
    Check(ProcedurePredicateId),
    Join(Vec<ProcedureNodeId>),
    End,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub id: ProcedureNodeId,
    pub form: NodeForm,
    pub input: ValueType,
    pub output: ValueType,
    pub dependencies: Vec<Dependency>,
    pub prerequisites: Vec<ProcedureNodeId>,
    pub preconditions: Vec<ProcedurePredicateId>,
    pub postconditions: Vec<ProcedurePredicateId>,
    pub context: ContextId,
    pub required_context: Vec<ProcedureObjectId>,
    pub scope: Vec<ProcedureDependencyId>,
    pub resources: Vec<ResourceId>,
    pub ceiling: Vec<Capability>,
    pub floor: Vec<Gate>,
    pub failure: Option<ProcedureNodeId>,
    pub cancellation: Option<ProcedureNodeId>,
    pub cost: ExecutionCost,
    /// Required on Effect, absent on all other forms in this closed profile.
    pub adapter: Option<AdapterId>,
}

/// Complete digest material: logical identity, typed IR, contracts and pins.
/// Future results, signatures and this procedure's own revision are excluded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcedureMaterial {
    pub realm: RealmId,
    pub id: ProcedureId,
    pub package: PackageRef,
    pub skill: SkillRef,
    pub dependency_lock: DependencyLockDigest,
    pub closure: Vec<ObjectDeclaration>,
    pub contexts: Vec<ContextDeclaration>,
    pub scope: Vec<ScopeDeclaration>,
    pub resources: Vec<ResourceDeclaration>,
    pub adapters: Vec<AdapterDeclaration>,
    pub gates: Vec<GateDefinition>,
    pub predicates: Vec<PredicateSignature>,
    pub input: ValueType,
    pub output: ValueType,
    pub ceiling: Vec<Capability>,
    pub floor: Vec<Gate>,
    pub rank: ProcedureRank,
    pub entry: ProcedureNodeId,
    pub nodes: Vec<Node>,
    pub preconditions: Vec<ProcedurePredicateId>,
    pub postconditions: Vec<ProcedurePredicateId>,
}

/// The revision is supplied, not recomputed here; digest binding is a wire concern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Procedure {
    pub revision: ProcedureRevision,
    pub material: ProcedureMaterial,
}

/// Supplied finite catalog and registry. Neither is authenticated by structure.
/// Admission retains this exact input, including the supplied list ordering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcedureInput {
    pub procedure: Procedure,
    pub catalog: Vec<Procedure>,
    pub registry: Vec<PredicateSignature>,
}
