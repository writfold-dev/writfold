//! `needs` at `agent prepare` (#154): each gate a skill or profile needs,
//! judged for the run and its subject repository by the code a guard rule's
//! `gate:<id>` requirement uses ([`super::fact`]), so the two agree. The
//! decision is [`writfold_core::invoke::prepare`] over these facts.
use super::{Facts, fact, judge};
use crate::gate::subject::Subject;
use crate::guard::rules::Require;
use crate::index::query::Rec;
use crate::runs;
use crate::session::Session;
use writfold_core::guard::Fact;

/// Each needed gate's fact, with what is owed when it is not met.
pub(crate) fn judge_needs(
    session: &Session<'_>,
    rec: &Rec,
    subject: Option<Subject>,
    ids: &[String],
) -> Vec<(Fact, String)> {
    if ids.is_empty() {
        return Vec::new();
    }
    let mut facts = Facts {
        run: Ok(rec.clone()),
        state: runs::state(session, rec),
        subject: Ok(subject),
        gates: None,
        step: None,
    };
    facts.gates = Some(judge(session, &facts));
    ids.iter()
        .map(|id| {
            let owed = fact(&facts, &Require::Gate(id.clone()));
            (owed.fact, owed.text)
        })
        .collect()
}
