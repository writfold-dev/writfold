//! Grounding: what each cited reference stands on now, for a checklist
//! gate's cover rows.
//!
//! | Reference | Grounded when | Standing recorded |
//! |---|---|---|
//! | a KB citation token `⟦cite …⟧` or locator `repo@commit:path[:a-b]` | `stale` says fresh | the verdict |
//! | `gate:<id>` | the gate is satisfied (attested: attested) | the gate's standing |
//! | a gate receipt (`runs/<run>/gates/….json`, or its name) | it passed, about the gate's current definition and bytes, and the gate stands | `satisfied`, `stale`, … |
//! | an attestation (`runs/<run>/statements/….json`) | it is the gate's current attestation | `attested` or `stale` |
//! | an observation (`runs/<run>/observed/….json`) | its status is `ok` and its citation is fresh | `ok`, `not-fresh:<n>`, or the status (`timeout` …) |
//!
//! Gate receipts and statements are read from the current run only; a
//! waiver is never evidence. Each decision is the kernel's
//! ([`writfold_core::ground`]); the standing words are rendered here. A
//! checklist gate resolves against the other gates only, so it cannot cite
//! another checklist: such a reference is not grounded. Anything else,
//! including receipts of the tools removed in November 2026, is
//! `unresolved`. The standing is recorded in the cover row; when it
//! differs on a later read, the cited evidence has moved and the row is
//! stale.
use super::status::{Filings, Report};
use super::subject::Subject;
use crate::error::Result;
use crate::model::Citation;
use crate::parse::cite::{locator, tokens};
use crate::session::Session;
use crate::stale::{self, Checked, Cite, Verdict};
use writfold_core::gate::{Outcome, Standing};
use writfold_core::ground::{
    ReceiptFacts, of_gate, of_observation, of_receipt, of_statement, of_token,
};
use writfold_core::grounding::Grounding;
use writfold_core::observe::Status;

/// What one citation stands on now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolved {
    pub reference: String,
    pub grounding: Grounding,
    /// A short word for what was found; a change means the evidence moved.
    pub standing: String,
    /// The KB file the evidence is in, when there is one.
    pub path: Option<String>,
    /// Grounded by a citation whose verdict was read from a check receipt
    /// (checked elsewhere). The standing word stays `fresh`, so stored
    /// cover rows and assessed receipts still compare equal.
    pub recorded: bool,
}

/// Everything grounding reads.
pub struct Context<'s, 'r> {
    pub session: &'s Session<'s>,
    pub subject: &'s Subject,
    pub plain: &'r Report<'r>,
    pub filings: &'r Filings,
    /// `runs/<run>`.
    pub run_dir: &'s str,
}

fn found(reference: &str, grounding: Grounding, standing: &str, path: Option<&str>) -> Resolved {
    Resolved {
        reference: reference.to_owned(),
        grounding,
        standing: standing.to_owned(),
        path: path.map(str::to_owned),
        recorded: false,
    }
}

/// [`found`], grounded on verdicts some of which came from a receipt.
fn fresh(reference: &str, standing: &str, path: Option<&str>, all: &[Checked]) -> Resolved {
    Resolved {
        recorded: all.iter().any(|c| c.recorded.is_some()),
        ..found(reference, Grounding::Grounded, standing, path)
    }
}

fn ungrounded(reference: &str, standing: &str, path: Option<&str>) -> Resolved {
    found(reference, Grounding::Ungrounded, standing, path)
}

/// The note an answer adds when `n` references were grounded by a check
/// receipt.
pub fn recorded_note(n: usize) -> Option<String> {
    let said = match n {
        0 => return None,
        1 => "1 reference is".to_owned(),
        n => format!("{n} references are"),
    };
    Some(format!(
        "{said} fresh by a check receipt (checked elsewhere); writfold kb stale --cached dates them"
    ))
}

