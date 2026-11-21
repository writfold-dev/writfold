//! `deliver attest` and `deliver waive`: a statement about one obligation
//! of a candidate, bound to its present bytes and the obligation's digest.
use super::{DeliverVerb, candidate, open_run, status};
use crate::error::{KbError, Result};
use crate::gate::statement::{self, Kind, Statement};
use crate::index::query::Rec;
use crate::runs;
use crate::session::Session;
use crate::time;
use crate::write::{Op, atomic, unique};

/// Record an attestation or a waiver about one obligation of a candidate.
pub(super) fn say(
    session: &Session<'_>,
    verb: &DeliverVerb,
    state: &std::path::Path,
    probe: crate::write::Probe,
    now: i64,
) -> Result<(Rec, String, String, Option<String>)> {
    let DeliverVerb::Say {
        kind,
        candidate,
        check,
        text,
        by,
        run,
        mode,
    } = verb
    else {
        return Err(KbError::Usage("not a statement".into()));
    };
    let flag = match kind {
        Kind::Attested => "--statement",
        Kind::Waived => "--reason",
    };
    let text = statement::check_text(flag, text)?;
    let by = statement::check_text("--by", by)?;
    let rec = open_run(session, run.as_deref(), true)?;
    let root = &session.ws.root;
    let cands = candidate::list(root, &runs::dir(&rec), &rec.id);
    let want = super::pick::Want {
        named: candidate.as_deref(),
        mode: mode.as_deref(),
        verb: Some(match kind {
            Kind::Attested => "deliver attest",
            Kind::Waived => "deliver waive",
        }),
    };
    let c = super::pick::pick(session, &rec, &cands, &want)?;
    let obligations = status::obligations(root, c)?;
    let Some(ob) = obligations.iter().find(|o| o.name == *check) else {
        let names: Vec<&str> = obligations.iter().map(|o| o.name.as_str()).collect();
        return Err(KbError::Refused(format!(
            "{check} is not an obligation of candidate {} (its mode asks for: {})",
            c.id,
            names.join(", ")
        )));
    };
    if *kind == Kind::Waived
        && let Ok(m) = &c.manifest
        && let Some(why) = super::policy::no_waiver(&m.mode, ob, &c.id)
    {
        return Err(KbError::Refused(why));
    }
    if *kind == Kind::Attested && ob.class != "attested" {
        return Err(KbError::Refused(format!(
            "{check} is a {} check: only its own passing receipt satisfies it (deliver waive sets it aside, with a reason)",
            ob.class
        )));
    }
    let ahead = ahead(session, c, &rec.id, check);
    let path = format!(
        "{}/{}/{check}-{}.json",
        c.dir,
        statement::DIR,
        unique(now, check)
    );
    let bytes = statement::bytes(&Statement {
        kind: *kind,
        obligation: check.clone(),
        run: rec.id.clone(),
        subject: c.id.clone(),
        commit: None,
        content: c.digest(),
        definition: ob.digest.clone(),
        text,
        by,
        recorded: time::stamp(now),
        seq: statement::next_seq(&statement::filed(root, &c.dir, &rec.id), check),
    });
    let ops = [Op::Create {
        path: path.clone(),
        bytes,
    }];
    atomic::apply(root, state, &ops, probe)?;
    let id = c.id.clone();
    Ok((rec, id, path, ahead))
}

/// A note when a mechanical or observed obligation of the candidate is not
/// met: repairing the candidate will make this statement stale.
fn ahead(
    session: &Session<'_>,
    c: &candidate::Candidate,
    run: &str,
    check: &str,
) -> Option<String> {
    let filings = status::Filings::read(&session.ws.root, c, run);
    let judged = super::recheck::judged(session, c, &filings).ok()?;
    let j = judged
        .iter()
        .find(|j| j.obligation.name != check && j.obligation.class != "attested" && j.unmet())?;
    Some(format!(
        "{} is [{}]; repairing the candidate will make this statement stale",
        j.obligation.name,
        j.standing.name()
    ))
}
