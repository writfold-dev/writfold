use crate::{ServeError, VERSION};
use writfold_domain::{ContentDigest, RequestGeneration};
use writfold_protocol::{
    generation::{GenerationAdvance, advance_generation},
    session::{Session, SessionError, SessionIdentity},
};
use writfold_runtime::stdio::{InputFrame, InvocationIdentity};
use writfold_wire::{
    Limits, MAX_SUPPORTED_DEPTH,
    rpc::{
        Capability, Frame, InitializeParams, MAX_REQUEST_BYTES, Request, RequestId,
        response::{
            Fault, MAX_RESPONSE_BYTES, ServerMetadata, Success, encode_error, encode_success,
        },
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Flow {
    Continue,
    Stop,
}

pub(super) struct Dispatch {
    pub response: Option<Vec<u8>>,
    pub flow: Flow,
}

pub(super) struct Engine<'a> {
    session: Session<'a>,
    generation: RequestGeneration,
    metadata: ServerMetadata<'a>,
    limits: Limits,
    maximum_response: usize,
}

impl<'a> Engine<'a> {
    pub fn new(
        identity: &'a InvocationIdentity,
        contract: &'a ContentDigest,
    ) -> Result<Self, ServeError> {
        Ok(Self {
            session: Session::new(SessionIdentity::new(
                identity.instance(),
                identity.incarnation(),
            )),
            generation: RequestGeneration::first(),
            metadata: ServerMetadata::new(
                VERSION,
                identity.instance(),
                identity.incarnation(),
                contract,
            ),
            // The byte ceiling also bounds counts; no smaller hidden object-list limit.
            limits: Limits::new(
                MAX_REQUEST_BYTES,
                MAX_SUPPORTED_DEPTH,
                MAX_REQUEST_BYTES,
                MAX_REQUEST_BYTES,
            )?,
            maximum_response: MAX_RESPONSE_BYTES,
        })
    }

    pub fn handle(&mut self, input: InputFrame<'_>) -> Result<Dispatch, ServeError> {
        let frame = match input {
            InputFrame::Oversized => {
                return self.refusal(RequestId::Null, None, Fault::Oversized, Flow::Continue);
            }
            InputFrame::Bytes(bytes) => match Frame::parse(bytes, self.limits) {
                Ok(frame) => frame,
                Err(_) => return self.refusal(RequestId::Null, None, Fault::Parse, Flow::Continue),
            },
        };
        let request = match frame.request() {
            Ok(request) => request,
            Err(error) => {
                return self.refusal(
                    error.response_id(),
                    None,
                    Fault::InvalidRequest,
                    Flow::Continue,
                );
            }
        };
        let generation = match advance_generation(self.generation, &[]) {
            GenerationAdvance::Allocated(value) => value,
            GenerationAdvance::Exhausted => {
                return self.refusal(request.id(), None, Fault::GenerationExhausted, Flow::Stop);
            }
            GenerationAdvance::InUse => return Err(ServeError::GenerationInUse),
        };
        self.generation = generation;
        self.dispatch(request, generation)
    }

    fn dispatch(
        &mut self,
        request: Request<'_>,
        generation: RequestGeneration,
    ) -> Result<Dispatch, ServeError> {
        let success = match self.decide(request) {
            Ok(success) => success,
            Err(fault) => {
                return self.refusal(request.id(), Some(generation), fault, Flow::Continue);
            }
        };
        let response = if request.is_notification() {
            None
        } else {
            // No state commit precedes successful complete encoding. If a valid
            // huge ID cannot fit a bounded correlated reply, close the connection
            // rather than truncate it or fabricate a null-ID success/refusal.
            Some(encode_success(
                request.id(),
                self.metadata,
                generation,
                success,
                self.maximum_response,
            )?)
        };
        let flow = match success {
            Success::Initialize => {
                self.session.initialize().map_err(ServeError::Session)?;
                Flow::Continue
            }
            Success::Shutdown => {
                self.session.shutdown().map_err(ServeError::Session)?;
                Flow::Stop
            }
        };
        Ok(Dispatch { response, flow })
    }

    fn decide(&self, request: Request<'_>) -> Result<Success, Fault> {
        // Method lookup precedes lifecycle and parameter failures. Unavailable
        // data methods are absent, not registered handlers returning placeholders.
        match request.method() {
            "initialize" => {
                self.session.require_running().map_err(session_fault)?;
                let parameters =
                    InitializeParams::parse(request.params()).map_err(|_| Fault::InvalidParams)?;
                self.session
                    .require_uninitialized()
                    .map_err(session_fault)?;
                let admitted = parameters
                    .admit()
                    .map_err(|_| Fault::UnsupportedCapability)?;
                if admitted
                    .capabilities()
                    .any(|capability| capability != Capability::Notifications)
                {
                    return Err(Fault::UnsupportedCapability);
                }
                Ok(Success::Initialize)
            }
            "shutdown" => {
                self.session.require_initialized().map_err(session_fault)?;
                request
                    .shutdown_params()
                    .map_err(|_| Fault::InvalidParams)?;
                Ok(Success::Shutdown)
            }
            _ => Err(Fault::MethodNotFound),
        }
    }

    fn refusal(
        &self,
        id: RequestId<'_>,
        generation: Option<RequestGeneration>,
        fault: Fault,
        flow: Flow,
    ) -> Result<Dispatch, ServeError> {
        let response = if matches!(id, RequestId::Absent) {
            None
        } else {
            Some(encode_error(
                id,
                self.metadata,
                generation,
                fault,
                self.maximum_response,
            )?)
        };
        Ok(Dispatch { response, flow })
    }
}

fn session_fault(error: SessionError) -> Fault {
    match error {
        SessionError::InitializationRequired => Fault::InitializationRequired,
        SessionError::AlreadyInitialized => Fault::AlreadyInitialized,
        SessionError::ShutDown => Fault::ShutDown,
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