/// Resolve one reference, as written in a cover row.
pub fn resolve(cx: &Context<'_, '_>, reference: &str) -> Result<Resolved> {
    let r = reference.trim();
    if r.starts_with('⟦') {
        return token(cx, reference, None);
    }
    if let Some(id) = r.strip_prefix("gate:") {
        return Ok(gate(cx, reference, id));
    }
    if let Some(id) = r.strip_prefix("delivery:") {
        return Ok(super::ground_delivery::resolve(cx, reference, id.trim()));
    }
    if let Some(path) = file(cx, r) {
        let parts: Vec<&str> = path.split('/').collect();
        return match parts.as_slice() {
            ["runs", _, "gates", _] => Ok(receipt(cx, reference, &path)),
            ["runs", _, "statements", _] => Ok(statement(cx, reference, &path)),
            ["runs", _, "observed", _] => observed(cx, reference, &path),
            _ => Ok(ungrounded(reference, "unresolved", Some(&path))),
        };
    }
    if locator(r, 1).is_some() {
        return token(cx, reference, None);
    }
    Ok(ungrounded(reference, "unresolved", None))
}

/// A KB-relative path under `runs/`, or a receipt name in this run.
fn file(cx: &Context<'_, '_>, r: &str) -> Option<String> {
    let root = &cx.session.ws.root;
    if r.starts_with("runs/") && crate::gate::places::contained(r) {
        return root.join(r).is_file().then(|| r.to_owned());
    }
    if r.contains('/') || r.is_empty() {
        return None;
    }
    // A bare name with or without its extension (`<stamp>-<hex>` or
    // `<stamp>-<hex>.json`), as `ls` prints it.
    let stem = r.strip_suffix(".json").unwrap_or(r);
    ["gates", "statements", "observed"]
        .iter()
        .map(|dir| format!("{}/{dir}/{stem}.json", cx.run_dir))
        .find(|p| root.join(p).is_file())
}

pub(super) fn verdicts(
    cx: &Context<'_, '_>,
    found: Vec<Citation>,
    path: &str,
) -> Result<Vec<Checked>> {
    let cites: Vec<Cite> = found
        .into_iter()
        .filter(|c| !c.illustrative)
        .enumerate()
        .map(|(ord, c)| Cite::from_citation(path, ord, c))
        .collect();
    stale::check(cx.session, &cites, false)
}

/// A citation token or locator, checked by `stale`.
fn token(cx: &Context<'_, '_>, reference: &str, path: Option<&str>) -> Result<Resolved> {
    let r = reference.trim();
    let read = match r.starts_with('⟦') {
        true => tokens(r),
        false => locator(r, 1).into_iter().collect(),
    };
    let found_verdicts = verdicts(cx, read, path.unwrap_or("weft"))?;
    let not_fresh = (found_verdicts.iter())
        .filter(|c| c.verdict != Verdict::Fresh)
        .count();
    Ok(
        match (
            of_token(found_verdicts.len(), not_fresh),
            found_verdicts.as_slice(),
        ) {
            (Grounding::Ungrounded, [c]) => ungrounded(reference, c.verdict.name(), path),
            (Grounding::Ungrounded, _) => ungrounded(reference, "not-one-citation", path),
            _ => fresh(reference, "fresh", path, &found_verdicts),
        },
    )
}

/// `gate:<id>`: the gate's standing in this run.
fn gate(cx: &Context<'_, '_>, reference: &str, id: &str) -> Resolved {
    let Some(j) = cx.plain.gates.iter().find(|j| j.definition.id == id) else {
        return ungrounded(reference, "no-such-gate", None);
    };
    let path = match j.standing {
        Standing::Attested => j.statements.attested.map(|(f, _)| f.path.as_str()),
        _ => j.receipt.map(|(f, _)| f.path.as_str()),
    };
    // A waiver is never evidence ([`of_gate`]).
    match of_gate(true, j.standing) {
        Grounding::Grounded => found(reference, Grounding::Grounded, "satisfied", path),
        Grounding::Attested => found(reference, Grounding::Attested, "attested", path),
        Grounding::Ungrounded => ungrounded(reference, j.standing.name(), path),
    }
}

