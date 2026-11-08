//! Deterministic embedding of a Lean `Procedure` case into Rust procedure input.
//!
//! Lean's judgment assumes contexts, scopes, resources, adapters and gate
//! definitions exist. The embedding declares exactly those, so any Rust refusal
//! of an embedded Lean-valid case is a real strictness difference to report.
//! A Lean value that cannot become a Rust domain value (for example a zero cost)
//! is a Rust refusal at the domain boundary.
use serde_json::Value;
use std::collections::BTreeSet;
use writfold_domain::engineering::dependency::{ExportName, PackageId, PackageRef};
use writfold_domain::engineering::execution::{Capability, Gate, ValueType};
use writfold_domain::engineering::procedure::*;
use writfold_domain::engineering::{
    BlobDigest, DependencyLockDigest, EngineeringDigest, PackagePayloadDigest, ProcedureRevision,
    SkillRevision,
};
use writfold_domain::{ExecutionCost, ProcedureNodeId, ProcedureRank};

pub type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

fn field<'a>(value: &'a Value, key: &str) -> Fallible<&'a Value> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field {key}").into())
}
fn number(value: &Value, key: &str) -> Fallible<u64> {
    field(value, key)?
        .as_u64()
        .ok_or_else(|| format!("{key} is not a natural").into())
}
fn list<'a>(value: &'a Value, key: &str) -> Fallible<&'a Vec<Value>> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key} is not a list").into())
}
fn numbers(value: &Value, key: &str) -> Fallible<Vec<u64>> {
    list(value, key)?
        .iter()
        .map(|v| v.as_u64().ok_or_else(|| "not a natural".into()))
        .collect()
}
fn text<'a>(value: &'a Value, key: &str) -> Fallible<&'a str> {
    field(value, key)?
        .as_str()
        .ok_or_else(|| format!("{key} is not text").into())
}
fn digest<T: EngineeringDigest>(n: u64) -> T {
    let mut bytes = [0u8; 32];
    bytes[24..].copy_from_slice(&n.to_be_bytes());
    T::from_digest(bytes)
}
fn named<T: std::str::FromStr>(prefix: &str, n: u64) -> Fallible<T>
where
    T::Err: std::error::Error + 'static,
{
    Ok(format!("{prefix}-{n}").parse::<T>()?)
}
fn value_type(value: &Value, key: &str) -> Fallible<ValueType> {
    Ok(match text(value, key)? {
        "unit" => ValueType::Unit,
        "artifact" => ValueType::Artifact,
        "evidence" => ValueType::Evidence,
        other => return Err(format!("unknown value type {other}").into()),
    })
}
fn capability(name: &str) -> Fallible<Capability> {
    Ok(match name {
        "read" => Capability::Read,
        "check" => Capability::Check,
        "write" => Capability::Write,
        "publish" => Capability::Publish,
        other => return Err(format!("unknown capability {other}").into()),
    })
}
fn capabilities(value: &Value, key: &str) -> Fallible<Vec<Capability>> {
    list(value, key)?
        .iter()
        .map(|v| capability(v.as_str().unwrap_or_default()))
        .collect()
}
fn gates(value: &Value, key: &str) -> Fallible<Vec<Gate>> {
    list(value, key)?
        .iter()
        .map(|v| match v.as_str() {
            Some("test") => Ok(Gate::Test),
            Some("review") => Ok(Gate::Review),
            Some("compatibility") => Ok(Gate::Compatibility),
            _ => Err("unknown gate".into()),
        })
        .collect()
}
fn node_ids(value: &Value, key: &str) -> Fallible<Vec<ProcedureNodeId>> {
    numbers(value, key)?
        .into_iter()
        .map(|n| Ok(ProcedureNodeId::new(n)?))
        .collect()
}
fn optional_node(value: &Value, key: &str) -> Fallible<Option<ProcedureNodeId>> {
    match field(value, key)? {
        Value::Null => Ok(None),
        v => Ok(Some(ProcedureNodeId::new(v.as_u64().ok_or("route")?)?)),
    }
}
fn package(n: u64) -> Fallible<PackageRef> {
    Ok(PackageRef::new(
        PackageId::try_from(format!("package-{n}"))?,
        digest::<PackagePayloadDigest>(n),
    ))
}

