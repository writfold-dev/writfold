use super::{
    Fault, MAX_RESPONSE_BYTES, METHOD_SCHEMAS, PROFILE, ResponseError, ServerMetadata, Success,
    TRIAL_METHOD_SCHEMAS,
    bounded::{Bounded, Failure},
    fault,
};
use crate::rpc::{MAX_REQUEST_BYTES, PROTOCOL_VERSION, RequestId};
use serde::Serialize;
use writfold_domain::RequestGeneration;

#[derive(Serialize)]
pub(super) struct Session<'a> {
    instance_id: &'a str,
    incarnation: &'a str,
}

impl<'a> Session<'a> {
    pub(super) fn new(metadata: ServerMetadata<'a>) -> Self {
        Self {
            instance_id: metadata.instance.as_str(),
            incarnation: metadata.incarnation.as_str(),
        }
    }
}

#[derive(Serialize)]
struct Server<'a> {
    name: &'static str,
    version: &'a str,
}

#[derive(Serialize)]
struct Contract<'a> {
    namespace: &'static str,
    schema: u64,
    sha256: &'a str,
}

#[derive(Serialize)]
struct Shutdown<'a> {
    exit_code: u8,
    complete: bool,
    generation: u64,
    session: Session<'a>,
    payload: ShutdownPayload,
}

#[derive(Serialize)]
struct ShutdownPayload {
    shutdown: bool,
}

/// Encode a success without changing lifecycle state or allocating ID copies.
pub fn encode_success(
    id: RequestId<'_>,
    metadata: ServerMetadata<'_>,
    generation: RequestGeneration,
    success: Success,
    maximum: usize,
) -> Result<Vec<u8>, ResponseError> {
    let mut output = begin(id, maximum)?;
    output.append(b",\"result\":")?;
    match success {
        Success::Initialize => initialize(&mut output, metadata, generation, &CONTROL)?,
        Success::InitializeTrial => initialize(&mut output, metadata, generation, &TRIAL)?,
        Success::Shutdown => output.json(&Shutdown {
            exit_code: 0,
            complete: true,
            generation: generation.get(),
            session: Session::new(metadata),
            payload: ShutdownPayload { shutdown: true },
        })?,
    }
    finish(output)
}

/// Encode a static structured refusal; failure never substitutes a null ID.
pub fn encode_error(
    id: RequestId<'_>,
    metadata: ServerMetadata<'_>,
    generation: Option<RequestGeneration>,
    fault: Fault,
    maximum: usize,
) -> Result<Vec<u8>, ResponseError> {
    let mut output = begin(id, maximum)?;
    output.append(b",\"error\":")?;
    fault::encode(&mut output, metadata, generation, fault)?;
    finish(output)
}

fn begin(id: RequestId<'_>, maximum: usize) -> Result<Bounded, ResponseError> {
    if matches!(id, RequestId::Absent) {
        return Err(ResponseError::AbsentId);
    }
    let mut output = Bounded::new(maximum.min(MAX_RESPONSE_BYTES));
    output.append(b"{\"jsonrpc\":\"2.0\",\"id\":")?;
    match id {
        RequestId::Absent => return Err(ResponseError::AbsentId),
        RequestId::Null => output.append(b"null")?,
        RequestId::String(value) => output.json(value)?,
        // The request boundary has already validated this integer token. Write
        // its borrowed decimal bytes directly, without parse/narrow/copy steps.
        RequestId::Integer(value) => output.append(value.as_decimal().as_bytes())?,
    }
    Ok(output)
}

fn finish(mut output: Bounded) -> Result<Vec<u8>, ResponseError> {
    output.append(b"}\n")?;
    Ok(output.into_bytes())
}

/// What one initialization advertises; fixed per negotiated capability set.
struct Advertisement {
    scope: &'static str,
    capabilities: &'static [&'static str],
    methods: &'static str,
    trust_boundary: &'static str,
}

const CONTROL: Advertisement = Advertisement {
    scope: "transport-control-only",
    capabilities: &["notifications"],
    methods: METHOD_SCHEMAS,
    trust_boundary: "Transport/session control only; no authorized corpus, retained release, grounding or effect service is available.",
};

const TRIAL: Advertisement = Advertisement {
    scope: "transport-control+trial-worktree",
    capabilities: &["notifications", "writfold.trial/1"],
    methods: TRIAL_METHOD_SCHEMAS,
    trust_boundary: "Transport/session control plus writfold-trial-worktree/1 show, retain and prepare over the server's configured trial state; approve, apply and recover are CLI-only. No authorized corpus, retained release or grounding service is available.",
};

fn initialize(
    output: &mut Bounded,
    metadata: ServerMetadata<'_>,
    generation: RequestGeneration,
    advertised: &Advertisement,
) -> Result<(), Failure> {
    // Only validated integer IDs and the compiled schema are emitted as raw
    // JSON. Every dynamic string goes through serde's escaping writer.
    output.append(b"{\"protocol_version\":")?;
    output.json(PROTOCOL_VERSION)?;
    output.append(b",\"profile\":")?;
    output.json(PROFILE)?;
    output.append(b",\"implementation_scope\":")?;
    output.json(advertised.scope)?;
    output.append(b",\"server\":")?;
    output.json(&Server {
        name: "writfold",
        version: metadata.version,
    })?;
    output.append(b",\"session\":")?;
    output.json(&Session::new(metadata))?;
    output.append(b",\"generation\":")?;
    output.json(&generation.get())?;
    output.append(b",\"capabilities\":")?;
    output.json(advertised.capabilities)?;
    output.append(b",\"batch\":false,\"notifications\":true,\"max_request_bytes\":")?;
    output.json(&MAX_REQUEST_BYTES)?;
    output.append(b",\"max_response_bytes\":")?;
    output.json(&MAX_RESPONSE_BYTES)?;
    output.append(b",\"methods\":")?;
    output.append(advertised.methods.as_bytes())?;
    output.append(b",\"contract\":")?;
    output.json(&Contract {
        namespace: "writfold-agent-contract",
        schema: 1,
        sha256: metadata.contract_digest.as_str(),
    })?;
    output.append(b",\"trust_boundary\":")?;
    output.json(advertised.trust_boundary)?;
    output.append(b"}")
}
