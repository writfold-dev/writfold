use super::*;
use crate::{
    RawJson, canonical,
    semantic::{InvalidRequest, enc_portfolio_plan, enc_portfolio_selection_plan},
};
use serde_json::{Value, json};

impl Operation {
    /// Prepare a new v3 input. The supplied semantic ID is replaced in the new
    /// value only; an existing prepared operation is never accepted or mutated.
    pub fn prepare_admission(
        base: Option<ReleaseId>,
        policy: PolicyDigest,
        captures: Vec<SourceCaptureId>,
        target_use: String,
        semantic: PortfolioPlan,
        toolchain: ContentDigest,
        limits: Limits,
    ) -> Result<Self, BoundaryError> {
        let value = json!({
            "schema_version": 3, "kind": "admission", "id": "",
            "base": base.as_ref().map(ReleaseId::as_str), "policy": policy.as_str(),
            "captures": captures.iter().map(SourceCaptureId::as_str).collect::<Vec<_>>(),
            "target_use": target_use, "plan": enc_portfolio_plan(&semantic),
            "toolchain": toolchain.as_str(),
        });
        prepare(value, limits)
    }

    /// Prepare a new forward-selection input, without upgrading any legacy input.
    pub fn prepare_selection(
        base: ReleaseId,
        retained: ReleaseId,
        policy: PolicyDigest,
        target_use: String,
        semantic: PortfolioSelectionPlan,
        toolchain: ContentDigest,
        limits: Limits,
    ) -> Result<Self, BoundaryError> {
        let value = json!({
            "schema_version": 3, "kind": "selection", "id": "",
            "base": base.as_str(), "retained": retained.as_str(), "policy": policy.as_str(),
            "target_use": target_use, "plan": enc_portfolio_selection_plan(&semantic),
            "toolchain": toolchain.as_str(),
        });
        prepare(value, limits)
    }
}

fn prepare(value: Value, limits: Limits) -> Result<Operation, BoundaryError> {
    let mut raw = RawJson { value };
    let id = codec::identity_material(&mut raw, limits.bytes())?;
    let outer = raw.value.as_object_mut().ok_or(InvalidRequest)?;
    outer.insert("id".into(), Value::String(id.clone()));
    outer
        .get_mut("plan")
        .and_then(Value::as_object_mut)
        .ok_or(InvalidRequest)?
        .insert("id".into(), Value::String(id));
    Operation::decode(&canonical(&raw, limits.bytes())?, limits)
}
