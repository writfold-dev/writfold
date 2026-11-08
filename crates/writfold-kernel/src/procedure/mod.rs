//! Finite structural procedure admission refining `Engineering/Procedure.lean`.
//!
//! Every Lean `ProcedureTyped` conjunct has a stricter production check here, plus
//! explicit declaration, pin and duplicate-freedom refinements. This is ordinary
//! Rust: it is not a proved translation of the Lean judgment. Only the reused
//! `call_rank_allowed` guard and finite-chain lemma carry existing Verus proofs.
//! Admission establishes no catalog authenticity, content digest, gate truth,
//! current resource, adapter safety, grant, or permission to execute anything.
mod declarations;
mod forms;
mod nodes;

use declarations::Declarations;
use std::{collections::BTreeMap, error::Error, fmt};
use writfold_domain::ProcedureNodeId;
use writfold_domain::engineering::procedure::{Procedure, ProcedureInput};

/// The closed refusal vocabulary. Each value names the violated rule family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rule {
    Duplicate,
    Entry,
    MissingEnd,
    UnknownReference,
    PredicateType,
    DependencyType,
    BackwardDependency,
    ForwardRoute,
    FormType,
    Capability,
    Floor,
    CallRank,
    CallIdentity,
    Adapter,
    Context,
    Resource,
    Realm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmissionError {
    pub rule: Rule,
    pub field: &'static str,
    pub node: Option<ProcedureNodeId>,
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.node {
            Some(node) => write!(
                f,
                "procedure {:?} violation in {} at node {}",
                self.rule,
                self.field,
                node.get()
            ),
            None => write!(f, "procedure {:?} violation in {}", self.rule, self.field),
        }
    }
}
impl Error for AdmissionError {}

pub type Result<T, E = AdmissionError> = std::result::Result<T, E>;

fn fail(rule: Rule, field: &'static str) -> AdmissionError {
    AdmissionError {
        rule,
        field,
        node: None,
    }
}

/// The exact immutable supplied input after structural admission. It retains
/// every declaration and list order; it is not an execution or effect capability.
#[derive(Debug)]
pub struct CheckedProcedure {
    input: ProcedureInput,
}

impl CheckedProcedure {
    pub fn input(&self) -> &ProcedureInput {
        &self.input
    }
    pub fn procedure(&self) -> &Procedure {
        &self.input.procedure
    }
    pub fn into_input(self) -> ProcedureInput {
        self.input
    }
}

/// Catalog keyed by the exact pair (logical ProcedureId, ProcedureRevision).
type Catalog<'a> = BTreeMap<
    (
        &'a writfold_domain::ProcedureId,
        &'a writfold_domain::engineering::ProcedureRevision,
    ),
    &'a Procedure,
>;

/// Admit the entire finite supplied catalog, not merely the called signatures.
/// All procedures must share the root's realm; the input is returned unchanged.
pub fn validate(input: ProcedureInput) -> Result<CheckedProcedure> {
    check(&input)?;
    Ok(CheckedProcedure { input })
}

fn check(input: &ProcedureInput) -> Result<()> {
    let registry = index(&input.registry, |p| &p.id, "registry")?;
    let realm = &input.procedure.material.realm;
    let mut catalog: Catalog<'_> = BTreeMap::new();
    for procedure in std::iter::once(&input.procedure).chain(&input.catalog) {
        let key = (&procedure.material.id, &procedure.revision);
        if catalog.insert(key, procedure).is_some() {
            return Err(fail(Rule::Duplicate, "catalog"));
        }
        if &procedure.material.realm != realm {
            return Err(fail(Rule::Realm, "catalog realm"));
        }
    }
    for procedure in std::iter::once(&input.procedure).chain(&input.catalog) {
        let declarations = Declarations::admit(&procedure.material, &registry)?;
        nodes::check_all(&declarations, &catalog)?;
    }
    Ok(())
}

/// Index declarations by exact identity, refusing a repeated key.
fn index<'a, T, K: Ord>(
    items: &'a [T],
    key: impl Fn(&'a T) -> &'a K,
    field: &'static str,
) -> Result<BTreeMap<&'a K, &'a T>> {
    let mut map = BTreeMap::new();
    for item in items {
        if map.insert(key(item), item).is_some() {
            return Err(fail(Rule::Duplicate, field));
        }
    }
    Ok(map)
}

fn unique<T: Eq>(items: &[T], field: &'static str) -> Result<()> {
    for (position, item) in items.iter().enumerate() {
        if items.iter().take(position).any(|earlier| earlier == item) {
            return Err(fail(Rule::Duplicate, field));
        }
    }
    Ok(())
}

fn subset<T: Eq>(small: &[T], large: &[T], rule: Rule, field: &'static str) -> Result<()> {
    if small.iter().all(|item| large.contains(item)) {
        Ok(())
    } else {
        Err(fail(rule, field))
    }
}
