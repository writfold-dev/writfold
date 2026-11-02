//! Version-2 semantic request dispatch after strict DTO validation.
use super::*;
use serde_json::{Value, json};
use writfold_domain::*;
use writfold_kernel::{checker, time, workflow as kernel};

const BASE: &str = "schema_version semantic_version id op";
pub fn request_fields<'a>(
    request: &'a Value,
    fields: &str,
) -> Result<&'a serde_json::Map<String, Value>> {
    obj(request, &format!("{BASE} {fields}"))
}
pub fn handle(
    request: &Value,
    approvals: &[ModelApproval],
    selections: &[ModelSelectionApproval],
    retained: &[KB],
) -> Result<Value> {
    let r = object(request)?;
    if nat(field(r, "schema_version")?)? != Integer::from(2)
        || text(field(r, "semantic_version")?)? != "2.0.0"
    {
        return Err(InvalidRequest);
    }
    nonempty(field(r, "id")?)?;
    let op = text(field(r, "op")?)?;
    match op.as_str() {
        "semantic_version" => {
            request_fields(request, "")?;
            Ok(json!("2.0.0"))
        }
        "fresh" => {
            request_fields(request, "now observed_at ttl")?;
            let result = time::assess_fresh(
                &dec_ttl(&r["ttl"])?,
                &nat(&r["observed_at"])?,
                &nat(&r["now"])?,
            );
            Ok(json!(match result {
                Freshness::Fresh => "fresh",
                Freshness::Stale => "stale",
                Freshness::Unknown => "unknown",
            }))
        }
        "validate" => {
            request_fields(request, "kb")?;
            dec_kb(&r["kb"])?;
            Ok(json!(true))
        }
        "conclude" => {
            request_fields(request, "investigation")?;
            Ok(json!(kernel::conclude(&dec_investigation(
                &r["investigation"]
            )?)))
        }
        "authored_transition" | "authored_candidate_valid" => {
            request_fields(request, "proposal target_status now")?;
            let proposal = dec_authored_proposal(&r["proposal"])?;
            let target = dec_authored_status(&r["target_status"])?;
            let now = nat(&r["now"])?;
            Ok(json!(if op == "authored_transition" {
                kernel::authored_transition(&proposal, target, &now)
            } else {
                kernel::authored_candidate_valid(&proposal, target, &now)
            }))
        }
        "select" | "retrieve_selected" => handle_selection(request, &op, selections, retained),
        _ => handle_contextual(request, &op, approvals),
    }
}
fn handle_contextual(request: &Value, op: &str, approvals: &[ModelApproval]) -> Result<Value> {
    let fields = match op {
        "check" => "witness",
        "supported" => "claim polarity",
        "assess" | "usable" => "claim",
        "reuse" => "claim receipt",
        "retrieve" => "offset limit",
        "admit" => "plan",
        "sequence" => "journal plans",
        "transition" => "proposal target_status",
        _ => return Err(InvalidRequest),
    };
    let r = request_fields(request, &format!("kb context {fields}"))?;
    let kb = dec_kb(&r["kb"])?;
    let context = dec_context(&r["context"])?;
    match op {
        "check" => Ok(enc_checked(
            checker::check(&kb, &context, &dec_witness(&r["witness"])?).as_ref(),
        )),
        "supported" => Ok(json!(checker::supported(
            &kb,
            &context,
            &dec_ref(&r["claim"])?,
            polarity(&r["polarity"])?
        ))),
        "assess" | "usable" => {
            let result = checker::assess(&kb, &context, &dec_ref(&r["claim"])?);
            Ok(if op == "usable" {
                json!(result.usable())
            } else {
                enc_assessment(&result)
            })
        }
        "reuse" => Ok(json!(checker::reusable(
            &kb,
            &context,
            &dec_ref(&r["claim"])?,
            &dec_receipt(&r["receipt"])?
        ))),
        "retrieve" => retrieve(
            &kb,
            &context,
            kb.claims().iter(),
            &nat(&r["offset"])?,
            &nat(&r["limit"])?,
        ),
        "transition" => Ok(json!(kernel::transition(
            &kb,
            &context,
            &dec_proposal(&r["proposal"])?,
            dec_proposal_status(&r["target_status"])?
        ))),
        "admit" => {
            let plan = dec_plan(&r["plan"])?;
            let result = kernel::apply_plan(&kb, &context, &[], &plan, approvals);
            Ok(
                json!({"kb": enc_kb(result.kb()), "journal": result.journal().iter().map(enc_journal_entry).collect::<Vec<_>>(), "outcome": outcome(result.outcome())}),
            )
        }
        "sequence" => sequence(&kb, &context, &r["journal"], &r["plans"], approvals),
        _ => Err(InvalidRequest),
    }
}
pub fn retrieve<'a>(
    kb: &KB,
    context: &Context,
    claims: impl Iterator<Item = &'a Claim>,
    offset: &Integer,
    limit: &Integer,
) -> Result<Value> {
    if !checker::context_matches(kb, context) {
        return Err(InvalidRequest);
    }
    let mut relevant = Integer::from(0);
    let mut records = Vec::new();
    let end = offset + limit;
    for claim in claims.filter(|claim| narrower(context.target(), claim.statement().scope())) {
        if &relevant >= offset && relevant < end {
            records.push(json!({"claim": enc_claim(claim), "assessment": enc_assessment(&checker::assess(kb, context, &claim.claim_ref()))}));
        }
        relevant = &relevant + &Integer::from(1);
    }
    let consumed = offset + &Integer::from(records.len());
    let omitted = if relevant > consumed {
        &relevant - &consumed
    } else {
        Integer::from(0)
    };
    let complete = omitted == Integer::from(0);
    Ok(
        json!({"snapshot": kb.snapshot().as_str(), "records": records, "omitted": enc_integer(&omitted), "complete": complete,
        "next_offset": if complete { Value::Null } else { enc_integer(&consumed) }}),
    )
}
fn handle_selection(
    request: &Value,
    op: &str,
    approvals: &[ModelSelectionApproval],
    retained: &[KB],
) -> Result<Value> {
    let r = request_fields(
        request,
        if op == "select" {
            "state context journal plan"
        } else {
            "state context offset limit"
        },
    )?;
    let state = dec_selection_state(&r["state"])?;
    let context = dec_context(&r["context"])?;
    if op == "retrieve_selected" {
        return retrieve(
            state.kb(),
            &context,
            state.kb().claims().iter().filter(|claim| {
                state
                    .selected()
                    .iter()
                    .any(|reference| claim.matches_ref(reference))
            }),
            &nat(&r["offset"])?,
            &nat(&r["limit"])?,
        );
    }
    let journal = items(&r["journal"], dec_selection_journal_entry)?;
    if !unique(
        &journal
            .iter()
            .map(|entry| entry.plan().id())
            .collect::<Vec<_>>(),
    ) {
        return Err(InvalidRequest);
    }
    let plan = dec_selection_plan(&r["plan"])?;
    let result = kernel::apply_selection(&state, &context, &journal, &plan, approvals, retained);
    Ok(
        json!({"state": enc_selection_state(result.state()), "journal": result.journal().iter().map(enc_selection_journal_entry).collect::<Vec<_>>(), "outcome": outcome(result.outcome())}),
    )
}
fn sequence(
    kb: &KB,
    context: &Context,
    journal: &Value,
    plans: &Value,
    approvals: &[ModelApproval],
) -> Result<Value> {
    if !checker::context_matches(kb, context) {
        return Err(InvalidRequest);
    }
    let mut journal = items(journal, dec_journal_entry)?;
    if !unique(
        &journal
            .iter()
            .map(|entry| entry.plan().id())
            .collect::<Vec<_>>(),
    ) {
        return Err(InvalidRequest);
    }
    let plans = items(plans, dec_plan)?;
    let mut kb = kb.clone();
    let mut outcomes = Vec::new();
    for plan in &plans {
        let framed = Context::new(
            kb.snapshot().clone(),
            context.semantic_version().clone(),
            context.ruleset().clone(),
            context.checker().clone(),
            context.policy().clone(),
            context.now().clone(),
            context.target().clone(),
            context.intended_use().clone(),
            context.sources().clone(),
        );
        let applied = kernel::apply_plan(&kb, &framed, &journal, plan, approvals);
        let (next_kb, next_journal, result) = applied.into_parts();
        outcomes.push(outcome(&result));
        let next_kb = next_kb.into_owned();
        let next_journal = next_journal.into_owned();
        kb = next_kb;
        journal = next_journal;
    }
    Ok(
        json!({"kb": enc_kb(&kb), "journal": journal.iter().map(enc_journal_entry).collect::<Vec<_>>(), "outcomes": outcomes}),
    )
}
pub fn outcome(value: &CommitOutcome) -> &'static str {
    match value {
        CommitOutcome::Applied => "applied",
        CommitOutcome::AlreadyApplied => "already_applied",
        CommitOutcome::InputConflict => "input_conflict",
        CommitOutcome::PredecessorConflict => "predecessor_conflict",
        CommitOutcome::Rejected => "rejected",
    }
}
pub fn polarity(value: &Value) -> Result<Polarity> {
    match value.as_str() {
        Some("positive") => Ok(Polarity::Positive),
        Some("negative") => Ok(Polarity::Negative),
        _ => Err(InvalidRequest),
    }
}
