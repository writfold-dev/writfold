use crate::{Error, RawJson, canonical::canonical_prefixed};
use serde_json::Value;

/// Historical schema 2 is for retained verification, never new signing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalDomain {
    HistoricalSchema2,
    WorkflowSchema3,
}

impl ApprovalDomain {
    pub fn prefix(self) -> &'static [u8] {
        match self {
            Self::HistoricalSchema2 => b"kctx-operation-v2\0",
            Self::WorkflowSchema3 => b"kctx-operation-v3\0",
        }
    }
}

/// Raw envelope classifications, not semantically validated operation records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationVariant {
    LegacyAdmissionPlan,
    LegacyReleaseSelectionPlan,
    AdmissionPlan,
    ReleaseSelectionPlan,
}

impl OperationVariant {
    pub fn domain(self) -> ApprovalDomain {
        match self {
            Self::LegacyAdmissionPlan | Self::LegacyReleaseSelectionPlan => {
                ApprovalDomain::HistoricalSchema2
            }
            Self::AdmissionPlan | Self::ReleaseSelectionPlan => ApprovalDomain::WorkflowSchema3,
        }
    }

    /// Select exactly one version/kind and check its outer field set/types.
    /// Nested semantic records and nominal identities are deliberately left to
    /// the typed codec, not interpreted or reconstructed by this byte boundary.
    pub fn select(raw: &RawJson) -> Result<Self, Error> {
        let fields = raw
            .value()
            .as_object()
            .ok_or(envelope("expected operation object"))?;
        let version = fields.get("schema_version").and_then(Value::as_u64);
        let kind = fields.get("kind").and_then(Value::as_str);
        let variant = match (version, kind) {
            (Some(2), Some("admission")) => Self::LegacyAdmissionPlan,
            (Some(2), Some("selection")) => Self::LegacyReleaseSelectionPlan,
            (Some(3), Some("admission")) => Self::AdmissionPlan,
            (Some(3), Some("selection")) => Self::ReleaseSelectionPlan,
            _ => return Err(envelope("unsupported operation schema or kind")),
        };
        let admission = matches!(variant, Self::LegacyAdmissionPlan | Self::AdmissionPlan);
        let legacy = variant.domain() == ApprovalDomain::HistoricalSchema2;
        let extra = if admission { "captures" } else { "retained" };
        let context = if legacy { "context" } else { "target_use" };
        let expected = [
            "schema_version",
            "kind",
            "id",
            "base",
            "policy",
            extra,
            context,
            "plan",
            "toolchain",
        ];
        if fields.len() != expected.len() || expected.iter().any(|key| !fields.contains_key(*key)) {
            return Err(envelope("unexpected or missing operation fields"));
        }
        for key in ["id", "policy", "toolchain"] {
            if !fields.get(key).is_some_and(Value::is_string) {
                return Err(envelope("operation identity requires a string"));
            }
        }
        let base_valid = fields
            .get("base")
            .is_some_and(|value| value.is_string() || (admission && value.is_null()));
        let extra_valid = fields.get(extra).is_some_and(|value| {
            if admission {
                value
                    .as_array()
                    .is_some_and(|items| items.iter().all(Value::is_string))
            } else {
                value.is_string()
            }
        });
        let context_valid = fields.get(context).is_some_and(|value| {
            if legacy {
                value.is_object()
            } else {
                value.is_string()
            }
        });
        if !base_valid
            || !extra_valid
            || !context_valid
            || !fields.get("plan").is_some_and(Value::is_object)
        {
            return Err(envelope("invalid operation field type"));
        }
        Ok(variant)
    }

    /// Full outer plan bytes are signed, not a digest, approval envelope or
    /// semantic `plan` subobject. No domain fallback or verification is done.
    pub fn approval_message(self, raw: &RawJson, max_bytes: usize) -> Result<Vec<u8>, Error> {
        if Self::select(raw)? != self {
            return Err(envelope(
                "operation variant differs from requested signing domain",
            ));
        }
        canonical_prefixed(raw, self.domain().prefix(), max_bytes)
    }
}

fn envelope(reason: &'static str) -> Error {
    Error::Envelope { reason }
}