/// The registered gate predicate added by the embedding; Lean has no gate definitions.
pub const GATE_PREDICATE: u64 = 1000;

pub fn signature(value: &Value) -> Fallible<PredicateSignature> {
    Ok(PredicateSignature {
        id: named("predicate", number(value, "id")?)?,
        input: value_type(value, "input")?,
        output: value_type(value, "output")?,
        definition: digest::<BlobDigest>(number(value, "id")?),
    })
}

pub fn gate_signature() -> Fallible<PredicateSignature> {
    Ok(PredicateSignature {
        id: named("predicate", GATE_PREDICATE)?,
        input: ValueType::Unit,
        output: ValueType::Evidence,
        definition: digest::<BlobDigest>(GATE_PREDICATE),
    })
}

fn form(value: &Value) -> Fallible<NodeForm> {
    Ok(match text(value, "kind")? {
        "read" => NodeForm::Read(named("object", number(value, "object")?)?),
        "select" => NodeForm::Select(node_ids(value, "routes")?),
        "judge" => NodeForm::Judge(named("predicate", number(value, "predicate")?)?),
        "check" => NodeForm::Check(named("predicate", number(value, "predicate")?)?),
        "call" => NodeForm::Call(CallTarget {
            procedure: named("procedure", number(value, "procedure")?)?,
            revision: digest::<ProcedureRevision>(number(value, "revision")?),
            input: value_type(value, "input")?,
            output: value_type(value, "output")?,
            delegation: capabilities(value, "delegation")?,
        }),
        "effect" => NodeForm::Effect(capability(text(value, "capability")?)?),
        "join" => NodeForm::Join(node_ids(value, "calls")?),
        "end" => NodeForm::End,
        other => return Err(format!("unknown form {other}").into()),
    })
}

fn node(value: &Value) -> Fallible<Node> {
    let id = number(value, "id")?;
    let form = form(field(value, "form")?)?;
    let adapter = match form {
        NodeForm::Effect(_) => Some(named("adapter", id)?),
        _ => None,
    };
    let dependencies = list(value, "dependencies")?
        .iter()
        .map(|d| {
            Ok(Dependency {
                node: ProcedureNodeId::new(number(d, "node")?)?,
                value_type: value_type(d, "type")?,
            })
        })
        .collect::<Fallible<_>>()?;
    Ok(Node {
        id: ProcedureNodeId::new(id)?,
        form,
        input: value_type(value, "input")?,
        output: value_type(value, "output")?,
        dependencies,
        prerequisites: node_ids(value, "prerequisites")?,
        preconditions: numbers(value, "preconditions")?
            .into_iter()
            .map(|n| named("predicate", n))
            .collect::<Fallible<_>>()?,
        postconditions: numbers(value, "postconditions")?
            .into_iter()
            .map(|n| named("predicate", n))
            .collect::<Fallible<_>>()?,
        context: named("context", number(value, "context")?)?,
        required_context: numbers(value, "required_context")?
            .into_iter()
            .map(|n| named("object", n))
            .collect::<Fallible<_>>()?,
        scope: numbers(value, "scope")?
            .into_iter()
            .map(|n| named("scope", n))
            .collect::<Fallible<_>>()?,
        resources: numbers(value, "resources")?
            .into_iter()
            .map(|n| named("resource", n))
            .collect::<Fallible<_>>()?,
        ceiling: capabilities(value, "ceiling")?,
        floor: gates(value, "floor")?,
        failure: optional_node(value, "failure")?,
        cancellation: optional_node(value, "cancellation")?,
        cost: ExecutionCost::new(number(value, "cost")?)?,
        adapter,
    })
}

fn resources(nodes: &[Node], ceiling: &[Capability]) -> Vec<ResourceDeclaration> {
    let mut declared: Vec<ResourceDeclaration> = Vec::new();
    for node in nodes {
        for id in &node.resources {
            if !declared.iter().any(|r| &r.id == id) {
                declared.push(ResourceDeclaration {
                    id: id.clone(),
                    context: node.context.clone(),
                    ceiling: ceiling.to_vec(),
                });
            }
        }
    }
    declared
}

