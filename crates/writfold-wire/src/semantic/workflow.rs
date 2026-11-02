use super::codec::*;
use super::*;
use serde_json::{Value, json};
use writfold_domain::*;

pub fn dec_plan(value: &Value) -> Result<Plan> {
    let v = obj(
        value,
        "id expected_snapshot candidate authority claim witness",
    )?;
    Ok(Plan::new(
        identity(field(v, "id")?)?,
        identity(field(v, "expected_snapshot")?)?,
        dec_kb(field(v, "candidate")?)?,
        dec_authority(field(v, "authority")?)?,
        dec_ref(field(v, "claim")?)?,
        dec_witness(field(v, "witness")?)?,
    ))
}

pub fn enc_plan(value: &Plan) -> Value {
    json!({
        "id": enc_identity(value.id()),
        "expected_snapshot": enc_identity(value.expected_snapshot()),
        "candidate": enc_kb(value.candidate()),
        "authority": enc_authority(value.authority()),
        "claim": enc_ref(value.claim()),
        "witness": enc_witness(value.witness())
    })
}

pub fn dec_journal_entry(value: &Value) -> Result<JournalEntry> {
    Ok(JournalEntry::new(dec_plan(field(
        obj(value, "plan")?,
        "plan",
    )?)?))
}

pub fn enc_journal_entry(value: &JournalEntry) -> Value {
    json!({
        "plan": enc_plan(value.plan())
    })
}

pub fn dec_proposal(value: &Value) -> Result<Proposal> {
    let v = obj(
        value,
        "id revision target status actor executor acceptor gates",
    )?;
    Ok(Proposal::new(
        identity(field(v, "id")?)?,
        identity(field(v, "revision")?)?,
        dec_ref(field(v, "target")?)?,
        dec_proposal_status(field(v, "status")?)?,
        optional_identity(field(v, "actor")?)?,
        optional_identity(field(v, "executor")?)?,
        optional_identity(field(v, "acceptor")?)?,
        items(field(v, "gates")?, identity)?,
    ))
}

pub fn enc_proposal(value: &Proposal) -> Value {
    json!({
        "id": enc_identity(value.id()),
        "revision": enc_identity(value.revision()),
        "target": enc_ref(value.target()),
        "status": enc_proposal_status(value.status()),
        "actor": enc_optional_identity(value.actor()),
        "executor": enc_optional_identity(value.executor()),
        "acceptor": enc_optional_identity(value.acceptor()),
        "gates": value.gates().iter().map(enc_identity).collect::<Vec<_>>()
    })
}

pub fn dec_hypothesis(value: &Value) -> Result<Hypothesis> {
    let v = obj(value, "text status")?;
    Ok(Hypothesis::new(
        text(field(v, "text")?)?,
        dec_hypothesis_status(field(v, "status")?)?,
    ))
}

pub fn dec_investigation(value: &Value) -> Result<Investigation> {
    let v = obj(value, "checked_against scope_and_limits hypotheses")?;
    Ok(Investigation::new(
        text(field(v, "checked_against")?)?,
        text(field(v, "scope_and_limits")?)?,
        items(field(v, "hypotheses")?, dec_hypothesis)?,
    ))
}

pub fn enc_investigation(value: &Investigation) -> Value {
    json!({
        "checked_against": value.checked_against(),
        "scope_and_limits": value.scope_and_limits(),
        "hypotheses": value.hypotheses().iter().map(|hypothesis| json!({"text": hypothesis.text(), "status": enc_hypothesis_status(hypothesis.status())})).collect::<Vec<_>>()
    })
}

pub fn dec_authored_gate_definition(value: &Value) -> Result<AuthoredGateDefinition> {
    let v = obj(value, "id claim check when by checker")?;
    Ok(AuthoredGateDefinition::new(
        identity(field(v, "id")?)?,
        text(field(v, "claim")?)?,
        text(field(v, "check")?)?,
        dec_gate_when(field(v, "when")?)?,
        optional_identity(field(v, "by")?)?,
        optional_identity(field(v, "checker")?)?,
    ))
}

