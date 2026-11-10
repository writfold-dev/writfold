//! Bounded `writfold.trial/1` results and refusals.
//!
//! A result is the shutdown-shaped envelope (`exit_code`, `complete`,
//! `generation`, `session`, `payload`). A refusal reuses the control fault
//! envelope, but its detail is the application's own message: exactly the text
//! the CLI prints after `writfold: `, never more host detail than that.
use super::{MAX_RESPONSE_BYTES, ResponseError, ServerMetadata, bounded::Bounded, codec::Session};
use crate::rpc::RequestId;
use serde::Serialize;
use writfold_domain::RequestGeneration;

/// The diagnostic class of a trial refusal (the shared E1xx registry).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrialCategory {
    /// E101: a named file, repository or object was not found.
    NotFound,
    /// E102: the application refused the request's inputs.
    Malformed,
    /// E104: the ledger refused the step (out of order, stale head, held lease).
    Conflict,
    /// E105: the operation did not complete, or its result exceeds the budget.
    Incomplete,
    /// E106: the host refused access.
    Permission,
}

impl TrialCategory {
    fn names(self) -> (&'static str, &'static str) {
        match self {
            Self::NotFound => ("not-found", "E101"),
            Self::Malformed => ("malformed", "E102"),
            Self::Conflict => ("conflict", "E104"),
            Self::Incomplete => ("incomplete", "E105"),
            Self::Permission => ("permission", "E106"),
        }
    }
}

/// Whether the refused call is known to have written nothing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrialOutcome {
    /// Decided before any write, or the method is read-only.
    Rejected,
    /// A writing method failed after it may have written; observe before retry.
    Unknown,
}

#[derive(Clone, Copy, Debug)]
pub struct TrialRefusal<'a> {
    pub category: TrialCategory,
    pub outcome: TrialOutcome,
    pub message: &'a str,
}

#[derive(Serialize)]
struct Diagnostic<'a> {
    code: &'static str,
    severity: &'static str,
    message: &'a str,
    labels: [(); 0],
    help: Option<&'static str>,
}

#[derive(Serialize)]
struct Data<'a> {
    schema_version: u64,
    category: &'static str,
    message: &'a str,
    diagnostic: Diagnostic<'a>,
    exit_code: Option<u8>,
    complete: bool,
    generation: Option<u64>,
    session: Session<'a>,
    outcome: &'static str,
}

#[derive(Serialize)]
struct Error<'a> {
    code: i32,
    message: &'static str,
    data: Data<'a>,
}

#[derive(Serialize)]
struct Result<'a, T> {
    exit_code: u8,
    complete: bool,
    generation: u64,
    session: Session<'a>,
    payload: &'a T,
}

/// `trial/show` payload: the CLI's exact rendering of the validated history.
#[derive(Serialize)]
pub struct ShowPayload<'a> {
    pub run: &'a str,
    pub text: &'a str,
}

/// `trial/retain` payload: the binding a Weft report cites.
#[derive(Serialize)]
pub struct RetainPayload<'a> {
    pub binding: &'a str,
    pub file: &'a str,
    pub commit: &'a str,
    pub start: u64,
    pub end: u64,
}

#[derive(Serialize)]
pub struct SpliceSummary {
    pub start: u64,
    pub end: u64,
    pub inserted: u64,
}

/// `trial/prepare` payload. The worktree path is not reported (the CLI does
/// not print it either); the next step is the CLI's `writfold trial approve`.
#[derive(Serialize)]
pub struct PreparePayload<'a> {
    pub run: &'a str,
    pub file: &'a str,
    pub base: &'a str,
    pub prepared: &'a str,
    pub splices: &'a [SpliceSummary],
}

/// One `citation "tuft:@VIEW#N"` template `trial/run` replaced.
#[derive(Serialize)]
pub struct ResolvedCitation<'a> {
    pub placeholder: &'a str,
    pub binding: &'a str,
    pub path: &'a str,
    pub line: u64,
}

