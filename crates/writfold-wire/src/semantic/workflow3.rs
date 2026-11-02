use super::codec::*;
use super::*;
use serde_json::{Value, json};
use writfold_domain::*;

pub fn dec_named_context(value: &Value) -> Result<NamedContext> {
    let v = obj(value, "id context")?;
    Ok(NamedContext::new(
        identity(field(v, "id")?)?,
        dec_context(field(v, "context")?)?,
    ))
}

pub fn enc_named_context(value: &NamedContext) -> Value {
    json!({
        "id": enc_identity(value.id()),
        "context": enc_context(value.context())
    })
}

pub fn dec_claim_assignment(value: &Value) -> Result<ClaimAssignment> {
    let v = obj(value, "claim context witness")?;
    Ok(ClaimAssignment::new(
        dec_ref(field(v, "claim")?)?,
        identity(field(v, "context")?)?,
        dec_witness(field(v, "witness")?)?,
    ))
}

pub fn enc_claim_assignment(value: &ClaimAssignment) -> Value {
    json!({
        "claim": enc_ref(value.claim()),
        "context": enc_identity(value.context()),
        "witness": enc_witness(value.witness())
    })
}

pub fn dec_portfolio_state(value: &Value) -> Result<PortfolioState> {
    let v = obj(value, "kb selected contexts")?;
    Ok(PortfolioState::new(
        dec_kb(field(v, "kb")?)?,
        items(field(v, "selected")?, dec_ref)?,
        items(field(v, "contexts")?, dec_named_context)?,
    ))
}

pub fn enc_portfolio_state(value: &PortfolioState) -> Value {
    json!({
        "kb": enc_kb(value.kb()),
        "selected": value.selected().iter().map(enc_ref).collect::<Vec<_>>(),
        "contexts": value.contexts().iter().map(enc_named_context).collect::<Vec<_>>()
    })
}

pub fn dec_portfolio_plan(value: &Value) -> Result<PortfolioPlan> {
    let v = obj(
        value,
        "id expected_snapshot candidate actor contexts assignments retired_contexts",
    )?;
    Ok(PortfolioPlan::new(
        identity(field(v, "id")?)?,
        identity(field(v, "expected_snapshot")?)?,
        dec_kb(field(v, "candidate")?)?,
        identity(field(v, "actor")?)?,
        items(field(v, "contexts")?, dec_named_context)?,
        items(field(v, "assignments")?, dec_claim_assignment)?,
        items(field(v, "retired_contexts")?, identity)?,
    ))
}

pub fn enc_portfolio_plan(value: &PortfolioPlan) -> Value {
    json!({
        "id": enc_identity(value.id()),
        "expected_snapshot": enc_identity(value.expected_snapshot()),
        "candidate": enc_kb(value.candidate()),
        "actor": enc_identity(value.actor()),
        "contexts": value.contexts().iter().map(enc_named_context).collect::<Vec<_>>(),
        "assignments": value.assignments().iter().map(enc_claim_assignment).collect::<Vec<_>>(),
        "retired_contexts": value.retired_contexts().iter().map(enc_identity).collect::<Vec<_>>()
    })
}

pub fn dec_verified_portfolio_approval(value: &Value) -> Result<ModelPortfolioApproval> {
    let v = obj(value, "plan")?;
    Ok(ModelPortfolioApproval::new(dec_portfolio_plan(field(
        v, "plan",
    )?)?))
}

pub fn enc_verified_portfolio_approval(value: &ModelPortfolioApproval) -> Value {
    json!({
        "plan": enc_portfolio_plan(value.plan())
    })
}

pub fn dec_portfolio_journal_entry(value: &Value) -> Result<PortfolioJournalEntry> {
    let v = obj(value, "plan")?;
    Ok(PortfolioJournalEntry::new(dec_portfolio_plan(field(
        v, "plan",
    )?)?))
}

pub fn enc_portfolio_journal_entry(value: &PortfolioJournalEntry) -> Value {
    json!({
        "plan": enc_portfolio_plan(value.plan())
    })
}

pub fn dec_portfolio_result(value: &Value) -> Result<PortfolioResult> {
    let v = obj(value, "state journal outcome")?;
    Ok(PortfolioResult::new(
        dec_portfolio_state(field(v, "state")?)?,
        items(field(v, "journal")?, dec_portfolio_journal_entry)?,
        dec_commit_outcome(field(v, "outcome")?)?,
    ))
}

pub fn enc_portfolio_result(value: &PortfolioResult) -> Value {
    json!({
        "state": enc_portfolio_state(value.state()),
        "journal": value.journal().iter().map(enc_portfolio_journal_entry).collect::<Vec<_>>(),
        "outcome": enc_commit_outcome(value.outcome())
    })
}

pub fn dec_portfolio_selection_plan(value: &Value) -> Result<PortfolioSelectionPlan> {
    let v = obj(
        value,
        "id expected_snapshot retained_snapshot new_snapshot actor contexts selected",
    )?;
    Ok(PortfolioSelectionPlan::new(
        identity(field(v, "id")?)?,
        identity(field(v, "expected_snapshot")?)?,
        identity(field(v, "retained_snapshot")?)?,
        identity(field(v, "new_snapshot")?)?,
        identity(field(v, "actor")?)?,
        items(field(v, "contexts")?, dec_named_context)?,
        items(field(v, "selected")?, dec_ref)?,
    ))
}

