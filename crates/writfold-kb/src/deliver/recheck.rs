//! Cited evidence that moved. A `claims.cited-fresh` receipt is bound to
//! the candidate's bytes, not to the records it cites, so on every read
//! (`deliver status`, `resume`, the delivery guard, a plan step) a passed
//! receipt stands only while every citation token in the candidate is
//! still fresh: the tokens go through `stale` again, locally, nothing
//! fetched (the call `deliver check` makes). If any is no longer fresh the
//! obligation's checked state is `stale` ([`rechecked`]), and its standing is judged again
//! with the same kernel ([`writfold_core::gate::standing`]), so a waiver
//! still stands. A candidate with no citation token costs nothing more.
//!
//! Limits: an `observed` mode gate is not re-run on read, and prose
//! without a citation token is not seen.
use super::candidate::Candidate;
use super::checks;
use super::config::CLAIMS;
use super::status::{self, Filings, Judged};
use crate::error::Result;
use crate::session::Session;
use crate::stale::{self, Verdict};
use writfold_core::delivery::rechecked;
use writfold_core::gate::{State, standing};

/// Re-check the citations behind a passed `claims.cited-fresh` receipt.
pub fn apply(session: &Session<'_>, c: &Candidate, judged: &mut [Judged<'_>]) {
    let Some(j) = judged
        .iter_mut()
        .find(|j| j.obligation.name == CLAIMS && j.obligation.gate.is_none())
        .filter(|j| j.checked == State::Satisfied)
    else {
        return;
    };
    let Some(bytes) = &c.bytes else {
        return;
    };
    let cites = checks::cites(&String::from_utf8_lossy(bytes), &c.payload_path());
    if cites.is_empty() {
        return;
    }
    // A re-check that cannot run cannot show the evidence still holds.
    let moved = match stale::check(session, &cites, false) {
        Ok(checked) => checked
            .iter()
            .filter(|c| c.verdict != Verdict::Fresh)
            .count(),
        Err(_) => cites.len(),
    };
    if moved == 0 {
        return;
    }
    j.checked = rechecked(j.checked, moved);
    j.standing = standing(false, j.checked, j.statements.statements);
    j.moved = Some((moved, cites.len()));
}

/// Every obligation of `c`, judged, with the re-check.
pub fn judged<'a>(session: &Session<'_>, c: &Candidate, f: &'a Filings) -> Result<Vec<Judged<'a>>> {
    let mut judged = status::judge(status::obligations(&session.ws.root, c)?, c, f);
    apply(session, c, &mut judged);
    Ok(judged)
}

/// What a check line adds when cited evidence moved.
pub fn note(moved: Option<(usize, usize)>) -> Option<String> {
    let (n, m) = moved?;
    Some(format!(
        "; cited evidence moved: {n} of {m} citation tokens no longer fresh"
    ))
}
