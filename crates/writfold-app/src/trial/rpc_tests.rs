//! Typed refusal mapping and the response budget of the `serve` trial adapter.
use super::BeforeEffects;
use super::rpc::{Reply, refusal};
use serde_json::Value;
use writfold_domain::{ContentDigest, InstanceId, RequestGeneration, SessionIncarnation};
use writfold_wire::rpc::RequestId;
use writfold_wire::rpc::response::{
    MAX_RESPONSE_BYTES, ServerMetadata, ShowPayload, TrialCategory, TrialOutcome,
};
use writfold_wire::rpc::trial::TrialMethod;

type TestResult<T = (), E = Box<dyn std::error::Error>> = Result<T, E>;
type Boxed = Box<dyn std::error::Error + Send + Sync>;

#[test]
fn refusals_are_typed_and_writing_failures_are_unknown() {
    let decided: Boxed = Box::new(BeforeEffects("report refused: x".into()));
    let (category, outcome, message) = refusal(TrialMethod::Prepare, &*decided);
    assert_eq!(
        (category, outcome),
        (TrialCategory::Malformed, TrialOutcome::Rejected)
    );
    // The marker is invisible in the text: exactly what the CLI prints.
    assert_eq!(message, "report refused: x");
    let late: Boxed = "install failed".into();
    assert_eq!(
        refusal(TrialMethod::Prepare, &*late).1,
        TrialOutcome::Unknown
    );
    assert_eq!(
        refusal(TrialMethod::Retain, &*late).1,
        TrialOutcome::Unknown
    );
    assert_eq!(refusal(TrialMethod::Show, &*late).1, TrialOutcome::Rejected);
    let conflict: Boxed = Box::new(crate::ledger::LedgerError::Conflict);
    assert_eq!(
        refusal(TrialMethod::Retain, &*conflict).0,
        TrialCategory::Conflict
    );
    let denied: Boxed = Box::new(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
    assert_eq!(
        refusal(TrialMethod::Show, &*denied).0,
        TrialCategory::Permission
    );
    let wrapped: Boxed = Box::new(BeforeEffects(Box::new(std::io::Error::from(
        std::io::ErrorKind::NotFound,
    ))));
    assert_eq!(
        refusal(TrialMethod::Prepare, &*wrapped),
        (
            TrialCategory::NotFound,
            TrialOutcome::Rejected,
            "entity not found".to_owned()
        )
    );
}

#[test]
fn an_oversized_result_refuses_cleanly_and_is_never_truncated() -> TestResult {
    let instance: InstanceId = "stdio-instance".parse()?;
    let incarnation: SessionIncarnation = "stdio-session".parse()?;
    let digest: ContentDigest = "c".repeat(64).parse()?;
    let reply = Reply {
        id: RequestId::String("big"),
        metadata: ServerMetadata::new("1", &instance, &incarnation, &digest),
        generation: RequestGeneration::from_u64(3),
        maximum: MAX_RESPONSE_BYTES,
    };
    let text = "x".repeat(MAX_RESPONSE_BYTES);
    let payload = ShowPayload {
        run: "run.1.aaaaaa",
        text: &text,
    };
    let bytes = reply.deliver(&payload, TrialOutcome::Rejected, "too big")?;
    assert!(bytes.len() <= MAX_RESPONSE_BYTES);
    let value: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(value["id"], "big");
    assert_eq!(value["error"]["data"]["category"], "incomplete");
    assert_eq!(value["error"]["data"]["diagnostic"]["code"], "E105");
    assert_eq!(value["error"]["data"]["message"], "too big");
    assert!(value.get("result").is_none());
    let small = ShowPayload {
        run: "run.1.aaaaaa",
        text: "fits",
    };
    let value: Value =
        serde_json::from_slice(&reply.deliver(&small, TrialOutcome::Rejected, "too big")?)?;
    assert_eq!(value["result"]["payload"]["text"], "fits");
    Ok(())
}
