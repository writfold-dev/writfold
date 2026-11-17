//! `deliver withdraw` and `deliver posted`: statements about a candidate
//! as a whole, each one create-new file under the existing write protocol.
//! Writfold sends nothing; a post record says what someone did.
use super::{DeliverVerb, candidate, open_run, pick, post, render, status, withdraw};
use crate::error::{KbError, Result};
use crate::index::query::Rec;
use crate::runs;
use crate::session::Session;
use crate::write::{Op, Probe, atomic, unique};
use serde_json::json;
use std::path::Path;

/// Record a withdrawal or a post; returns the run, the candidate's ID,
/// the notes and the verb's name.
pub(super) fn record(
    session: &Session<'_>,
    verb: &DeliverVerb,
    state: &Path,
    probe: Probe,
) -> Result<(Rec, String, Vec<String>, &'static str)> {
    let (candidate, run, mode, name) = match verb {
        DeliverVerb::Withdraw {
            candidate,
            run,
            mode,
            ..
        } => (candidate, run, mode, "deliver withdraw"),
        DeliverVerb::Posted {
            candidate,
            run,
            mode,
            ..
        } => (candidate, run, mode, "deliver posted"),
        _ => return Err(KbError::Usage("not a record".into())),
    };
    let rec = open_run(session, run.as_deref(), true)?;
    let root = &session.ws.root;
    let run_dir = runs::dir(&rec);
    let cands = candidate::list(root, &run_dir, &rec.id);
    let want = pick::Want {
        named: candidate.as_deref(),
        mode: mode.as_deref(),
        verb: Some(name),
    };
    let c = pick::pick(session, &rec, &cands, &want)?;
    let (op, path, notes) = match verb {
        DeliverVerb::Withdraw { reason, by, .. } => {
            let (op, path) =
                withdraw::plan(root, c, (&rec.id, &run_dir), (reason, by), session.now)?;
            let note = format!(
                "recorded {path} (a statement); a withdrawn candidate decides no step and the guard never passes it"
            );
            (op, path, vec![note])
        }
        DeliverVerb::Posted { reference, by, .. } => posted(session, c, &rec.id, (reference, by))?,
        _ => return Err(KbError::Usage("not a record".into())),
    };
    let _ = path;
    atomic::apply(root, state, &[op], probe)?;
    let verb_name = match name {
        "deliver withdraw" => "deliver-withdraw",
        _ => "deliver-posted",
    };
    Ok((rec, c.id.clone(), notes, verb_name))
}

/// Plan a post record of `c`.
fn posted(
    session: &Session<'_>,
    c: &candidate::Candidate,
    run: &str,
    (reference, by): (&str, &str),
) -> Result<(Op, String, Vec<String>)> {
    let root = &session.ws.root;
    let reference = crate::write::one_line("--ref", reference)?;
    if reference.chars().count() > post::MAX_REF {
        return Err(KbError::Usage(format!(
            "--ref is longer than {} characters",
            post::MAX_REF
        )));
    }
    let by = crate::gate::statement::check_text("--by", by)?;
    let filings = status::Filings::read(root, c, run);
    let ready = super::recheck::judged(session, c, &filings).is_ok_and(|j| status::ready(&j));
    let payload = render::payload(root, c)
        .ok_or_else(|| KbError::Refused(format!("candidate {} has no readable payload", c.id)))?;
    let seq = post::all(root, c, run)
        .iter()
        .map(|p| p.seq)
        .max()
        .unwrap_or(0)
        + 1;
    let path = format!(
        "{}/{}{}.json",
        post::dir(c),
        post::PREFIX,
        unique(session.now, &c.id)
    );
    let bytes = super::record::seal(json!({
        "schema": post::SCHEMA,
        "run": run,
        "candidate": c.id,
        "content": c.digest(),
        "payload": candidate::sha(&payload),
        "ref": reference,
        "by": by,
        "ready": ready,
        "seq": seq,
        "recorded": crate::time::stamp(session.now),
    }));
    let mut notes = vec![format!(
        "recorded {path} (a statement; Writfold sent nothing and saw no post)"
    )];
    if !ready {
        notes.push(format!(
            "candidate {} was not ready when this was recorded; the record keeps !unready-when-recorded",
            c.id
        ));
    }
    Ok((
        Op::Create {
            path: path.clone(),
            bytes,
        },
        path,
        notes,
    ))
}
