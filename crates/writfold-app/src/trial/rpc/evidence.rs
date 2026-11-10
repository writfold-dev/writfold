//! The read-only evidence methods: `trial/evidence-list` and
//! `trial/evidence-show`, each one call of the CLI's own function. Every entry
//! is verified before it is listed or shown; tampered evidence refuses.
use super::super::evidence::{self, Query};
use super::{Encoded, Reply, budget_message};
use std::path::Path;
use writfold_wire::rpc::response::{EvidenceListPayload, EvidenceShowPayload, TrialOutcome};
use writfold_wire::rpc::trial::{EvidenceListParams, EvidenceShowParams, TrialMethod};

pub(super) fn list(state: &Path, params: EvidenceListParams<'_>, reply: Reply<'_>) -> Encoded {
    let limit = params.limit.map_or(evidence::DEFAULT_LIMIT, |limit| {
        usize::try_from(limit).unwrap_or(usize::MAX)
    });
    let query = Query {
        path: params.path,
        commit: params.commit,
        limit,
    };
    let (rows, trailer) = evidence::list(state, &query)?;
    let payload = EvidenceListPayload {
        rows: &rows,
        trailer: &trailer,
    };
    let message = format!(
        "{}; narrow it with path, commit or limit",
        budget_message(TrialMethod::EvidenceList)
    );
    Ok(reply.deliver(&payload, TrialOutcome::Rejected, &message))
}

pub(super) fn show(state: &Path, params: EvidenceShowParams<'_>, reply: Reply<'_>) -> Encoded {
    let shown = evidence::show(
        state,
        params.binding,
        params.context_lines.unwrap_or(0),
        params.repo.map(Path::new),
    )?;
    let entry = &shown.entry;
    let payload = EvidenceShowPayload {
        binding: &entry.binding,
        file: &entry.path,
        commit: &entry.commit,
        start: entry.range.0,
        end: entry.range.1,
        line: entry.line,
        text: &shown.text,
    };
    Ok(reply.deliver(
        &payload,
        TrialOutcome::Rejected,
        &budget_message(TrialMethod::EvidenceShow),
    ))
}