pub fn enc_authored_gate_definition(value: &AuthoredGateDefinition) -> Value {
    json!({
        "id": enc_identity(value.id()),
        "claim": value.claim(),
        "check": value.check(),
        "when": enc_gate_when(value.when()),
        "by": enc_optional_identity(value.by()),
        "checker": enc_optional_identity(value.checker())
    })
}

pub fn dec_authored_gate_result(value: &Value) -> Result<AuthoredGateResult> {
    let v = obj(
        value,
        "definition target_revision checker result observed_at expires_at proposal_revision environment scope evidence_refs actor",
    )?;
    Ok(AuthoredGateResult::new(
        dec_authored_gate_definition(field(v, "definition")?)?,
        optional_identity(field(v, "target_revision")?)?,
        optional_identity(field(v, "checker")?)?,
        dec_gate_outcome(field(v, "result")?)?,
        nat(field(v, "observed_at")?)?,
        nat(field(v, "expires_at")?)?,
        optional_identity(field(v, "proposal_revision")?)?,
        text(field(v, "environment")?)?,
        text(field(v, "scope")?)?,
        items(field(v, "evidence_refs")?, text)?,
        optional_identity(field(v, "actor")?)?,
    ))
}

pub fn enc_authored_gate_result(value: &AuthoredGateResult) -> Value {
    json!({
        "definition": enc_authored_gate_definition(value.definition()),
        "target_revision": enc_optional_identity(value.target_revision()),
        "checker": enc_optional_identity(value.checker()),
        "result": enc_gate_outcome(value.result()),
        "observed_at": enc_integer(value.observed_at()),
        "expires_at": enc_integer(value.expires_at()),
        "proposal_revision": enc_optional_identity(value.proposal_revision()),
        "environment": value.environment(),
        "scope": value.scope(),
        "evidence_refs": value.evidence_refs(),
        "actor": enc_optional_identity(value.actor())
    })
}

pub fn dec_authored_replacement(value: &Value) -> Result<AuthoredReplacement> {
    let v = obj(value, "id superseded_by")?;
    Ok(AuthoredReplacement::new(
        identity(field(v, "id")?)?,
        if field(v, "superseded_by")?.is_null() {
            None
        } else {
            Some(identity(field(v, "superseded_by")?)?)
        },
    ))
}

pub fn dec_authored_landed(value: &Value) -> Result<AuthoredLanded> {
    let v = obj(value, "reference target_revision")?;
    Ok(AuthoredLanded::new(
        text(field(v, "reference")?)?,
        identity(field(v, "target_revision")?)?,
    ))
}

pub fn dec_authored_proposal(value: &Value) -> Result<AuthoredProposal> {
    let v = obj(
        value,
        "status profile research_present alternatives_count target_revision gate_definitions gate_results actor acceptor executor verifier landed_refs regression_note id superseded_by proposals accepted_revision proposal_revision environment scope resolved_landed",
    )?;
    let definitions = items(field(v, "gate_definitions")?, dec_authored_gate_definition)?;
    if definitions
        .iter()
        .enumerate()
        .any(|(i, d)| definitions[..i].iter().any(|prior| prior.id() == d.id()))
    {
        return Err(InvalidRequest);
    }
    Ok(AuthoredProposal::new(
        dec_authored_status(field(v, "status")?)?,
        dec_authored_profile(field(v, "profile")?)?,
        boolean(field(v, "research_present")?)?,
        nat(field(v, "alternatives_count")?)?,
        optional_identity(field(v, "target_revision")?)?,
        definitions,
        items(field(v, "gate_results")?, dec_authored_gate_result)?,
        optional_identity(field(v, "actor")?)?,
        optional_identity(field(v, "acceptor")?)?,
        optional_identity(field(v, "executor")?)?,
        optional_identity(field(v, "verifier")?)?,
        items(field(v, "landed_refs")?, text)?,
        text(field(v, "regression_note")?)?,
        identity(field(v, "id")?)?,
        if field(v, "superseded_by")?.is_null() {
            None
        } else {
            Some(identity(field(v, "superseded_by")?)?)
        },
        items(field(v, "proposals")?, dec_authored_replacement)?,
        optional_identity(field(v, "accepted_revision")?)?,
        optional_identity(field(v, "proposal_revision")?)?,
        text(field(v, "environment")?)?,
        text(field(v, "scope")?)?,
        items(field(v, "resolved_landed")?, dec_authored_landed)?,
    ))
}

