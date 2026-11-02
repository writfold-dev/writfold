use super::codec::*;
use super::*;
use serde_json::{Value, json};
use writfold_domain::*;

pub fn dec_selection_state(value: &Value) -> Result<SelectionState> {
    let v = obj(value, "kb selected")?;
    let state = SelectionState::new(
        dec_kb(field(v, "kb")?)?,
        items(field(v, "selected")?, dec_ref)?,
    );
    if !unique(state.selected())
        || state
            .selected()
            .iter()
            .any(|r| !state.kb().claims().iter().any(|c| c.matches_ref(r)))
    {
        return Err(InvalidRequest);
    }
    Ok(state)
}

pub fn enc_selection_state(value: &SelectionState) -> Value {
    json!({
        "kb": enc_kb(value.kb()),
        "selected": value.selected().iter().map(enc_ref).collect::<Vec<_>>()
    })
}

pub fn dec_selection_plan(value: &Value) -> Result<SelectionPlan> {
    let v = obj(
        value,
        "id expected_snapshot target_snapshot new_snapshot selected actor policy intended_use context",
    )?;
    let selected = items(field(v, "selected")?, dec_ref)?;
    if !unique(&selected) {
        return Err(InvalidRequest);
    }
    Ok(SelectionPlan::new(
        identity(field(v, "id")?)?,
        identity(field(v, "expected_snapshot")?)?,
        identity(field(v, "target_snapshot")?)?,
        identity(field(v, "new_snapshot")?)?,
        selected,
        optional_identity(field(v, "actor")?)?,
        identity(field(v, "policy")?)?,
        text(field(v, "intended_use")?)?,
        dec_context(field(v, "context")?)?,
    ))
}

pub fn enc_selection_plan(value: &SelectionPlan) -> Value {
    json!({
        "id": enc_identity(value.id()),
        "expected_snapshot": enc_identity(value.expected_snapshot()),
        "target_snapshot": enc_identity(value.target_snapshot()),
        "new_snapshot": enc_identity(value.new_snapshot()),
        "selected": value.selected().iter().map(enc_ref).collect::<Vec<_>>(),
        "actor": enc_optional_identity(value.actor()),
        "policy": enc_identity(value.policy()),
        "intended_use": value.intended_use(),
        "context": enc_context(value.context())
    })
}

pub fn dec_selection_journal_entry(value: &Value) -> Result<SelectionJournalEntry> {
    Ok(SelectionJournalEntry::new(dec_selection_plan(field(
        obj(value, "plan")?,
        "plan",
    )?)?))
}

pub fn enc_selection_journal_entry(value: &SelectionJournalEntry) -> Value {
    json!({
        "plan": enc_selection_plan(value.plan())
    })
}

pub fn enc_selection_result(value: &SelectionResult) -> Value {
    json!({
        "state": enc_selection_state(value.state()),
        "journal": value.journal().iter().map(enc_selection_journal_entry).collect::<Vec<_>>(),
        "outcome": enc_commit_outcome(value.outcome())
    })
}

pub fn dec_selection_result(value: &Value) -> Result<SelectionResult> {
    let v = obj(value, "state journal outcome")?;
    Ok(SelectionResult::new(
        dec_selection_state(field(v, "state")?)?,
        items(field(v, "journal")?, dec_selection_journal_entry)?,
        dec_commit_outcome(field(v, "outcome")?)?,
    ))
}
