//! `deliver check`: evaluate the candidate's built-in checks and its
//! mode's script gates against its current bytes and record one receipt
//! per check, all in one write (the same `Op::Create` receipts as before;
//! a script gate adds no write path).
use super::candidate::{self, Candidate};
use super::checks::{self, Evaluated};
use super::config::{self, BOUND, CLAIMS, LENGTH, PARAGRAPHS, PROSE, REFERENCES, VALE};
use super::receipt::{self, Facts};
use crate::error::{KbError, Result};
use crate::index::query::Rec;
use crate::session::Session;
use crate::write::{Op, Probe, atomic, unique};
use crate::{runs, time};
use sha2::Digest;
use std::path::Path;

/// Run the checks (or the one named) of a candidate; returns its ID and
/// notes naming each receipt.
pub(super) fn run(
    session: &Session<'_>,
    rec: &Rec,
    want: &super::pick::Want<'_>,
    only: Option<&str>,
    state: &Path,
    probe: Probe,
) -> Result<(String, Vec<String>)> {
    let root = &session.ws.root;
    let cands = candidate::list(root, &runs::dir(rec), &rec.id);
    let c = super::pick::pick(session, rec, &cands, want)?;
    let m = c
        .manifest
        .as_ref()
        .map_err(|why| KbError::Refused(format!("candidate {}: {why}", c.id)))?;
    let bytes = c.bytes.as_ref().ok_or_else(|| {
        KbError::Refused(format!("candidate {} has no {}", c.id, candidate::PAYLOAD))
    })?;
    let mode = config::mode(root, &m.mode)?;
    let payload = super::render::render(bytes, &mode.rendering);
    let payload_path = write_payload(root, c, &mode.rendering, &payload, (state, probe))?;
    let text = (
        String::from_utf8_lossy(&payload).into_owned(),
        String::from_utf8_lossy(bytes).into_owned(),
    );
    let deny = config::deny(root, &m.audience)?;
    let wanted: Vec<config::Obligation> = config::obligations(&mode, &deny)
        .into_iter()
        .filter(|o| o.class != "attested")
        .filter(|o| only.is_none_or(|n| n == o.name))
        .collect();
    if wanted.is_empty() {
        return Err(KbError::Refused(match only {
            Some(n) => format!("{n} is not a check of mode {} (see deliver status)", m.mode),
            None => format!("mode {} declares no checks", m.mode),
        }));
    }
    let now = session.now;
    let content = candidate::sha(bytes);
    let mut ops = Vec::new();
    let mut notes = Vec::new();
    let filed = receipt::filed(root, &c.dir, &rec.id, &c.id);
    for ob in &wanted {
        let evaluated = evaluate(
            (session, rec),
            c,
            (&text.0, &text.1),
            ob,
            (&mode, &deny),
            &payload_path,
        )?;
        let path = format!(
            "{}/{}/{}-{}.json",
            c.dir,
            receipt::DIR,
            ob.name,
            unique(now, &ob.name)
        );
        notes.push(format!("recorded {path} ({})", evaluated.outcome.name()));
        ops.push(Op::Create {
            path,
            bytes: receipt::bytes(&Facts {
                check: &ob.name,
                class: ob.class,
                definition: &ob.digest,
                run: &rec.id,
                candidate: &c.id,
                content: &content,
                evaluated: &evaluated,
                seq: receipt::next_seq(&filed, &ob.name),
                recorded: &time::stamp(now),
            }),
        });
    }
    atomic::apply(root, state, &ops, probe)?;
    Ok((c.id.clone(), notes))
}

/// Write `payload.md` beside the candidate when the rendering is not
/// verbatim; returns the KB path of the file that holds the payload.
fn write_payload(
    root: &Path,
    c: &Candidate,
    rendering: &str,
    payload: &[u8],
    (state, probe): (&Path, Probe),
) -> Result<String> {
    if rendering == super::render::VERBATIM {
        return Ok(c.payload_path());
    }
    let path = format!("{}/{}", c.dir, super::render::FILE);
    let op = match std::fs::read(root.join(&path)) {
        Ok(old) if old == payload => return Ok(path),
        Ok(old) => Op::Replace {
            path: path.clone(),
            expected: crate::index::hex(&sha2::Sha256::digest(&old)),
            bytes: payload.to_vec(),
        },
        Err(_) => Op::Create {
            path: path.clone(),
            bytes: payload.to_vec(),
        },
    };
    atomic::apply(root, state, &[op], probe)?;
    Ok(path)
}

/// Evaluate one obligation: the payload for the paragraph bound, the deny
/// list and script gates; the candidate's own tokens for cited claims.
fn evaluate(
    (session, rec): (&Session<'_>, &Rec),
    c: &Candidate,
    (payload, source): (&str, &str),
    ob: &config::Obligation,
    (mode, deny): (&config::Mode, &config::Deny),
    payload_path: &str,
) -> Result<Evaluated> {
    if let Some(gate) = &ob.gate {
        return Ok(super::script::evaluate(
            &session.ws.root,
            c,
            gate,
            payload_path,
        ));
    }
    Ok(match ob.name.as_str() {
        PARAGRAPHS => checks::paragraphs(payload, mode),
        REFERENCES => checks::references(payload, deny),
        CLAIMS => checks::claims(session, source, &c.payload_path())?,
        LENGTH => checks::length(payload, mode),
        BOUND => super::bound::evaluate(session, rec, c, mode.agent.as_deref()),
        PROSE => match &mode.prose {
            Some(rules) => super::prose::evaluate(payload, rules),
            None => return Err(KbError::Refused(format!("{PROSE} needs a prose rule set"))),
        },
        VALE => match &mode.vale {
            Some(v) => super::vale::evaluate(&session.ws.root, payload, v),
            None => return Err(KbError::Refused(format!("{VALE} needs a Vale profile"))),
        },
        other => return Err(KbError::Refused(format!("{other} is not a built-in check"))),
    })
}
