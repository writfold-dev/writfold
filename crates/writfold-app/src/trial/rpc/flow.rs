//! The edit-flow methods: `trial/show`, `trial/retain`, `trial/prepare` and
//! `trial/run`, each one call of the CLI's own function.
use super::super::BeforeEffects;
use super::super::run::weft::Pinned;
use super::super::run::{self, Ran};
use super::super::{
    Prepared, prepare as prepare_edit, retain::retain as retain_bytes, show as show_run,
};
use super::{Encoded, Reply, budget_message};
use std::path::Path;
use writfold_wire::rpc::response::{
    CheckSummary, PreparePayload, ResolvedCitation, RetainPayload, RunPayload, ShowPayload,
    SpliceSummary, TrialOutcome,
};
use writfold_wire::rpc::trial::{PrepareParams, RetainParams, RunParams, ShowParams, TrialMethod};

pub(super) fn show(state: &Path, params: ShowParams<'_>, reply: Reply<'_>) -> Encoded {
    let text = show_run::show(state, params.run)?;
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

pub(super) fn retain(state: &Path, params: RetainParams<'_>, reply: Reply<'_>) -> Encoded {
    let range = (params.start, params.end);
    let r = retain_bytes(
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

fn splices(prepared: &Prepared) -> Vec<SpliceSummary> {
    prepared
        .edits
        .iter()
        .map(|&(start, end, inserted)| SpliceSummary {
            start,
            end,
            inserted: inserted as u64,
        })
        .collect()
}

/// The refusal for a result over budget names the run, which exists.
fn prepared_message(method: TrialMethod, run: &str) -> String {
    format!(
        "prepared run {run}, but {}; inspect it with trial/show",
        budget_message(method)
    )
}

pub(super) fn prepare(state: &Path, params: PrepareParams<'_>, reply: Reply<'_>) -> Encoded {
    let report = params.report.map(Path::new);
    let p = prepare_edit(
        state,
        Path::new(params.repo),
        params.file,
        Path::new(params.recipe),
        report,
    )?;
    let splices = splices(&p);
    let payload = PreparePayload {
        run: &p.run,
        file: &p.path,
        base: &p.base,
        prepared: &p.prepared,
        splices: &splices,
    };
    let message = prepared_message(TrialMethod::Prepare, &p.run);
    Ok(reply.deliver(&payload, TrialOutcome::Unknown, &message))
}

/// A run asked for a Weft check, but the server was started without
/// `--weft PATH`. Decided before any step runs.
#[derive(Debug)]
pub(crate) struct WeftNotConfigured;

impl std::fmt::Display for WeftNotConfigured {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "trial/run with weft needs a Weft binary fixed at startup: writfold serve --weft PATH",
        )
    }
}

impl std::error::Error for WeftNotConfigured {}

/// `trial/run`: exactly [`run::run`] and [`run::render`], as the CLI's
/// `writfold trial run`. It stops at a prepared edit; nothing approves. The
/// Weft binary is the one pinned at startup, which must still have its digest.
pub(super) fn run(
    (state, pinned): (&Path, Option<&Pinned>),
    params: RunParams<'_>,
    reply: Reply<'_>,
) -> Encoded {
    let weft = match (params.weft, pinned) {
        (None, _) => None,
        (Some(review), Some(pin)) => Some((Path::new(review), pin)),
        (Some(_), None) => return Err(Box::new(BeforeEffects(Box::new(WeftNotConfigured)))),
    };
    let ran = run::run(&run::Request {
        state,
        repo: Path::new(params.repo),
        file: params.file,
        recipe: Path::new(params.recipe),
        tuft: params.tuft.map(Path::new),
        weft: weft.map(|(review, pin)| (review, pin.bin.as_path())),
        weft_sha256: weft.map(|(_, pin)| pin.sha256.as_str()),
    })?;
    let text = run::render(&ran);
    deliver_run(&ran, &text, reply)
}

fn deliver_run(ran: &Ran, text: &str, reply: Reply<'_>) -> Encoded {
    let p = &ran.prepared;
    let splices = splices(p);
    let weft = ran.weft.as_ref();
    let resolved: Vec<ResolvedCitation> = weft
        .map(|w| w.resolved.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|r| ResolvedCitation {
            placeholder: &r.placeholder,
            binding: &r.binding,
            path: &r.path,
            line: r.line,
        })
        .collect();
    let checks: Vec<CheckSummary> = weft
        .map(|w| w.checks.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|c| CheckSummary {
            name: &c.name,
            result: c.result.as_str(),
        })
        .collect();
    let payload = RunPayload {
        run: &p.run,
        file: &p.path,
        base: &p.base,
        prepared: &p.prepared,
        splices: &splices,
        resolved: &resolved,
        checks: &checks,
        text,
    };
    let message = prepared_message(TrialMethod::Run, &p.run);
    Ok(reply.deliver(&payload, TrialOutcome::Unknown, &message))
}