fn scope(nodes: &[Node], package: &PackageRef) -> Vec<ScopeDeclaration> {
    let mut declared: Vec<ScopeDeclaration> = Vec::new();
    for id in nodes.iter().flat_map(|n| &n.scope) {
        if !declared.iter().any(|s| &s.id == id) {
            declared.push(ScopeDeclaration {
                id: id.clone(),
                package: package.clone(),
            });
        }
    }
    declared
}

fn adapters(nodes: &[Node], package: &PackageRef) -> Fallible<Vec<AdapterDeclaration>> {
    let mut declared = Vec::new();
    for node in nodes {
        if let (NodeForm::Effect(capability), Some(id)) = (&node.form, &node.adapter) {
            declared.push(AdapterDeclaration {
                id: id.clone(),
                package: package.clone(),
                export: ExportName::try_from("apply".to_owned())?,
                implementation: digest::<BlobDigest>(node.id.get()),
                input: node.input,
                output: node.output,
                capability: *capability,
            });
        }
    }
    Ok(declared)
}

fn gate_definitions(floor: &[Gate], nodes: &[Node]) -> Fallible<Vec<GateDefinition>> {
    let used: BTreeSet<u8> = floor
        .iter()
        .chain(nodes.iter().flat_map(|n| &n.floor))
        .map(|g| *g as u8)
        .collect();
    [Gate::Test, Gate::Review, Gate::Compatibility]
        .into_iter()
        .filter(|g| used.contains(&(*g as u8)))
        .map(|gate| {
            Ok(GateDefinition {
                gate,
                id: named("gate", gate as u64)?,
                definition: digest::<BlobDigest>(gate as u64),
                predicate: named("predicate", GATE_PREDICATE)?,
            })
        })
        .collect()
}

fn predicates(value: &Value, key: &str) -> Fallible<Vec<writfold_domain::ProcedurePredicateId>> {
    numbers(value, key)?
        .into_iter()
        .map(|n| named("predicate", n))
        .collect()
}

fn closure(
    value: &Value,
) -> Fallible<(
    Vec<ObjectDeclaration>,
    Vec<writfold_domain::ProcedureObjectId>,
)> {
    let ids = numbers(value, "closure")?;
    let declarations = ids
        .iter()
        .map(|n| {
            Ok(ObjectDeclaration {
                id: named("object", *n)?,
                blob: digest::<BlobDigest>(*n),
            })
        })
        .collect::<Fallible<_>>()?;
    let objects = ids
        .iter()
        .map(|n| named("object", *n))
        .collect::<Fallible<_>>()?;
    Ok((declarations, objects))
}

pub fn procedure(value: &Value, registry: &[PredicateSignature]) -> Fallible<Procedure> {
    let nodes = list(value, "nodes")?
        .iter()
        .map(node)
        .collect::<Fallible<Vec<_>>>()?;
    let ceiling = capabilities(value, "ceiling")?;
    let floor = gates(value, "floor")?;
    let own_package = package(number(value, "package")?)?;
    let (closure, objects) = closure(value)?;
    let contexts: BTreeSet<_> = nodes.iter().map(|n| n.context.clone()).collect();
    Ok(Procedure {
        revision: digest::<ProcedureRevision>(number(value, "revision")?),
        material: ProcedureMaterial {
            realm: "realm".parse()?,
            id: named("procedure", number(value, "id")?)?,
            skill: SkillRef {
                id: "skill".parse()?,
                revision: digest::<SkillRevision>(1),
            },
            dependency_lock: digest::<DependencyLockDigest>(1),
            closure,
            contexts: contexts
                .into_iter()
                .map(|id| ContextDeclaration {
                    id,
                    objects: objects.clone(),
                })
                .collect(),
            scope: scope(&nodes, &own_package),
            resources: resources(&nodes, &ceiling),
            adapters: adapters(&nodes, &own_package)?,
            gates: gate_definitions(&floor, &nodes)?,
            package: own_package,
            predicates: registry.to_vec(),
            input: value_type(value, "input")?,
            output: value_type(value, "output")?,
            ceiling,
            floor,
            rank: ProcedureRank::new(number(value, "rank")?)?,
            entry: ProcedureNodeId::new(number(value, "entry")?)?,
            nodes,
            preconditions: predicates(value, "preconditions")?,
            postconditions: predicates(value, "postconditions")?,
        },
    })
}
