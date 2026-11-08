//! A small valid catalog: `review` reads, judges, calls `check`, joins, writes
//! through a declared adapter and ends. `check` is a rank-0 leaf procedure.
use writfold_domain::engineering::dependency::{ExportName, PackageId, PackageRef};
use writfold_domain::engineering::execution::{Capability, Gate, ValueType};
use writfold_domain::engineering::procedure::*;
use writfold_domain::engineering::{
    BlobDigest, DependencyLockDigest, EngineeringDigest, PackagePayloadDigest, ProcedureRevision,
    SkillRevision,
};
use writfold_domain::{ExecutionCost, ProcedureNodeId, ProcedureRank};

pub(super) type TestResult<T = (), E = Box<dyn std::error::Error>> = Result<T, E>;

pub(super) fn id(value: u64) -> TestResult<ProcedureNodeId> {
    Ok(ProcedureNodeId::new(value)?)
}

fn digest<T: EngineeringDigest>(byte: u8) -> T {
    T::from_digest([byte; 32])
}

pub(super) fn package(byte: u8) -> TestResult<PackageRef> {
    Ok(PackageRef::new(
        PackageId::try_from(format!("package-{byte}"))?,
        digest::<PackagePayloadDigest>(byte),
    ))
}

pub(super) fn signature(name: &str, input: ValueType) -> TestResult<PredicateSignature> {
    Ok(PredicateSignature {
        id: name.parse()?,
        input,
        output: ValueType::Evidence,
        definition: digest::<BlobDigest>(9),
    })
}

/// A node with every optional list empty, inside context `ctx`.
pub(super) fn node(
    number: u64,
    form: NodeForm,
    input: ValueType,
    output: ValueType,
) -> TestResult<Node> {
    Ok(Node {
        id: id(number)?,
        form,
        input,
        output,
        dependencies: vec![],
        prerequisites: vec![],
        preconditions: vec![],
        postconditions: vec![],
        context: "ctx".parse()?,
        required_context: vec![],
        scope: vec![],
        resources: vec![],
        ceiling: vec![Capability::Read, Capability::Check, Capability::Write],
        floor: vec![Gate::Review],
        failure: None,
        cancellation: None,
        cost: ExecutionCost::new(1)?,
        adapter: None,
    })
}

pub(super) fn dependency(number: u64, value_type: ValueType) -> TestResult<Dependency> {
    Ok(Dependency {
        node: id(number)?,
        value_type,
    })
}

fn material(name: &str, rank: u64, nodes: Vec<Node>) -> TestResult<ProcedureMaterial> {
    Ok(ProcedureMaterial {
        realm: "realm".parse()?,
        id: name.parse()?,
        package: package(1)?,
        skill: SkillRef {
            id: "skill".parse()?,
            revision: digest::<SkillRevision>(2),
        },
        dependency_lock: digest::<DependencyLockDigest>(3),
        closure: vec![ObjectDeclaration {
            id: "source".parse()?,
            blob: digest::<BlobDigest>(4),
        }],
        contexts: vec![ContextDeclaration {
            id: "ctx".parse()?,
            objects: vec!["source".parse()?],
        }],
        scope: vec![ScopeDeclaration {
            id: "tooling".parse()?,
            package: package(5)?,
        }],
        resources: vec![ResourceDeclaration {
            id: "worktree".parse()?,
            context: "ctx".parse()?,
            ceiling: vec![Capability::Read, Capability::Write],
        }],
        adapters: vec![AdapterDeclaration {
            id: "materialize".parse()?,
            package: package(5)?,
            export: ExportName::try_from("apply".to_owned())?,
            implementation: digest::<BlobDigest>(6),
            input: ValueType::Evidence,
            output: ValueType::Artifact,
            capability: Capability::Write,
        }],
        gates: vec![GateDefinition {
            gate: Gate::Review,
            id: "review-gate".parse()?,
            definition: digest::<BlobDigest>(7),
            predicate: "reviewed".parse()?,
        }],
        predicates: vec![
            signature("reviewed", ValueType::Artifact)?,
            signature("ready", ValueType::Unit)?,
        ],
        input: ValueType::Unit,
        output: ValueType::Artifact,
        ceiling: vec![Capability::Read, Capability::Check, Capability::Write],
        floor: vec![Gate::Review],
        rank: ProcedureRank::new(rank)?,
        entry: id(1)?,
        nodes,
        preconditions: vec!["ready".parse()?],
        postconditions: vec![],
    })
}

