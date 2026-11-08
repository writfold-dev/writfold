//! Closed node-form typing (Lean `FormTyped`) with production declaration checks.
use super::declarations::Declarations;
use super::nodes::{backward, forward};
use super::{Catalog, Result, Rule, fail, subset, unique};
use writfold_domain::engineering::dependency::{DependencyKind, call_rank_allowed};
use writfold_domain::engineering::execution::{Capability, ValueType};
use writfold_domain::engineering::procedure::{CallTarget, Node, NodeForm};
use writfold_domain::{ProcedureNodeId, ProcedureObjectId};

pub(super) fn check(
    declarations: &Declarations<'_>,
    catalog: &Catalog<'_>,
    node: &Node,
) -> Result<()> {
    match &node.form {
        NodeForm::Read(object) => read(declarations, node, object),
        NodeForm::Select(routes) => select(declarations, node, routes),
        NodeForm::Judge(id) | NodeForm::Check(id) => {
            declarations.predicate(id, node.input, node.output)?;
            if node.output != ValueType::Evidence {
                return Err(fail(Rule::FormType, "predicate output"));
            }
            Ok(())
        }
        NodeForm::Call(target) => call(declarations, catalog, node, target),
        NodeForm::Effect(capability) => effect(declarations, node, *capability),
        NodeForm::Join(calls) => join(declarations, node, calls),
        NodeForm::End => {
            if node.output != declarations.material.output {
                return Err(fail(Rule::FormType, "end output"));
            }
            Ok(())
        }
    }
}

fn read(declarations: &Declarations<'_>, node: &Node, object: &ProcedureObjectId) -> Result<()> {
    if !declarations.objects.contains_key(object) {
        return Err(fail(Rule::UnknownReference, "read object"));
    }
    if node.output != ValueType::Artifact {
        return Err(fail(Rule::FormType, "read output"));
    }
    Ok(())
}

fn select(declarations: &Declarations<'_>, node: &Node, routes: &[ProcedureNodeId]) -> Result<()> {
    if routes.is_empty() {
        return Err(fail(Rule::FormType, "select routes"));
    }
    unique(routes, "select routes")?;
    for id in routes {
        forward(declarations, node, id)?;
    }
    Ok(())
}

/// Calls name an exact catalog pair, strictly descend in rank, delegate no more
/// than the node ceiling, and the callee needs no more than it is delegated.
fn call(
    declarations: &Declarations<'_>,
    catalog: &Catalog<'_>,
    node: &Node,
    target: &CallTarget,
) -> Result<()> {
    unique(&target.delegation, "delegation")?;
    let callee = catalog
        .get(&(&target.procedure, &target.revision))
        .ok_or_else(|| fail(Rule::CallIdentity, "callee"))?;
    let callee = &callee.material;
    if callee.input != target.input
        || callee.output != target.output
        || node.input != target.input
        || node.output != target.output
    {
        return Err(fail(Rule::FormType, "call signature"));
    }
    let caller_rank = declarations.material.rank.get();
    if !call_rank_allowed(
        DependencyKind::InvokeProcedure,
        caller_rank,
        callee.rank.get(),
    ) {
        return Err(fail(Rule::CallRank, "callee rank"));
    }
    subset(
        &target.delegation,
        &node.ceiling,
        Rule::Capability,
        "delegation",
    )?;
    subset(
        &callee.ceiling,
        &target.delegation,
        Rule::Capability,
        "callee ceiling",
    )?;
    subset(&node.floor, &callee.floor, Rule::Floor, "callee floor")?;
    if !declarations.package_declared(&callee.package) {
        return Err(fail(Rule::UnknownReference, "callee package"));
    }
    Ok(())
}

/// Effects need a node capability, nonempty resources that each permit it, and
/// an adapter declared with exactly this capability and value signature.
fn effect(declarations: &Declarations<'_>, node: &Node, capability: Capability) -> Result<()> {
    if !node.ceiling.contains(&capability) {
        return Err(fail(Rule::Capability, "effect"));
    }
    if node.resources.is_empty() {
        return Err(fail(Rule::Resource, "effect resources"));
    }
    for id in &node.resources {
        let permits = declarations
            .resources
            .get(id)
            .is_some_and(|resource| resource.ceiling.contains(&capability));
        if !permits {
            return Err(fail(Rule::Capability, "resource capability"));
        }
    }
    let adapter = node
        .adapter
        .as_ref()
        .and_then(|id| declarations.adapters.get(id))
        .ok_or_else(|| fail(Rule::Adapter, "effect adapter"))?;
    if adapter.capability != capability
        || adapter.input != node.input
        || adapter.output != node.output
    {
        return Err(fail(Rule::Adapter, "adapter signature"));
    }
    Ok(())
}

fn join(declarations: &Declarations<'_>, node: &Node, calls: &[ProcedureNodeId]) -> Result<()> {
    if calls.is_empty() {
        return Err(fail(Rule::FormType, "join calls"));
    }
    unique(calls, "join calls")?;
    for id in calls {
        let source = backward(declarations, node, id, "join call")?;
        if !matches!(source.form, NodeForm::Call(_)) {
            return Err(fail(Rule::FormType, "join call"));
        }
    }
    Ok(())
}
