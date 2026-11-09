//! `writfold.trial/1` over `serve`: `trial/show`, `trial/retain` and
//! `trial/prepare` call exactly the application functions the CLI calls, with
//! the state directory fixed at server startup.
//!
//! Approve, apply and recover are deliberately absent: the operator key decides
//! an approval, and an agent session must not approve its own edit.
//!
//! A refusal carries the application's own message, exactly what the CLI
//! prints. Its outcome is `rejected` when the method is read-only or the
//! refusal was decided before any write ([`BeforeEffects`]); otherwise the call
//! may have written, and the outcome is `unknown`.
use super::refused::BeforeEffects;
use super::{Result, prepare, retain::retain, show::show};
use crate::ledger::LedgerError;
use std::error::Error;
use std::path::Path;
use writfold_domain::RequestGeneration;
use writfold_runtime::git::GitError;
use writfold_wire::rpc::response::{
    Fault, PreparePayload, ResponseError, RetainPayload, ServerMetadata, ShowPayload,
    SpliceSummary, TrialCategory, TrialOutcome, TrialRefusal, encode_trial_refusal,
    encode_trial_success,
};
use writfold_wire::rpc::trial::{PrepareParams, RetainParams, ShowParams, TrialMethod};
use writfold_wire::rpc::{Request, RequestId, response::MAX_RESPONSE_BYTES};

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

/// The class of a trial error, from its type, never from its text.
fn category(error: &(dyn Error + 'static)) -> TrialCategory {
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
pub(crate) fn refusal(
    method: TrialMethod,
    error: &(dyn Error + Send + Sync + 'static),
) -> (TrialCategory, TrialOutcome, String) {
    let before = error.downcast_ref::<BeforeEffects>();
    let cause: &(dyn Error + 'static) = match before {
        Some(BeforeEffects(inner)) => &**inner,
        None => error,
    };
    let outcome = if before.is_some() || !method.writes() {
        TrialOutcome::Rejected
    } else {
        TrialOutcome::Unknown
    };
    (category(cause), outcome, error.to_string())
}

fn budget_message(method: TrialMethod) -> String {
    format!(
        "{} result exceeds the {MAX_RESPONSE_BYTES}-byte response budget; nothing was truncated",
        method.as_str()
    )
}

fn run_show(
    state: &Path,
    params: ShowParams<'_>,
    reply: Reply<'_>,
) -> Result<Result<Vec<u8>, ResponseError>> {
    let text = show(state, params.run)?;
    let payload = ShowPayload {
        run: params.run,
        text: &text,
    };
    Ok(reply.deliver(
        &payload,
        TrialOutcome::Rejected,
        &budget_message(TrialMethod::Show),
    ))
}

fn run_retain(
    state: &Path,
    params: RetainParams<'_>,
    reply: Reply<'_>,
) -> Result<Result<Vec<u8>, ResponseError>> {
    let range = (params.start, params.end);
    let r = retain(
        state,
        Path::new(params.repo),
        params.commit,
        params.file,
        range,
    )?;
    let payload = RetainPayload {
        binding: &r.binding,
        file: &r.path,
        commit: &r.commit,
        start: r.range.0,
        end: r.range.1,
    };
    let message = format!(
        "retained binding {}, but {}",
        r.binding,
        budget_message(TrialMethod::Retain)
    );
    Ok(reply.deliver(&payload, TrialOutcome::Unknown, &message))
}

fn run_prepare(
    state: &Path,
    params: PrepareParams<'_>,
    reply: Reply<'_>,
) -> Result<Result<Vec<u8>, ResponseError>> {
    let report = params.report.map(Path::new);
    let p = prepare(
        state,
        Path::new(params.repo),
        params.file,
        Path::new(params.recipe),
        report,
    )?;
    let splices: Vec<SpliceSummary> = p
        .edits
        .iter()
        .map(|&(start, end, inserted)| SpliceSummary {
            start,
            end,
            inserted: inserted as u64,
        })
        .collect();
    let payload = PreparePayload {
        run: &p.run,
        file: &p.path,
        base: &p.base,
        prepared: &p.prepared,
        splices: &splices,
    };
    let message = format!(
        "prepared run {}, but {}; inspect it with trial/show",
        p.run,
        budget_message(TrialMethod::Prepare)
    );
    Ok(reply.deliver(&payload, TrialOutcome::Unknown, &message))
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
    let ran = match method {
        TrialMethod::Show => run_show(
            state,
            ShowParams::parse(params).map_err(|_| Fault::InvalidParams)?,
            reply,
        ),
        TrialMethod::Retain => run_retain(
            state,
            RetainParams::parse(params).map_err(|_| Fault::InvalidParams)?,
            reply,
        ),
        TrialMethod::Prepare => run_prepare(
            state,
            PrepareParams::parse(params).map_err(|_| Fault::InvalidParams)?,
            reply,
        ),
    };
    Ok(match ran {
        Ok(encoded) => encoded,
        Err(error) => {
            let (category, outcome, message) = refusal(method, &*error);
            reply.refuse(category, outcome, &message)
        }
    })
}
