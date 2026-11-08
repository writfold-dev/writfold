//! Per-node structure: duplicate-free lists, ceilings, floors, context, scope,
//! resources, typed backward edges, conditions and forward routes (Lean `NodeTyped`).
use super::declarations::Declarations;
use super::{Catalog, Result, Rule, fail, forms, subset, unique};
use writfold_domain::ProcedureNodeId;
use writfold_domain::engineering::procedure::{Node, NodeForm};

/// Check every node, attributing the first violation to its node identity.
pub(super) fn check_all(declarations: &Declarations<'_>, catalog: &Catalog<'_>) -> Result<()> {
    for node in &declarations.material.nodes {
        check(declarations, catalog, node).map_err(|mut error| {
            error.node = Some(node.id);
            error
        })?;
    }
    Ok(())
}

fn check(declarations: &Declarations<'_>, catalog: &Catalog<'_>, node: &Node) -> Result<()> {
    lists(node)?;
    authority(declarations, node)?;
    context(declarations, node)?;
    edges(declarations, node)?;
    declarations.conditions(&node.preconditions, node.input)?;
    declarations.conditions(&node.postconditions, node.output)?;
    for target in node.failure.iter().chain(&node.cancellation) {
        forward(declarations, node, target)?;
    }
    if !matches!(node.form, NodeForm::Effect(_)) && node.adapter.is_some() {
        return Err(fail(Rule::Adapter, "non-effect adapter"));
    }
    forms::check(declarations, catalog, node)
}

/// Positive cost and node identity are checked domain quantities already.
fn lists(node: &Node) -> Result<()> {
    for (position, dependency) in node.dependencies.iter().enumerate() {
        let mut earlier = node.dependencies.iter().take(position);
        if earlier.any(|d| d.node == dependency.node) {
            return Err(fail(Rule::Duplicate, "dependencies"));
        }
    }
    unique(&node.prerequisites, "prerequisites")?;
    unique(&node.required_context, "required context")?;
    unique(&node.scope, "node scope")?;
    unique(&node.resources, "node resources")?;
    unique(&node.ceiling, "node ceiling")?;
    unique(&node.floor, "node floor")
}

/// Node ceilings narrow the procedure ceiling; node floors widen its floor.
fn authority(declarations: &Declarations<'_>, node: &Node) -> Result<()> {
    let material = declarations.material;
    subset(
        &node.ceiling,
        &material.ceiling,
        Rule::Capability,
        "node ceiling",
    )?;
    subset(&material.floor, &node.floor, Rule::Floor, "node floor")?;
    declarations.gate_definitions(&node.floor)
}

fn context(declarations: &Declarations<'_>, node: &Node) -> Result<()> {
    let context = declarations
        .contexts
        .get(&node.context)
        .ok_or_else(|| fail(Rule::UnknownReference, "node context"))?;
    subset(
        &node.required_context,
        &context.objects,
        Rule::Context,
        "required context",
    )?;
    if !node
        .scope
        .iter()
        .all(|id| declarations.scope.contains_key(id))
    {
        return Err(fail(Rule::UnknownReference, "node scope"));
    }
    for id in &node.resources {
        let resource = declarations
            .resources
            .get(id)
            .ok_or_else(|| fail(Rule::UnknownReference, "node resource"))?;
        if resource.context != node.context {
            return Err(fail(Rule::Resource, "resource context"));
        }
    }
    Ok(())
}

/// Data edges and prerequisites point strictly backward: a finite DAG certificate.
fn edges(declarations: &Declarations<'_>, node: &Node) -> Result<()> {
    for dependency in &node.dependencies {
        let source = backward(declarations, node, &dependency.node, "dependency")?;
        if source.output != dependency.value_type || dependency.value_type != node.input {
            return Err(fail(Rule::DependencyType, "dependency"));
        }
    }
    for id in &node.prerequisites {
        backward(declarations, node, id, "prerequisite")?;
    }
    Ok(())
}

pub(super) fn backward<'a>(
    declarations: &Declarations<'a>,
    node: &Node,
    id: &ProcedureNodeId,
    field: &'static str,
) -> Result<&'a Node> {
    let source = declarations
        .nodes
        .get(id)
        .copied()
        .ok_or_else(|| fail(Rule::UnknownReference, field))?;
    if source.id >= node.id {
        return Err(fail(Rule::BackwardDependency, field));
    }
    Ok(source)
}

/// Routes (failure, cancellation, selection) point strictly forward.
pub(super) fn forward(
    declarations: &Declarations<'_>,
    node: &Node,
    id: &ProcedureNodeId,
) -> Result<()> {
    let target = declarations
        .nodes
        .get(id)
        .ok_or_else(|| fail(Rule::UnknownReference, "route"))?;
    if target.id <= node.id {
        return Err(fail(Rule::ForwardRoute, "route"));
    }
    Ok(())
}
