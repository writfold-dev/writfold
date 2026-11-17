//! Which candidate a verb means. Named: that one (its ID, or the six hex
//! digits that end it). Not named: with `--mode M`, the candidate that
//! decides for `M` (the newest live one, if no done record binds it); else
//! the run's only live, unbound candidate; else a refusal that lists each
//! live, unbound candidate with its mode and state.
use super::candidate::{self, Candidate};
use super::{status, step, withdraw};
use crate::error::{KbError, Result};
use crate::index::query::Rec;
use crate::runs;
use crate::session::Session;

/// What a verb asks for.
pub struct Want<'v> {
    pub named: Option<&'v str>,
    pub mode: Option<&'v str>,
    /// The verb, for a refusal on a withdrawn candidate; `None` when the
    /// verb applies to one (`deliver status`, `deliver payload`).
    pub verb: Option<&'v str>,
}

fn state(session: &Session<'_>, c: &Candidate, run: &str) -> &'static str {
    let filings = status::Filings::read(&session.ws.root, c, run);
    match super::recheck::judged(session, c, &filings) {
        Ok(j) if status::ready(&j) => "ready",
        _ => "blocked",
    }
}

/// The candidate `want` means among `cands`, the run's candidates.
pub fn pick<'a>(
    session: &Session<'_>,
    rec: &Rec,
    cands: &'a [Candidate],
    want: &Want<'_>,
) -> Result<&'a Candidate> {
    let root = &session.ws.root;
    let c = match want.named {
        Some(_) => candidate::pick(cands, want.named, &rec.id)?,
        None => unnamed(session, rec, cands, want.mode)?,
    };
    if let (Some(verb), Some(w)) = (want.verb, withdraw::of(root, c, &rec.id)) {
        return Err(withdraw::refuse(&w, &c.id, verb));
    }
    Ok(c)
}

fn unnamed<'a>(
    session: &Session<'_>,
    rec: &Rec,
    cands: &'a [Candidate],
    mode: Option<&str>,
) -> Result<&'a Candidate> {
    let root = &session.ws.root;
    let run_dir = runs::dir(rec);
    let withdrawn = withdraw::ids(root, cands, &rec.id);
    let bound: Vec<String> = crate::plan::delivery::bound_steps(root, &run_dir, &rec.id)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    if let Some(mode) = mode {
        super::mode::mode(root, mode)?;
        let mine = step::of_mode(cands, mode);
        return step::deciding(&mine, &bound, &withdrawn).ok_or_else(|| {
            KbError::Refused(format!(
                "run {} has no live, unbound {mode} candidate: writfold kb deliver prepare --mode {mode} --from FILE",
                rec.id
            ))
        });
    }
    let live: Vec<&Candidate> = cands
        .iter()
        .filter(|c| !withdrawn.contains(&c.id) && !bound.contains(&c.id))
        .collect();
    match live.as_slice() {
        [one] => Ok(one),
        [] if cands.is_empty() => candidate::pick(cands, None, &rec.id),
        [] => Err(KbError::Refused(format!(
            "run {} has no live, unbound candidate: writfold kb deliver prepare --mode M --from FILE",
            rec.id
        ))),
        many => {
            let listed: Vec<String> = many
                .iter()
                .map(|c| {
                    let mode = c.manifest.as_ref().map_or("invalid", |m| m.mode.as_str());
                    format!("{} ({mode}, {})", c.id, state(session, c, &rec.id))
                })
                .collect();
            Err(KbError::Refused(format!(
                "{} live, unbound candidates; name one, or give --mode: {}",
                many.len(),
                listed.join(", ")
            )))
        }
    }
}

/// The candidate `named` (its ID, or the six hex digits that end it), or
/// the run's only candidate when none is named.
pub fn named<'a>(cands: &'a [Candidate], named: Option<&str>, run: &str) -> Result<&'a Candidate> {
    let found: Vec<&Candidate> = match named {
        Some(n) => cands
            .iter()
            .filter(|c| c.id == n || c.id.ends_with(&format!("-{n}")))
            .collect(),
        None => cands.iter().collect(),
    };
    match (found.as_slice(), named) {
        ([one], _) => Ok(one),
        ([], Some(n)) => Err(KbError::Refused(format!("no candidate {n} in run {run}"))),
        ([], None) => Err(KbError::Refused(format!(
            "run {run} has no delivery candidate: writfold kb deliver prepare --mode M --from FILE"
        ))),
        (many, _) => {
            let ids: Vec<&str> = many.iter().map(|c| c.id.as_str()).collect();
            Err(KbError::Refused(format!(
                "{} candidates; name one: {}",
                many.len(),
                ids.join(", ")
            )))
        }
    }
}
