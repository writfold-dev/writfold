use super::{Fault, ServerMetadata, bounded::Bounded, bounded::Failure, codec::Session};
use serde::Serialize;
use writfold_domain::RequestGeneration;

#[derive(Serialize)]
struct Error<'a> {
    code: i32,
    message: &'static str,
    data: Data<'a>,
}

#[derive(Serialize)]
struct Diagnostic {
    code: &'static str,
    severity: &'static str,
    message: &'static str,
    labels: [(); 0],
    help: Option<&'static str>,
}

#[derive(Serialize)]
struct Data<'a> {
    schema_version: u64,
    category: &'static str,
    message: &'static str,
    diagnostic: Diagnostic,
    exit_code: u8,
    complete: bool,
    generation: Option<u64>,
    session: Session<'a>,
    outcome: &'static str,
}

impl Fault {
    fn details(self) -> (i32, &'static str, &'static str, &'static str, &'static str) {
        match self {
            Self::Parse => (-32700, "Parse error", "malformed", "Parse error", "E102"),
            Self::Oversized => (
                -32600,
                "Request frame exceeds 65536 bytes",
                "malformed",
                "Request frame exceeds 65536 bytes",
                "E102",
            ),
            Self::InvalidRequest => (
                -32600,
                "Invalid Request",
                "malformed",
                "Invalid Request",
                "E102",
            ),
            Self::InvalidParams => (
                -32602,
                "Invalid params",
                "malformed",
                "Invalid params",
                "E102",
            ),
            Self::MethodNotFound => (
                -32601,
                "Method not found",
                "malformed",
                "Method not found",
                "E102",
            ),
            Self::InitializationRequired => (
                -32002,
                "Initialization required",
                "conflict",
                "initialize the protocol before invoking methods",
                "E104",
            ),
            Self::AlreadyInitialized => (
                -32000,
                "Operation failed",
                "conflict",
                "engine is already initialized",
                "E104",
            ),
            Self::ShutDown => (
                -32002,
                "Engine is shut down",
                "conflict",
                "engine is shut down",
                "E104",
            ),
            Self::UnsupportedCapability => (
                -32000,
                "Operation failed",
                "malformed",
                "unsupported requested capability",
                "E102",
            ),
            Self::GenerationExhausted => (
                -32000,
                "Operation failed",
                "conflict",
                "request generation exhausted",
                "E104",
            ),
        }
    }
}

pub(super) fn encode(
    output: &mut Bounded,
    metadata: ServerMetadata<'_>,
    generation: Option<RequestGeneration>,
    fault: Fault,
) -> Result<(), Failure> {
    let (code, message, category, detail, diagnostic) = fault.details();
    output.json(&Error {
        code,
        message,
        data: Data {
            schema_version: 1,
            category,
            message: detail,
            diagnostic: Diagnostic {
                code: diagnostic,
                severity: "error",
                message: detail,
                labels: [],
                help: None,
            },
            exit_code: 1,
            complete: false,
            generation: generation.map(RequestGeneration::get),
            session: Session::new(metadata),
            outcome: "rejected",
        },
    })
}
