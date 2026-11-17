//! Each candidate of a run as the guard and `deliver match` see it: its
//! readiness, what it still owes, whether it is withdrawn, its post
//! records and its present payload.
use super::{candidate, post, recheck, render, status, step, withdraw};
use crate::index::query::Rec;
use crate::runs;
use crate::session::Session;
use candidate::Candidate;

/// One candidate, found.
pub struct Found {
    pub c: Candidate,
    pub ready: bool,
    /// What it still owes (`<check> [<standing>]`, or why it cannot be met).
    pub owed: Vec<String>,
    /// The command that meets its first unmet obligation.
    pub next: Option<String>,
    pub withdrawn: Option<withdraw::Withdrawn>,
    /// The newest post record about its present bytes.
    pub posted: Option<post::Post>,
    /// Its present payload ([`render::payload`]).
    pub payload: Option<Vec<u8>>,
    /// Its mode, when its manifest reads.
    pub mode: Option<String>,
}

impl Found {
    /// The KB path of the file to post ([`Found::path_of`]).
    pub fn post_path(&self, root: &std::path::Path) -> String {
        Self::path_of(root, &self.c)
    }

    /// The KB path of the file to post: `payload.md` when the mode's
    /// rendering is not verbatim, else the candidate.
    pub fn path_of(root: &std::path::Path, c: &Candidate) -> String {
        let rendered = (c.manifest.as_ref().ok())
            .and_then(|m| super::mode::mode(root, &m.mode).ok())
            .is_some_and(|m| m.rendering != render::VERBATIM);
        match rendered {
            true => format!("{}/{}", c.dir, render::FILE),
            false => c.payload_path(),
        }
    }
}

/// Every candidate of the run, found.
pub fn all(session: &Session<'_>, rec: &Rec) -> Vec<Found> {
    let root = &session.ws.root;
    candidate::list(root, &runs::dir(rec), &rec.id)
        .into_iter()
        .map(|c| {
            let filings = status::Filings::read(root, &c, &rec.id);
            let (ready, owed, next) = match recheck::judged(session, &c, &filings) {
                Ok(judged) => {
                    let mut owed: Vec<String> = (judged.iter())
                        .filter(|j| j.unmet())
                        .map(|j| format!("{} [{}]", j.obligation.name, j.standing.name()))
                        .collect();
                    let ready = status::ready(&judged);
                    if let (false, true, Ok(m)) = (ready, owed.is_empty(), &c.manifest) {
                        owed.push(step::empty(&m.mode));
                    }
                    let next = super::next::next(root, &c, &judged);
                    (ready, owed, next)
                }
                Err(error) => (false, vec![error.to_string()], None),
            };
            let posts = post::all(root, &c, &rec.id);
            Found {
                ready,
                owed,
                next,
                withdrawn: withdraw::of(root, &c, &rec.id),
                posted: post::current(&posts, &c.digest()),
                payload: render::payload(root, &c),
                mode: c.manifest.as_ref().ok().map(|m| m.mode.clone()),
                c,
            }
        })
        .collect()
}