/// A gate receipt of this run: grounded while it passed about the gate's
/// current definition and bytes and the gate is not failing now.
fn receipt(cx: &Context<'_, '_>, reference: &str, path: &str) -> Resolved {
    let Some(r) = cx
        .filings
        .receipts
        .iter()
        .find(|f| f.path == path)
        .and_then(|f| f.parsed.as_ref().ok())
    else {
        return ungrounded(reference, "not-a-valid-receipt-of-this-run", Some(path));
    };
    let Some(j) = cx.plain.gates.iter().find(|j| j.definition.id == r.gate) else {
        return ungrounded(reference, "no-such-gate", Some(path));
    };
    let present = cx.plain.snapshot.scoped(&j.definition.patterns()).digest;
    let facts = ReceiptFacts {
        same_subject: r.repo == cx.subject.name,
        same_definition: r.definition == j.definition.digest,
        same_content: r.scope == present,
        passed: r.outcome == Outcome::Pass,
        gate_met: j.met(),
    };
    // The word names the first fact that fails, in this order.
    let standing = if !facts.same_subject {
        "other-subject"
    } else if !facts.same_definition || !facts.same_content {
        "stale"
    } else if !facts.passed {
        r.outcome.name()
    } else if !facts.gate_met {
        j.standing.name()
    } else {
        "satisfied"
    };
    match of_receipt(&facts) {
        Grounding::Grounded => found(reference, Grounding::Grounded, standing, Some(path)),
        _ => ungrounded(reference, standing, Some(path)),
    }
}

/// An attestation of this run: attested while it is the gate's current one.
fn statement(cx: &Context<'_, '_>, reference: &str, path: &str) -> Resolved {
    let current = cx.plain.gates.iter().any(|j| {
        j.standing == Standing::Attested
            && j.statements.attested.is_some_and(|(f, _)| f.path == path)
    });
    match of_statement(current) {
        Grounding::Ungrounded => ungrounded(reference, "not-a-current-attestation", Some(path)),
        grounding => found(reference, grounding, "attested", Some(path)),
    }
}

/// An observation: grounded only while it is `ok` and its embedded
/// citation is fresh ([`writfold_core::ground::of_observation`]). Any other
/// status never grounds; its standing is the status.
fn observed(cx: &Context<'_, '_>, reference: &str, path: &str) -> Result<Resolved> {
    let parsed = cx
        .session
        .text(path)
        .ok_or_else(|| "unreadable".to_owned())
        .and_then(|text| crate::observe::parse(&text));
    let Ok(o) = parsed else {
        return Ok(ungrounded(reference, "not-a-valid-observation", Some(path)));
    };
    let mut all = match &o.cite {
        Some(cite) => verdicts(cx, tokens(cite), path)?,
        None => Vec::new(),
    };
    if let Some(snapshot) = &o.snapshot {
        all.extend(verdicts(cx, tokens(snapshot), path)?);
    }
    let moved = all.iter().filter(|c| c.verdict != Verdict::Fresh).count();
    Ok(
        match (of_observation(o.status, all.len(), moved), o.status, moved) {
            (Grounding::Ungrounded, Status::Ok, 0) => {
                ungrounded(reference, "no-citations", Some(path))
            }
            (Grounding::Ungrounded, Status::Ok, n) => {
                ungrounded(reference, &format!("not-fresh:{n}"), Some(path))
            }
            (Grounding::Ungrounded, Status::NoData | Status::NotFound, _) => {
                negative(cx, reference, &o, path)?
            }
            (Grounding::Ungrounded, status, _) => ungrounded(reference, status.name(), Some(path)),
            _ => fresh(reference, "ok", Some(path), &all),
        },
    )
}

/// A look that found nothing: its status while every pin holds
/// ([`writfold_core::pin::nothing_found`]), else why not ([`super::pins`]).
fn negative(
    cx: &Context<'_, '_>,
    reference: &str,
    o: &crate::observe::Observation,
    path: &str,
) -> Result<Resolved> {
    // A catalog that cannot be read leaves the look unreadable: it never
    // holds, and nothing else is refused.
    let pins = match crate::stale::kinds::read(&cx.session.ws.root) {
        Ok(kinds) => super::pins::of(cx, &kinds, o, path)?,
        Err(_) => vec![writfold_core::pin::Pin::Unreadable],
    };
    let word = match writfold_core::pin::nothing_found(o.status, &pins) {
        true => o.status.name(),
        false => super::pins::word(&pins),
    };
    Ok(ungrounded(reference, word, Some(path)))
}