pub fn enc_authored_proposal(value: &AuthoredProposal) -> Value {
    json!({
        "status": enc_authored_status(value.status()),
        "profile": enc_authored_profile(value.profile()),
        "research_present": value.research_present(),
        "alternatives_count": enc_integer(value.alternatives_count()),
        "target_revision": enc_optional_identity(value.target_revision()),
        "gate_definitions": value.gate_definitions().iter().map(enc_authored_gate_definition).collect::<Vec<_>>(),
        "gate_results": value.gate_results().iter().map(enc_authored_gate_result).collect::<Vec<_>>(),
        "actor": enc_optional_identity(value.actor()),
        "acceptor": enc_optional_identity(value.acceptor()),
        "executor": enc_optional_identity(value.executor()),
        "verifier": enc_optional_identity(value.verifier()),
        "landed_refs": value.landed_refs(),
        "regression_note": value.regression_note(),
        "id": enc_identity(value.id()),
        "superseded_by": value.superseded_by().as_ref().map(enc_identity),
        "proposals": value.proposals().iter().map(|row| json!({"id": enc_identity(row.id()), "superseded_by": row.superseded_by().as_ref().map(enc_identity)})).collect::<Vec<_>>(),
        "accepted_revision": enc_optional_identity(value.accepted_revision()),
        "proposal_revision": enc_optional_identity(value.proposal_revision()),
        "environment": value.environment(),
        "scope": value.scope(),
        "resolved_landed": value.resolved_landed().iter().map(|resolution| json!({"reference": resolution.reference(), "target_revision": enc_identity(resolution.target_revision())})).collect::<Vec<_>>()
    })
}

pub fn dec_commit_outcome(value: &Value) -> Result<CommitOutcome> {
    match text(value)?.as_str() {
        "applied" => Ok(CommitOutcome::Applied),
        "already_applied" => Ok(CommitOutcome::AlreadyApplied),
        "input_conflict" => Ok(CommitOutcome::InputConflict),
        "predecessor_conflict" => Ok(CommitOutcome::PredecessorConflict),
        "rejected" => Ok(CommitOutcome::Rejected),
        _ => Err(InvalidRequest),
    }
}

pub fn enc_commit_outcome(value: &CommitOutcome) -> Value {
    Value::String(
        match value {
            CommitOutcome::Applied => "applied",
            CommitOutcome::AlreadyApplied => "already_applied",
            CommitOutcome::InputConflict => "input_conflict",
            CommitOutcome::PredecessorConflict => "predecessor_conflict",
            CommitOutcome::Rejected => "rejected",
        }
        .to_owned(),
    )
}

pub fn dec_proposal_status(value: &Value) -> Result<ProposalStatus> {
    match text(value)?.as_str() {
        "draft" => Ok(ProposalStatus::Draft),
        "proposed" => Ok(ProposalStatus::Proposed),
        "approved" => Ok(ProposalStatus::Approved),
        "applied" => Ok(ProposalStatus::Applied),
        "verified" => Ok(ProposalStatus::Verified),
        "rejected" => Ok(ProposalStatus::Rejected),
        _ => Err(InvalidRequest),
    }
}

pub fn enc_proposal_status(value: &ProposalStatus) -> Value {
    Value::String(
        match value {
            ProposalStatus::Draft => "draft",
            ProposalStatus::Proposed => "proposed",
            ProposalStatus::Approved => "approved",
            ProposalStatus::Applied => "applied",
            ProposalStatus::Verified => "verified",
            ProposalStatus::Rejected => "rejected",
        }
        .to_owned(),
    )
}

