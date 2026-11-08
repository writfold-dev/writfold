//! Procedure-level declarations: indexes, registry agreement, ceilings, gates,
//! contexts, resources, adapters, entry, end and procedure conditions.
use super::{Result, Rule, fail, index, subset, unique};
use std::collections::BTreeMap;
use writfold_domain::engineering::dependency::PackageRef;
use writfold_domain::engineering::execution::{Gate, ValueType};
use writfold_domain::engineering::procedure::*;
use writfold_domain::{
    AdapterId, ContextId, ProcedureDependencyId, ProcedureNodeId, ProcedureObjectId,
    ProcedurePredicateId, ResourceId,
};

pub(super) type Registry<'a> = BTreeMap<&'a ProcedurePredicateId, &'a PredicateSignature>;

/// Exact-identity indexes over one procedure's declarations. Constructed only
/// after every procedure-level declaration rule has been checked.
pub(super) struct Declarations<'a> {
    pub material: &'a ProcedureMaterial,
    pub nodes: BTreeMap<&'a ProcedureNodeId, &'a Node>,
    pub objects: BTreeMap<&'a ProcedureObjectId, &'a ObjectDeclaration>,
    pub contexts: BTreeMap<&'a ContextId, &'a ContextDeclaration>,
    pub scope: BTreeMap<&'a ProcedureDependencyId, &'a ScopeDeclaration>,
    pub resources: BTreeMap<&'a ResourceId, &'a ResourceDeclaration>,
    pub adapters: BTreeMap<&'a AdapterId, &'a AdapterDeclaration>,
    pub predicates: Registry<'a>,
}

impl<'a> Declarations<'a> {
    pub fn admit(material: &'a ProcedureMaterial, registry: &Registry<'_>) -> Result<Self> {
        let declarations = Self {
            material,
            nodes: index(&material.nodes, |n| &n.id, "nodes")?,
            objects: index(&material.closure, |o| &o.id, "closure")?,
            contexts: index(&material.contexts, |c| &c.id, "contexts")?,
            scope: index(&material.scope, |s| &s.id, "scope")?,
            resources: index(&material.resources, |r| &r.id, "resources")?,
            adapters: index(&material.adapters, |a| &a.id, "adapters")?,
            predicates: index(&material.predicates, |s| &s.id, "predicates")?,
        };
        declarations.registered(registry)?;
        declarations.gates()?;
        declarations.contexts_and_resources()?;
        declarations.adapters()?;
        declarations.entry_and_end()?;
        declarations.conditions(&material.preconditions, material.input)?;
        declarations.conditions(&material.postconditions, material.output)?;
        Ok(declarations)
    }

    /// Each locally declared signature must equal the supplied registry entry.
    fn registered(&self, registry: &Registry<'_>) -> Result<()> {
        for signature in &self.material.predicates {
            if registry.get(&signature.id).copied() != Some(signature) {
                return Err(fail(Rule::PredicateType, "registered signature"));
            }
        }
        Ok(())
    }

    fn gates(&self) -> Result<()> {
        let material = self.material;
        unique(&material.ceiling, "ceiling")?;
        unique(&material.floor, "floor")?;
        for (position, definition) in material.gates.iter().enumerate() {
            let mut earlier = material.gates.iter().take(position);
            if earlier.any(|g| g.gate == definition.gate || g.id == definition.id) {
                return Err(fail(Rule::Duplicate, "gates"));
            }
            let predicate = self
                .predicates
                .get(&definition.predicate)
                .ok_or_else(|| fail(Rule::UnknownReference, "gate predicate"))?;
            if predicate.output != ValueType::Evidence {
                return Err(fail(Rule::PredicateType, "gate predicate"));
            }
        }
        self.gate_definitions(&material.floor)
    }

    /// Every floor gate must have exactly one pinned definition.
    pub fn gate_definitions(&self, floor: &[Gate]) -> Result<()> {
        if floor
            .iter()
            .all(|gate| self.material.gates.iter().any(|g| g.gate == *gate))
        {
            Ok(())
        } else {
            Err(fail(Rule::UnknownReference, "gate definition"))
        }
    }

    fn contexts_and_resources(&self) -> Result<()> {
        for context in &self.material.contexts {
            unique(&context.objects, "context objects")?;
            if !context
                .objects
                .iter()
                .all(|id| self.objects.contains_key(id))
            {
                return Err(fail(Rule::UnknownReference, "context object"));
            }
        }
        for resource in &self.material.resources {
            unique(&resource.ceiling, "resource ceiling")?;
            subset(
                &resource.ceiling,
                &self.material.ceiling,
                Rule::Capability,
                "resource ceiling",
            )?;
            if !self.contexts.contains_key(&resource.context) {
                return Err(fail(Rule::UnknownReference, "resource context"));
            }
        }
        Ok(())
    }

    /// The procedure's own package or an explicitly declared scope package.
    pub fn package_declared(&self, package: &PackageRef) -> bool {
        *package == self.material.package
            || self.material.scope.iter().any(|s| s.package == *package)
    }

    fn adapters(&self) -> Result<()> {
        for adapter in &self.material.adapters {
            if !self.package_declared(&adapter.package) {
                return Err(fail(Rule::UnknownReference, "adapter package"));
            }
            if !self.material.ceiling.contains(&adapter.capability) {
                return Err(fail(Rule::Capability, "adapter capability"));
            }
        }
        Ok(())
    }

    fn entry_and_end(&self) -> Result<()> {
        let material = self.material;
        let entry = self
            .nodes
            .get(&material.entry)
            .ok_or_else(|| fail(Rule::Entry, "entry"))?;
        if entry.input != material.input
            || !entry.dependencies.is_empty()
            || !entry.prerequisites.is_empty()
        {
            return Err(fail(Rule::Entry, "entry"));
        }
        if material.nodes.iter().any(|n| n.form == NodeForm::End) {
            Ok(())
        } else {
            Err(fail(Rule::MissingEnd, "nodes"))
        }
    }

    /// Exact predicate signature lookup in this procedure's declared registry.
    pub fn predicate(
        &self,
        id: &ProcedurePredicateId,
        input: ValueType,
        output: ValueType,
    ) -> Result<()> {
        let signature = self
            .predicates
            .get(id)
            .ok_or_else(|| fail(Rule::UnknownReference, "predicate"))?;
        if signature.input != input || signature.output != output {
            return Err(fail(Rule::PredicateType, "predicate"));
        }
        Ok(())
    }

    /// Pre/postconditions are duplicate-free Evidence predicates over `input`.
    pub fn conditions(&self, ids: &[ProcedurePredicateId], input: ValueType) -> Result<()> {
        unique(ids, "conditions")?;
        for id in ids {
            self.predicate(id, input, ValueType::Evidence)?;
        }
        Ok(())
    }
}
