//! The read-only deliver verbs: `payload` (the bytes to post, from the
//! present candidate), `match` (the guard's question as a verb: are these
//! bytes a ready candidate's payload?) and `modes` ([`super::modes`]).
//! Nothing is written to the KB and nothing is sent.
use super::guarded::{self, Call, Read};
use super::{DeliverVerb, candidate, found, open_run, pick, render};
use crate::Request;
use crate::emit::{Answer, More, Output};
use crate::error::{KbError, Result};
use crate::runs;
use crate::session::Session;
use writfold_core::delivery::Unread;

/// Answer one read-only verb.
pub(super) fn answer(
    session: &Session<'_>,
    request: &Request,
    verb: &DeliverVerb,
) -> Result<Output> {
    match verb {
        DeliverVerb::Payload {
            candidate,
            to,
            run,
            mode,
        } => {
            let rec = open_run(session, run.as_deref(), false)?;
            let root = &session.ws.root;
            let cands = candidate::list(root, &runs::dir(&rec), &rec.id);
            let want = pick::Want {
                named: candidate.as_deref(),
                mode: mode.as_deref(),
                verb: None,
            };
            let c = pick::pick(session, &rec, &cands, &want)?;
            let payload = render::payload(root, c).ok_or_else(|| {
                KbError::Refused(format!("candidate {} has no readable payload", c.id))
            })?;
            payload_out(session, request, (&rec, c), &payload, to.as_deref())
        }
        DeliverVerb::Match { from, mode, run } => {
            let rec = open_run(session, run.as_deref(), false)?;
            let bytes = read_from(&request.cwd, from)?;
            let found = found::all(session, &rec);
            let answer = matched(session, &rec, &found, &bytes, mode.as_deref());
            let frame = crate::gate::frame(request, "deliver-match", vec![rec.id.clone()]);
            session.emit(&answer, &frame)
        }
        DeliverVerb::Modes { mode } => {
            let answer = super::modes::answer(&session.ws.root, mode.as_deref())?;
            let frame = crate::gate::frame(request, "deliver-modes", Vec::new());
            session.emit(&answer, &frame)
        }
        _ => Err(KbError::Usage("not a read-only deliver verb".into())),
    }
}

/// The bytes of `--from FILE`, or standard input for `-`.
pub(super) fn read_from(cwd: &std::path::Path, from: &std::path::Path) -> Result<Vec<u8>> {
    if from.as_os_str() == "-" {
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut std::io::stdin(), &mut bytes)?;
        return Ok(bytes);
    }
    std::fs::read(cwd.join(from))
        .map_err(|e| KbError::Usage(format!("--from {}: {e}", from.display())))
}

fn payload_out(
    session: &Session<'_>,
    request: &Request,
    (rec, c): (&crate::index::query::Rec, &candidate::Candidate),
    payload: &[u8],
    to: Option<&std::path::Path>,
) -> Result<Output> {
    if to.is_none() && !request.json {
        return Ok(Output {
            text: String::from_utf8_lossy(payload).into_owned(),
            exit: 0,
            report: None,
            shown: None,
            drawn: None,
        });
    }
    let mut answer = Answer::default();
    let mut row = super::status::row(c, &[]);
    row.status = None;
    row.flags.retain(|f| f == "edited");
    row.more.clear();
    if let Some(to) = to {
        let path = request.cwd.join(to);
        std::fs::write(&path, payload)?;
        answer.notes.push(format!(
            "wrote the payload of {} to {}: post that file",
            c.id,
            to.display()
        ));
    }
    row.extra.push((
        "payload",
        serde_json::json!(String::from_utf8_lossy(payload)),
    ));
    row.extra
        .push(("payload_digest", serde_json::json!(candidate::sha(payload))));
    answer.rows.push(row);
    let frame = crate::gate::frame(request, "deliver-payload", vec![rec.id.clone()]);
    session.emit(&answer, &frame)
}

/// `deliver match`: the guard's decision on these bytes under a route for
/// `mode` (any mode when absent), except that a posted candidate matches.
fn matched(
    session: &Session<'_>,
    rec: &crate::index::query::Rec,
    found: &[found::Found],
    bytes: &[u8],
    mode: Option<&str>,
) -> Answer {
    let root = &session.ws.root;
    let call = Call {
        name: "deliver match",
        mode,
        file_flag: None,
        unread: Unread::Deny,
        spent_matches: true,
    };
    let seen = guarded::seen(found, &call, Some(bytes));
    let matched = seen.iter().filter(|s| s.same).count();
    let postable = seen
        .iter()
        .filter(|s| writfold_core::delivery::postable(s))
        .count();
    let mut answer = Answer::default();
    let reason = guarded::decide(root, &rec.id, found, &call, &Read::Bytes(bytes.to_vec()));
    // The row shown: a matching candidate with a post record about its
    // present bytes (what is being confirmed), else the newest postable
    // one, else the newest holding the bytes; never the IDs' chance order.
    let pairs: Vec<_> = found.iter().zip(&seen).collect();
    let ok = |s: &&writfold_core::delivery::Seen| writfold_core::delivery::postable(s);
    let holder = (pairs.iter().rev())
        .find(|(f, s)| ok(s) && f.posted.is_some())
        .or_else(|| pairs.iter().rev().find(|(_, s)| ok(s)))
        .or_else(|| pairs.iter().rev().find(|(_, s)| s.same));
    if let Some((f, _)) = holder {
        let filings = super::status::Filings::read(root, &f.c, &rec.id);
        let judged = super::recheck::judged(session, &f.c, &filings).unwrap_or_default();
        let mut row = super::status::row(&f.c, &judged);
        row.more
            .retain(|m| m.label != "check" && m.label != "read" && m.label != "why");
        row.more.push(More {
            label: "match",
            text: format!(
                "these bytes are this candidate's payload ({})",
                candidate::sha(bytes)
            ),
            ..More::default()
        });
        let posts = super::post::all(root, &f.c, &rec.id);
        row.more.extend(super::post::line(&posts, &f.c.digest()));
        answer.rows.push(row);
    }
    answer.counts = vec![("matched", matched), ("postable", postable)];
    if let Some(reason) = reason {
        answer.flagged = true;
        answer.notes.push(format!("no match: {reason}"));
        answer
            .trailer_json
            .push(("reason", serde_json::json!(reason)));
    }
    answer
}