pub fn dec_hypothesis_status(value: &Value) -> Result<HypothesisStatus> {
    match text(value)?.as_str() {
        "supported" => Ok(HypothesisStatus::Supported),
        "refuted" => Ok(HypothesisStatus::Refuted),
        "open" => Ok(HypothesisStatus::Open),
        "unk" => Ok(HypothesisStatus::Unknown),
        _ => Err(InvalidRequest),
    }
}

pub fn enc_hypothesis_status(value: &HypothesisStatus) -> Value {
    Value::String(
        match value {
            HypothesisStatus::Supported => "supported",
            HypothesisStatus::Refuted => "refuted",
            HypothesisStatus::Open => "open",
            HypothesisStatus::Unknown => "unk",
        }
        .to_owned(),
    )
}

pub fn dec_authored_status(value: &Value) -> Result<AuthoredStatus> {
    match text(value)?.as_str() {
        "draft" => Ok(AuthoredStatus::Draft),
        "proposed" => Ok(AuthoredStatus::Proposed),
        "accepted" => Ok(AuthoredStatus::Accepted),
        "rejected" => Ok(AuthoredStatus::Rejected),
        "landing" => Ok(AuthoredStatus::Landing),
        "applied" => Ok(AuthoredStatus::Applied),
        "verified" => Ok(AuthoredStatus::Verified),
        "regressed" => Ok(AuthoredStatus::Regressed),
        "deferred" => Ok(AuthoredStatus::Deferred),
        "superseded" => Ok(AuthoredStatus::Superseded),
        _ => Err(InvalidRequest),
    }
}

pub fn enc_authored_status(value: &AuthoredStatus) -> Value {
    Value::String(
        match value {
            AuthoredStatus::Draft => "draft",
            AuthoredStatus::Proposed => "proposed",
            AuthoredStatus::Accepted => "accepted",
            AuthoredStatus::Rejected => "rejected",
            AuthoredStatus::Landing => "landing",
            AuthoredStatus::Applied => "applied",
            AuthoredStatus::Verified => "verified",
            AuthoredStatus::Regressed => "regressed",
            AuthoredStatus::Deferred => "deferred",
            AuthoredStatus::Superseded => "superseded",
        }
        .to_owned(),
    )
}

pub fn dec_authored_profile(value: &Value) -> Result<AuthoredProfile> {
    match text(value)?.as_str() {
        "modern" => Ok(AuthoredProfile::Modern),
        "legacy_limited" => Ok(AuthoredProfile::LegacyLimited),
        _ => Err(InvalidRequest),
    }
}

pub fn enc_authored_profile(value: &AuthoredProfile) -> Value {
    Value::String(
        match value {
            AuthoredProfile::Modern => "modern",
            AuthoredProfile::LegacyLimited => "legacy_limited",
        }
        .to_owned(),
    )
}

pub fn dec_gate_when(value: &Value) -> Result<GateWhen> {
    match text(value)?.as_str() {
        "land" => Ok(GateWhen::Land),
        "deploy" => Ok(GateWhen::Deploy),
        "use" => Ok(GateWhen::Use),
        _ => Err(InvalidRequest),
    }
}

pub fn enc_gate_when(value: &GateWhen) -> Value {
    Value::String(
        match value {
            GateWhen::Land => "land",
            GateWhen::Deploy => "deploy",
            GateWhen::Use => "use",
        }
        .to_owned(),
    )
}

pub fn dec_gate_outcome(value: &Value) -> Result<GateOutcome> {
    match text(value)?.as_str() {
        "pass" => Ok(GateOutcome::Pass),
        "fail" => Ok(GateOutcome::Fail),
        "unk" => Ok(GateOutcome::Unknown),
        _ => Err(InvalidRequest),
    }
}

pub fn enc_gate_outcome(value: &GateOutcome) -> Value {
    Value::String(
        match value {
            GateOutcome::Pass => "pass",
            GateOutcome::Fail => "fail",
            GateOutcome::Unknown => "unk",
        }
        .to_owned(),
    )
}
