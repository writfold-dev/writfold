//! `deliver check`: evaluate the candidate's built-in checks against its
//! current bytes and record one receipt per check, all in one write.
use super::candidate::{self, Candidate};
use super::checks::{self, Evaluated};
use super::config::{self, CLAIMS, PARAGRAPHS, REFERENCES};
use super::receipt::{self, Facts};
use crate::error::{KbError, Result};
use crate::index::query::Rec;
use crate::session::Session;
use crate::write::{Op, Probe, atomic, unique};
use crate::{runs, time};
use std::path::Path;

/// Run the checks (or the one named) of a candidate; returns its ID and
/// notes naming each receipt.
pub(super) fn run(
    session: &Session<'_>,
    rec: &Rec,
    named: Option<&str>,
    only: Option<&str>,
    state: &Path,
    probe: Probe,
) -> Result<(String, Vec<String>)> {
    let root = &session.ws.root;
    let cands = candidate::list(root, &runs::dir(rec), &rec.id);
    let c = candidate::pick(&cands, named, &rec.id)?;
    let m = c
        .manifest
        .as_ref()
        .map_err(|why| KbError::Refused(format!("candidate {}: {why}", c.id)))?;
    let bytes = c.bytes.as_ref().ok_or_else(|| {
        KbError::Refused(format!("candidate {} has no {}", c.id, candidate::PAYLOAD))
    })?;
    let text = String::from_utf8_lossy(bytes);
    let mode = config::mode(root, &m.mode)?;
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
    for ob in &wanted {
        let evaluated = evaluate(session, c, &text, &ob.name, (&mode, &deny))?;
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
                recorded: &time::stamp(now),
            }),
        });
    }
    atomic::apply(root, state, &ops, probe)?;
    Ok((c.id.clone(), notes))
}

fn evaluate(
    session: &Session<'_>,
    c: &Candidate,
    text: &str,
    check: &str,
    (mode, deny): (&config::Mode, &config::Deny),
) -> Result<Evaluated> {
    Ok(match check {
        PARAGRAPHS => checks::paragraphs(text, mode),
        REFERENCES => checks::references(text, deny),
        CLAIMS => checks::claims(session, text, &c.payload_path())?,
        other => return Err(KbError::Refused(format!("{other} is not a built-in check"))),
    })
}