pub fn enc_portfolio_selection_plan(value: &PortfolioSelectionPlan) -> Value {
    json!({
        "id": enc_identity(value.id()),
        "expected_snapshot": enc_identity(value.expected_snapshot()),
        "retained_snapshot": enc_identity(value.retained_snapshot()),
        "new_snapshot": enc_identity(value.new_snapshot()),
        "actor": enc_identity(value.actor()),
        "contexts": value.contexts().iter().map(enc_named_context).collect::<Vec<_>>(),
        "selected": value.selected().iter().map(enc_ref).collect::<Vec<_>>()
    })
}

pub fn dec_verified_portfolio_selection_approval(
    value: &Value,
) -> Result<ModelPortfolioSelectionApproval> {
    let v = obj(value, "plan")?;
    Ok(ModelPortfolioSelectionApproval::new(
        dec_portfolio_selection_plan(field(v, "plan")?)?,
    ))
}

pub fn enc_verified_portfolio_selection_approval(value: &ModelPortfolioSelectionApproval) -> Value {
    json!({
        "plan": enc_portfolio_selection_plan(value.plan())
    })
}

pub fn dec_portfolio_selection_journal_entry(
    value: &Value,
) -> Result<PortfolioSelectionJournalEntry> {
    let v = obj(value, "plan")?;
    Ok(PortfolioSelectionJournalEntry::new(
        dec_portfolio_selection_plan(field(v, "plan")?)?,
    ))
}

pub fn enc_portfolio_selection_journal_entry(value: &PortfolioSelectionJournalEntry) -> Value {
    json!({
        "plan": enc_portfolio_selection_plan(value.plan())
    })
}

pub fn dec_portfolio_selection_result(value: &Value) -> Result<PortfolioSelectionResult> {
    let v = obj(value, "state journal outcome")?;
    Ok(PortfolioSelectionResult::new(
        dec_portfolio_state(field(v, "state")?)?,
        items(field(v, "journal")?, dec_portfolio_selection_journal_entry)?,
        dec_commit_outcome(field(v, "outcome")?)?,
    ))
}

pub fn enc_portfolio_selection_result(value: &PortfolioSelectionResult) -> Value {
    json!({
        "state": enc_portfolio_state(value.state()),
        "journal": value.journal().iter().map(enc_portfolio_selection_journal_entry).collect::<Vec<_>>(),
        "outcome": enc_commit_outcome(value.outcome())
    })
}

pub fn dec_implementation_binding(value: &Value) -> Result<ImplementationBinding> {
    let v = obj(
        value,
        "design_revision source target_revision actor observed_at",
    )?;
    Ok(ImplementationBinding::new(
        identity::<DesignRevision>(field(v, "design_revision")?)?,
        identity(field(v, "source")?)?,
        identity(field(v, "target_revision")?)?,
        identity(field(v, "actor")?)?,
        nat(field(v, "observed_at")?)?,
    ))
}

pub fn enc_implementation_binding(value: &ImplementationBinding) -> Value {
    json!({
        "design_revision": enc_identity(value.design_revision()),
        "source": enc_identity(value.source()),
        "target_revision": enc_identity(value.target_revision()),
        "actor": enc_identity(value.actor()),
        "observed_at": enc_integer(value.observed_at())
    })
}

pub fn dec_design_proposal(value: &Value) -> Result<DesignProposal> {
    let v = obj(value, "authored target_source binding")?;
    Ok(DesignProposal::new(
        dec_authored_proposal(field(v, "authored")?)?,
        identity(field(v, "target_source")?)?,
        if field(v, "binding")?.is_null() {
            None
        } else {
            Some(dec_implementation_binding(field(v, "binding")?)?)
        },
    ))
}

pub fn enc_design_proposal(value: &DesignProposal) -> Value {
    json!({
        "authored": enc_authored_proposal(value.authored()),
        "target_source": enc_identity(value.target_source()),
        "binding": value.binding().as_ref().map(enc_implementation_binding)
    })
}

pub fn dec_design_binding_command(value: &Value) -> Result<DesignBindingCommand> {
    let v = obj(value, "expected binding")?;
    Ok(DesignBindingCommand::new(
        dec_design_proposal(field(v, "expected")?)?,
        dec_implementation_binding(field(v, "binding")?)?,
    ))
}

pub fn enc_design_binding_command(value: &DesignBindingCommand) -> Value {
    json!({
        "expected": enc_design_proposal(value.expected()),
        "binding": enc_implementation_binding(value.binding())
    })
}

pub fn dec_design_reopen_command(value: &Value) -> Result<DesignReopenCommand> {
    let v = obj(value, "expected actor reason")?;
    Ok(DesignReopenCommand::new(
        dec_design_proposal(field(v, "expected")?)?,
        identity(field(v, "actor")?)?,
        text(field(v, "reason")?)?,
    ))
}

pub fn enc_design_reopen_command(value: &DesignReopenCommand) -> Value {
    json!({
        "expected": enc_design_proposal(value.expected()),
        "actor": enc_identity(value.actor()),
        "reason": value.reason()
    })
}
