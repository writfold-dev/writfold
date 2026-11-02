//! Workflow-3 reference operations with externally supplied model assumptions.
use super::request::{outcome, request_fields};
use super::*;
use serde_json::{Value, json};
use writfold_domain::*;
use writfold_kernel::{design_workflow as design, portfolio as kernel};

pub fn handle(
    request: &Value,
    approvals: &[ModelPortfolioApproval],
    selections: &[ModelPortfolioSelectionApproval],
    retained: &[PortfolioState],
) -> Result<Value> {
    let r = object(request)?;
    if nat(field(r, "schema_version")?)? != Integer::from(3)
        || text(field(r, "semantic_version")?)? != "2.0.0"
    {
        return Err(InvalidRequest);
    }
    nonempty(field(r, "id")?)?;
    let op = field(r, "op")?.as_str().ok_or(InvalidRequest)?;
    match op {
        "semantic_version" | "workflow_version" => {
            request_fields(request, "")?;
            Ok(if op == "semantic_version" {
                json!("2.0.0")
            } else {
                json!(3)
            })
        }
        "portfolio_state_valid" => {
            request_fields(request, "state")?;
            Ok(json!(kernel::portfolio_state_valid(&dec_portfolio_state(
                &r["state"]
            )?)))
        }
        "portfolio_valid" => {
            request_fields(request, "state plan")?;
            Ok(json!(kernel::portfolio_plan_valid(
                &dec_portfolio_state(&r["state"])?,
                &dec_portfolio_plan(&r["plan"])?
            )))
        }
        "portfolio_admit" => {
            request_fields(request, "state journal plan")?;
            let state = dec_portfolio_state(&r["state"])?;
            let journal = items(&r["journal"], dec_portfolio_journal_entry)?;
            let plan = dec_portfolio_plan(&r["plan"])?;
            let result = kernel::apply_portfolio(&state, &journal, &plan, approvals);
            Ok(
                json!({"state": enc_portfolio_state(result.state()), "journal": result.journal().iter().map(enc_portfolio_journal_entry).collect::<Vec<_>>(), "outcome": outcome(result.outcome())}),
            )
        }
        "portfolio_query_allowed" | "portfolio_assess" => handle_query(request, op),
        "portfolio_selection_valid" => {
            request_fields(request, "state plan")?;
            Ok(json!(kernel::portfolio_selection_valid(
                &dec_portfolio_state(&r["state"])?,
                &dec_portfolio_selection_plan(&r["plan"])?,
                retained
            )))
        }
        "portfolio_select" => {
            request_fields(request, "state journal plan")?;
            let state = dec_portfolio_state(&r["state"])?;
            let journal = items(&r["journal"], dec_portfolio_selection_journal_entry)?;
            let plan = dec_portfolio_selection_plan(&r["plan"])?;
            let result =
                kernel::apply_portfolio_selection(&state, &journal, &plan, selections, retained);
            Ok(
                json!({"state": enc_portfolio_state(result.state()), "journal": result.journal().iter().map(enc_portfolio_selection_journal_entry).collect::<Vec<_>>(), "outcome": outcome(result.outcome())}),
            )
        }
        _ => handle_design(request, op),
    }
}

fn handle_query(request: &Value, operation: &str) -> Result<Value> {
    let r = request_fields(
        request,
        if operation == "portfolio_assess" {
            "state context_id context claim"
        } else {
            "state context_id context"
        },
    )?;
    let state = dec_portfolio_state(&r["state"])?;
    let id: ContextId = identity(&r["context_id"])?;
    let context = dec_context(&r["context"])?;
    if operation == "portfolio_query_allowed" {
        return Ok(json!(kernel::query_context_allowed(&state, &id, &context)));
    }
    Ok(
        kernel::assess_portfolio(&state, &id, &context, &dec_ref(&r["claim"])?)
            .as_ref()
            .map(enc_assessment)
            .unwrap_or(Value::Null),
    )
}
fn handle_design(request: &Value, op: &str) -> Result<Value> {
    match op {
        "design_valid" | "design_transition" => {
            let r = request_fields(request, "proposal target_status now")?;
            let proposal = dec_design_proposal(&r["proposal"])?;
            let target = dec_authored_status(&r["target_status"])?;
            let now = nat(&r["now"])?;
            Ok(json!(if op == "design_valid" {
                design::design_candidate_valid(&proposal, target, &now)
            } else {
                design::design_transition(&proposal, target, &now)
            }))
        }
        "design_bind" => {
            let r = request_fields(request, "proposal command now")?;
            Ok(design::bind_implementation(
                &dec_design_proposal(&r["proposal"])?,
                &dec_design_binding_command(&r["command"])?,
                &nat(&r["now"])?,
            )
            .as_ref()
            .map(enc_design_proposal)
            .unwrap_or(Value::Null))
        }
        "design_reopen" => {
            let r = request_fields(request, "proposal command")?;
            Ok(design::reopen_design(
                &dec_design_proposal(&r["proposal"])?,
                &dec_design_reopen_command(&r["command"])?,
            )
            .as_ref()
            .map(enc_design_proposal)
            .unwrap_or(Value::Null))
        }
        _ => Err(InvalidRequest),
    }
}
