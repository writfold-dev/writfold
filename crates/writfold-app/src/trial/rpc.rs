//! `writfold.trial/1` over `serve`: `trial/show`, `trial/retain`,
//! `trial/prepare`, `trial/run`, `trial/evidence-list` and
//! `trial/evidence-show` call exactly the application functions the CLI
//! calls, with the state directory fixed at server startup.
//!
//! Approve, apply and recover are deliberately absent: the operator key decides
//! an approval, and an agent session must not approve its own edit. `trial/run`
//! stops at a prepared edit, exactly as `writfold trial run` does.
//!
//! A refusal carries the application's own message, exactly what the CLI
//! prints. Its outcome is `rejected` when the method is read-only or the
//! refusal was decided before any write ([`BeforeEffects`]); otherwise the call
//! may have written, and the outcome is `unknown`.
mod evidence;
mod flow;

use super::refused::BeforeEffects;
use super::run::StepError;
use crate::ledger::LedgerError;
use std::error::Error;
use std::path::Path;
use writfold_domain::RequestGeneration;
use writfold_runtime::git::GitError;
use writfold_wire::rpc::response::{
    Fault, ResponseError, ServerMetadata, TrialCategory, TrialOutcome, TrialRefusal,
    encode_trial_refusal, encode_trial_success,
};
use writfold_wire::rpc::trial::{
    EvidenceListParams, EvidenceShowParams, PrepareParams, RetainParams, RunParams, ShowParams,
    TrialMethod,
};
use writfold_wire::rpc::{ParamsError, Request, RequestId, response::MAX_RESPONSE_BYTES};

/// An encoded reply, or the application error the CLI would print.
type Encoded = super::Result<Result<Vec<u8>, ResponseError>>;

/// Everything needed to encode one correlated reply.
#[derive(Clone, Copy)]
pub(crate) struct Reply<'a> {
    pub id: RequestId<'a>,
    pub metadata: ServerMetadata<'a>,
    pub generation: RequestGeneration,
    pub maximum: usize,
}

impl Reply<'_> {
    fn refuse(
        self,
        category: TrialCategory,
        outcome: TrialOutcome,
        message: &str,
    ) -> Result<Vec<u8>, ResponseError> {
        let refusal = TrialRefusal {
            category,
            outcome,
            message,
        };
        encode_trial_refusal(
            self.id,
            self.metadata,
            self.generation,
            refusal,
            self.maximum,
        )
    }

    /// A complete result, or a clean refusal when it exceeds the budget.
    /// Nothing is ever truncated.
    pub(crate) fn deliver<T: serde::Serialize>(
        self,
        payload: &T,
        oversized: TrialOutcome,
        message: &str,
    ) -> Result<Vec<u8>, ResponseError> {
        match encode_trial_success(
            self.id,
            self.metadata,
            self.generation,
            payload,
            self.maximum,
        ) {
            Err(ResponseError::Limit) => self.refuse(TrialCategory::Incomplete, oversized, message),
            other => other,
        }
    }
}

/// The error a refusal is classified by: through the pre-effect marker and a
/// `trial run` step, to the cause the step's own function returned.
fn cause<'e>(error: &'e (dyn Error + 'static)) -> &'e (dyn Error + 'static) {
    if let Some(BeforeEffects(inner)) = error.downcast_ref::<BeforeEffects>() {
        return cause(&**inner);
    }
    if let Some(step) = error.downcast_ref::<StepError>() {
        return cause(&*step.cause);
    }
    error
}

/// The class of a trial error, from its type, never from its text.
fn category(error: &(dyn Error + 'static)) -> TrialCategory {
    let error = cause(error);
    if let Some(ledger) = error.downcast_ref::<LedgerError>() {
        return match ledger {
            LedgerError::Refused(_) | LedgerError::Conflict => TrialCategory::Conflict,
            _ => TrialCategory::Incomplete,
        };
    }
    if let Some(io) = error.downcast_ref::<std::io::Error>() {
        return match io.kind() {
            std::io::ErrorKind::NotFound => TrialCategory::NotFound,
            std::io::ErrorKind::PermissionDenied => TrialCategory::Permission,
            _ => TrialCategory::Incomplete,
        };
    }
    if error.downcast_ref::<GitError>().is_some() {
        return TrialCategory::Incomplete;
    }
    TrialCategory::Malformed
}

/// Map an application error to its typed refusal, with the CLI's message.
///
/// Only a marker on the whole call proves nothing was written: a `trial/run`
/// step refused before its own writes may follow a step that wrote, so a
/// refused run is `unknown`.
pub(crate) fn refusal(
    method: TrialMethod,
    error: &(dyn Error + Send + Sync + 'static),
) -> (TrialCategory, TrialOutcome, String) {
    let before = error.downcast_ref::<BeforeEffects>().is_some();
    let outcome = if before || !method.writes() {
        TrialOutcome::Rejected
    } else {
        TrialOutcome::Unknown
    };
    (category(error), outcome, error.to_string())
}

fn budget_message(method: TrialMethod) -> String {
    format!(
        "{} result exceeds the {MAX_RESPONSE_BYTES}-byte response budget; nothing was truncated",
        method.as_str()
    )
}

/// Run one admitted trial request. `Err(Fault)` is a parameter refusal that the
/// caller encodes as a control fault; every application outcome is encoded here.
pub(crate) fn call(
    state: &Path,
    method: TrialMethod,
    request: Request<'_>,
    reply: Reply<'_>,
) -> Result<Result<Vec<u8>, ResponseError>, Fault> {
    let params = request.params();
    let invalid = |_: ParamsError| Fault::InvalidParams;
    let ran = match method {
        TrialMethod::Show => flow::show(state, ShowParams::parse(params).map_err(invalid)?, reply),
        TrialMethod::Retain => {
            flow::retain(state, RetainParams::parse(params).map_err(invalid)?, reply)
        }
        TrialMethod::Prepare => {
            flow::prepare(state, PrepareParams::parse(params).map_err(invalid)?, reply)
        }
        TrialMethod::Run => flow::run(state, RunParams::parse(params).map_err(invalid)?, reply),
        TrialMethod::EvidenceList => {
            let parsed = EvidenceListParams::parse(params).map_err(invalid)?;
            evidence::list(state, parsed, reply)
        }
        TrialMethod::EvidenceShow => {
            let parsed = EvidenceShowParams::parse(params).map_err(invalid)?;
            evidence::show(state, parsed, reply)
        }
    };
    Ok(match ran {
        Ok(encoded) => encoded,
        Err(error) => {
            let (category, outcome, message) = refusal(method, &*error);
            reply.refuse(category, outcome, &message)
        }
    })
}