/// The callee: Unit -> Artifact, rank 0, Read-only ceiling.
pub(super) fn leaf() -> TestResult<Procedure> {
    let mut material = material(
        "check",
        0,
        vec![
            node(
                1,
                NodeForm::Read("source".parse()?),
                ValueType::Unit,
                ValueType::Artifact,
            )?,
            Node {
                dependencies: vec![dependency(1, ValueType::Artifact)?],
                ..node(2, NodeForm::End, ValueType::Artifact, ValueType::Artifact)?
            },
        ],
    )?;
    material.ceiling = vec![Capability::Read];
    for node in &mut material.nodes {
        node.ceiling = vec![Capability::Read];
    }
    material.resources.clear();
    material.adapters.clear();
    Ok(Procedure {
        revision: digest::<ProcedureRevision>(10),
        material,
    })
}

pub(super) fn call_target() -> TestResult<CallTarget> {
    Ok(CallTarget {
        procedure: "check".parse()?,
        revision: digest::<ProcedureRevision>(10),
        input: ValueType::Unit,
        output: ValueType::Artifact,
        delegation: vec![Capability::Read],
    })
}

fn root_nodes() -> TestResult<Vec<Node>> {
    Ok(vec![
        Node {
            required_context: vec!["source".parse()?],
            ..node(
                1,
                NodeForm::Read("source".parse()?),
                ValueType::Unit,
                ValueType::Artifact,
            )?
        },
        Node {
            dependencies: vec![dependency(1, ValueType::Artifact)?],
            failure: Some(id(6)?),
            ..node(
                2,
                NodeForm::Judge("reviewed".parse()?),
                ValueType::Artifact,
                ValueType::Evidence,
            )?
        },
        node(
            3,
            NodeForm::Call(call_target()?),
            ValueType::Unit,
            ValueType::Artifact,
        )?,
        Node {
            prerequisites: vec![id(2)?],
            ..node(
                4,
                NodeForm::Join(vec![id(3)?]),
                ValueType::Evidence,
                ValueType::Evidence,
            )?
        },
        Node {
            resources: vec!["worktree".parse()?],
            adapter: Some("materialize".parse()?),
            prerequisites: vec![id(4)?],
            ..node(
                5,
                NodeForm::Effect(Capability::Write),
                ValueType::Evidence,
                ValueType::Artifact,
            )?
        },
        node(6, NodeForm::End, ValueType::Unit, ValueType::Artifact)?,
    ])
}

pub(super) fn input() -> TestResult<ProcedureInput> {
    let root = material("review", 1, root_nodes()?)?;
    let callee = leaf()?;
    let mut registry = root.predicates.clone();
    registry.push(signature("unused", ValueType::Evidence)?);
    Ok(ProcedureInput {
        procedure: Procedure {
            revision: digest::<ProcedureRevision>(11),
            material: root,
        },
        catalog: vec![callee],
        registry,
    })
}

/// Mutable access to one root node, failing rather than panicking if absent.
pub(super) fn at(supplied: &mut ProcedureInput, number: u64) -> TestResult<&mut Node> {
    let wanted = id(number)?;
    supplied
        .procedure
        .material
        .nodes
        .iter_mut()
        .find(|n| n.id == wanted)
        .ok_or_else(|| format!("fixture has no node {number}").into())
}
