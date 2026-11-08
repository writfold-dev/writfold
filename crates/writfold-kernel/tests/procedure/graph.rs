//! Node-level refusals: lists, typed edges, routes, authority, context,
//! resources and every closed node form, with node attribution.
use super::fixture::{TestResult, at, call_target, dependency, id};
use super::refused;
use writfold_domain::ProcedureRank;
use writfold_domain::engineering::execution::{Capability, ValueType};
use writfold_domain::engineering::procedure::NodeForm;
use writfold_domain::engineering::{EngineeringDigest, ProcedureRevision};
use writfold_kernel::procedure::Rule;

#[test]
fn edges_routes_and_lists() -> TestResult {
    refused(
        |p| {
            at(p, 2)?
                .dependencies
                .push(dependency(1, ValueType::Artifact)?);
            Ok(())
        },
        Rule::Duplicate,
        "dependencies",
        Some(2),
    )?;
    refused(
        |p| {
            at(p, 2)?.dependencies = vec![dependency(2, ValueType::Artifact)?];
            Ok(())
        },
        Rule::BackwardDependency,
        "dependency",
        Some(2),
    )?;
    refused(
        |p| {
            at(p, 2)?.dependencies = vec![dependency(4, ValueType::Artifact)?];
            Ok(())
        },
        Rule::BackwardDependency,
        "dependency",
        Some(2),
    )?;
    refused(
        |p| {
            at(p, 2)?.dependencies = vec![dependency(1, ValueType::Evidence)?];
            Ok(())
        },
        Rule::DependencyType,
        "dependency",
        Some(2),
    )
}

#[test]
fn prerequisites_and_routes() -> TestResult {
    refused(
        |p| {
            at(p, 4)?.prerequisites = vec![id(5)?];
            Ok(())
        },
        Rule::BackwardDependency,
        "prerequisite",
        Some(4),
    )?;
    refused(
        |p| {
            at(p, 2)?.failure = Some(id(1)?);
            Ok(())
        },
        Rule::ForwardRoute,
        "route",
        Some(2),
    )?;
    refused(
        |p| {
            at(p, 2)?.cancellation = Some(id(9)?);
            Ok(())
        },
        Rule::UnknownReference,
        "route",
        Some(2),
    )?;
    refused(
        |p| {
            at(p, 1)?.form = NodeForm::Select(vec![]);
            Ok(())
        },
        Rule::FormType,
        "select routes",
        Some(1),
    )?;
    refused(
        |p| {
            at(p, 1)?.form = NodeForm::Select(vec![id(1)?]);
            Ok(())
        },
        Rule::ForwardRoute,
        "route",
        Some(1),
    )
}

#[test]
fn authority_context_and_resources() -> TestResult {
    refused(
        |p| {
            at(p, 1)?.ceiling.push(Capability::Publish);
            Ok(())
        },
        Rule::Capability,
        "node ceiling",
        Some(1),
    )?;
    refused(
        |p| {
            at(p, 1)?.floor.clear();
            Ok(())
        },
        Rule::Floor,
        "node floor",
        Some(1),
    )?;
    refused(
        |p| {
            at(p, 1)?.context = "other".parse()?;
            Ok(())
        },
        Rule::UnknownReference,
        "node context",
        Some(1),
    )?;
    refused(
        |p| {
            at(p, 1)?.required_context = vec!["missing".parse()?];
            Ok(())
        },
        Rule::Context,
        "required context",
        Some(1),
    )?;
    refused(
        |p| {
            at(p, 1)?.scope = vec!["unknown".parse()?];
            Ok(())
        },
        Rule::UnknownReference,
        "node scope",
        Some(1),
    )?;
    refused(
        |p| {
            at(p, 1)?.resources = vec!["absent".parse()?];
            Ok(())
        },
        Rule::UnknownReference,
        "node resource",
        Some(1),
    )?;
    refused(
        |p| {
            at(p, 1)?.adapter = Some("materialize".parse()?);
            Ok(())
        },
        Rule::Adapter,
        "non-effect adapter",
        Some(1),
    )
}

#[test]
fn forms() -> TestResult {
    refused(
        |p| {
            at(p, 1)?.form = NodeForm::Read("missing".parse()?);
            Ok(())
        },
        Rule::UnknownReference,
        "read object",
        Some(1),
    )?;
    refused(
        |p| {
            at(p, 2)?.form = NodeForm::Check("ready".parse()?);
            Ok(())
        },
        Rule::PredicateType,
        "predicate",
        Some(2),
    )?;
    refused(
        |p| {
            at(p, 4)?.form = NodeForm::Join(vec![id(2)?]);
            Ok(())
        },
        Rule::FormType,
        "join call",
        Some(4),
    )?;
    refused(
        |p| {
            at(p, 4)?.form = NodeForm::Join(vec![]);
            Ok(())
        },
        Rule::FormType,
        "join calls",
        Some(4),
    )?;
    refused(
        |p| {
            at(p, 6)?.output = ValueType::Evidence;
            Ok(())
        },
        Rule::FormType,
        "end output",
        Some(6),
    )
}

#[test]
fn calls() -> TestResult {
    refused(
        |p| {
            let mut t = call_target()?;
            t.revision = ProcedureRevision::from_digest([12; 32]);
            at(p, 3)?.form = NodeForm::Call(t);
            Ok(())
        },
        Rule::CallIdentity,
        "callee",
        Some(3),
    )?;
    refused(
        |p| {
            let mut t = call_target()?;
            t.output = ValueType::Evidence;
            at(p, 3)?.form = NodeForm::Call(t);
            Ok(())
        },
        Rule::FormType,
        "call signature",
        Some(3),
    )?;
    refused(
        |p| {
            p.procedure.material.rank = ProcedureRank::new(0)?;
            Ok(())
        },
        Rule::CallRank,
        "callee rank",
        Some(3),
    )?;
    refused(
        |p| {
            let mut t = call_target()?;
            t.delegation = vec![];
            at(p, 3)?.form = NodeForm::Call(t);
            Ok(())
        },
        Rule::Capability,
        "callee ceiling",
        Some(3),
    )?;
    refused(
        |p| {
            let mut t = call_target()?;
            t.delegation.push(Capability::Read);
            at(p, 3)?.form = NodeForm::Call(t);
            Ok(())
        },
        Rule::Duplicate,
        "delegation",
        Some(3),
    )?;
    refused(
        |p| {
            at(p, 3)?.ceiling = vec![Capability::Check];
            Ok(())
        },
        Rule::Capability,
        "delegation",
        Some(3),
    )
}

#[test]
fn effects() -> TestResult {
    refused(
        |p| {
            at(p, 5)?.ceiling = vec![Capability::Read];
            Ok(())
        },
        Rule::Capability,
        "effect",
        Some(5),
    )?;
    refused(
        |p| {
            at(p, 5)?.resources.clear();
            Ok(())
        },
        Rule::Resource,
        "effect resources",
        Some(5),
    )?;
    refused(
        |p| {
            at(p, 5)?.adapter = None;
            Ok(())
        },
        Rule::Adapter,
        "effect adapter",
        Some(5),
    )?;
    refused(
        |p| {
            at(p, 5)?.form = NodeForm::Effect(Capability::Check);
            Ok(())
        },
        Rule::Capability,
        "resource capability",
        Some(5),
    )?;
    refused(
        |p| {
            let n = at(p, 5)?;
            n.output = ValueType::Evidence;
            Ok(())
        },
        Rule::Adapter,
        "adapter signature",
        Some(5),
    )
}
