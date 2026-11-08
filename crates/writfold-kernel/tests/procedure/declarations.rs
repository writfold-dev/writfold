//! Declaration-level refusals: catalog, realm, registry, ceilings, gates,
//! contexts, resources, adapters, entry, end and procedure conditions.
use super::fixture::{TestResult, at, id, node, package, signature};
use super::refused;
use writfold_domain::engineering::execution::{Capability, Gate, ValueType};
use writfold_domain::engineering::procedure::NodeForm;
use writfold_domain::{ProcedureObjectId, ProcedurePredicateId, RealmId};
use writfold_kernel::procedure::Rule;

#[test]
fn catalog_identity_realm_and_registry() -> TestResult {
    refused(
        |p| {
            p.catalog.push(p.catalog.first().cloned().ok_or("leaf")?);
            Ok(())
        },
        Rule::Duplicate,
        "catalog",
        None,
    )?;
    refused(
        |p| {
            let realm: RealmId = "elsewhere".parse()?;
            p.catalog
                .iter_mut()
                .for_each(|c| c.material.realm = realm.clone());
            Ok(())
        },
        Rule::Realm,
        "catalog realm",
        None,
    )?;
    refused(
        |p| {
            p.registry.push(signature("ready", ValueType::Unit)?);
            Ok(())
        },
        Rule::Duplicate,
        "registry",
        None,
    )?;
    refused(
        |p| {
            p.registry.retain(|s| s.id.as_str() != "ready");
            Ok(())
        },
        Rule::PredicateType,
        "registered signature",
        None,
    )
}

#[test]
fn procedure_declarations() -> TestResult {
    refused(
        |p| {
            let n = node(3, NodeForm::End, ValueType::Unit, ValueType::Artifact)?;
            p.procedure.material.nodes.push(n);
            Ok(())
        },
        Rule::Duplicate,
        "nodes",
        None,
    )?;
    refused(
        |p| {
            p.procedure.material.ceiling.push(Capability::Read);
            Ok(())
        },
        Rule::Duplicate,
        "ceiling",
        None,
    )?;
    refused(
        |p| {
            p.procedure.material.floor.push(Gate::Test);
            Ok(())
        },
        Rule::UnknownReference,
        "gate definition",
        None,
    )?;
    refused(
        |p| {
            let gates = &mut p.procedure.material.gates;
            gates.push(gates.first().cloned().ok_or("gate")?);
            Ok(())
        },
        Rule::Duplicate,
        "gates",
        None,
    )?;
    refused(
        |p| {
            let absent: ProcedurePredicateId = "absent".parse()?;
            p.procedure
                .material
                .gates
                .iter_mut()
                .for_each(|g| g.predicate = absent.clone());
            Ok(())
        },
        Rule::UnknownReference,
        "gate predicate",
        None,
    )
}

#[test]
fn procedure_contexts_resources_and_adapters() -> TestResult {
    refused(
        |p| {
            let object: ProcedureObjectId = "source".parse()?;
            p.procedure
                .material
                .contexts
                .iter_mut()
                .for_each(|c| c.objects.push(object.clone()));
            Ok(())
        },
        Rule::Duplicate,
        "context objects",
        None,
    )?;
    refused(
        |p| {
            p.procedure
                .material
                .resources
                .iter_mut()
                .for_each(|r| r.ceiling.push(Capability::Publish));
            Ok(())
        },
        Rule::Capability,
        "resource ceiling",
        None,
    )?;
    refused(
        |p| {
            let foreign = package(8)?;
            p.procedure
                .material
                .adapters
                .iter_mut()
                .for_each(|a| a.package = foreign.clone());
            Ok(())
        },
        Rule::UnknownReference,
        "adapter package",
        None,
    )
}

#[test]
fn entry_end_and_conditions() -> TestResult {
    refused(
        |p| {
            p.procedure.material.entry = id(9)?;
            Ok(())
        },
        Rule::Entry,
        "entry",
        None,
    )?;
    refused(
        |p| {
            p.procedure.material.entry = id(2)?;
            Ok(())
        },
        Rule::Entry,
        "entry",
        None,
    )?;
    refused(
        |p| {
            p.procedure
                .material
                .nodes
                .retain(|n| n.form != NodeForm::End);
            at(p, 2)?.failure = None;
            Ok(())
        },
        Rule::MissingEnd,
        "nodes",
        None,
    )?;
    refused(
        |p| {
            p.procedure.material.postconditions.push("ready".parse()?);
            Ok(())
        },
        Rule::PredicateType,
        "predicate",
        None,
    )
}