/// One check of the Weft report `trial/run` produced, as Weft states it
/// (local and hypothetical, never grounded).
#[derive(Serialize)]
pub struct CheckSummary<'a> {
    pub name: &'a str,
    pub result: &'a str,
}

/// `trial/run` payload: the prepared edit (as `trial/prepare` reports it), the
/// resolved citation templates and Weft checks (empty without a Weft step),
/// and `text`, exactly what `writfold trial run` prints. The run stops before
/// approval; the next step is the CLI's `writfold trial approve`.
#[derive(Serialize)]
pub struct RunPayload<'a> {
    pub run: &'a str,
    pub file: &'a str,
    pub base: &'a str,
    pub prepared: &'a str,
    pub splices: &'a [SpliceSummary],
    pub resolved: &'a [ResolvedCitation<'a>],
    pub checks: &'a [CheckSummary<'a>],
    pub text: &'a str,
}

/// `trial/evidence-list` payload: exactly the CLI's rows and trailer.
#[derive(Serialize)]
pub struct EvidenceListPayload<'a> {
    pub rows: &'a [String],
    pub trailer: &'a str,
}

/// `trial/evidence-show` payload: the full digest the request resolved to,
/// its verified source, and `text`, exactly what the CLI prints.
#[derive(Serialize)]
pub struct EvidenceShowPayload<'a> {
    pub binding: &'a str,
    pub file: &'a str,
    pub commit: &'a str,
    pub start: u64,
    pub end: u64,
    pub line: Option<u64>,
    pub text: &'a str,
}

fn envelope<T: Serialize>(
    id: RequestId<'_>,
    member: &[u8],
    body: &T,
    maximum: usize,
) -> std::result::Result<Vec<u8>, ResponseError> {
    let mut output = Bounded::new(maximum.min(MAX_RESPONSE_BYTES));
    output.append(b"{\"jsonrpc\":\"2.0\",\"id\":")?;
    match id {
        RequestId::Absent => return Err(ResponseError::AbsentId),
        RequestId::Null => output.append(b"null")?,
        RequestId::String(value) => output.json(value)?,
        RequestId::Integer(value) => output.append(value.as_decimal().as_bytes())?,
    }
    output.append(member)?;
    output.json(body)?;
    output.append(b"}\n")?;
    Ok(output.into_bytes())
}

/// Encode a complete trial result, or fail with `Limit` without partial bytes.
pub fn encode_trial_success<T: Serialize>(
    id: RequestId<'_>,
    metadata: ServerMetadata<'_>,
    generation: RequestGeneration,
    payload: &T,
    maximum: usize,
) -> std::result::Result<Vec<u8>, ResponseError> {
    let body = Result {
        exit_code: 0,
        complete: true,
        generation: generation.get(),
        session: Session::new(metadata),
        payload,
    };
    envelope(id, b",\"result\":", &body, maximum)
}

/// Encode a trial refusal (JSON-RPC -32000, Operation failed).
pub fn encode_trial_refusal(
    id: RequestId<'_>,
    metadata: ServerMetadata<'_>,
    generation: RequestGeneration,
    refusal: TrialRefusal<'_>,
    maximum: usize,
) -> std::result::Result<Vec<u8>, ResponseError> {
    let (category, code) = refusal.category.names();
    let (exit_code, outcome) = match refusal.outcome {
        TrialOutcome::Rejected => (Some(1), "rejected"),
        TrialOutcome::Unknown => (None, "unknown"),
    };
    let body = Error {
        code: -32000,
        message: "Operation failed",
        data: Data {
            schema_version: 1,
            category,
            message: refusal.message,
            diagnostic: Diagnostic {
                code,
                severity: "error",
                message: refusal.message,
                labels: [],
                help: None,
            },
            exit_code,
            complete: false,
            generation: Some(generation.get()),
            session: Session::new(metadata),
            outcome,
        },
    };
    envelope(id, b",\"error\":", &body, maximum)
}
