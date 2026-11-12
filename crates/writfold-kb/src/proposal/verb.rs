//! `propose ticket|decision|link …` and `proposals list|show|accept|drop`.
use super::{CAP, Proposal, Saved, codec, inbox};
use crate::emit::{Answer, Frame, More, Output};
use crate::error::{KbError, Result};
use crate::parse::cite::{locator, tokens};
use crate::session::Session;
use crate::stale::{self, Cite, Verdict};
use crate::workspace::{GitTool, Workspace};
use crate::write::{self, Write};
use crate::{Request, Verb, runs};
use std::path::Path;

/// `propose`: a `record` call to queue.
#[derive(Clone, Debug)]
pub struct Propose {
    pub write: Write,
    /// `--session`, else `$WRITFOLD_SESSION` (read by the CLI).
    pub session: Option<String>,
    /// `--run`, else the current-run pointer.
    pub run: Option<String>,
}

#[derive(Clone, Debug)]
pub enum ProposalVerb {
    Propose(Box<Propose>),
    List,
    Show { id: String },
    Accept { id: String },
    Drop { id: String },
}

/// Answer one proposal verb.
pub fn execute(ws: &Workspace, request: &Request, verb: &ProposalVerb, now: i64) -> Result<Output> {
    let state = super::state_of(ws).ok_or_else(|| {
        KbError::Refused(
            "proposals need a KB under Git (the queue lives in its Git directory)".into(),
        )
    })?;
    let tool = GitTool::locate()?;
    if let ProposalVerb::Accept { id } = verb {
        return accept(ws, &tool, request, &state, id, now);
    }
    let session = Session::open(ws, Some(&tool), false, now)?;
    let mut answer = Answer::default();
    let (name, args) = match verb {
        ProposalVerb::Propose(p) => {
            answer = propose(&session, &state, p, request.probe, now)?;
            ("propose", Vec::new())
        }
        ProposalVerb::List => {
            let all = super::all(&state);
            answer.rows = all.iter().map(|p| inbox::row(&state, p)).collect();
            if all.is_empty() {
                answer.notes.push("no pending proposals".into());
            }
            answer.counts.push(("proposals", all.len()));
            ("proposals-list", Vec::new())
        }
        ProposalVerb::Show { id } => {
            let found = existing(&state, id)?;
            let mut row = inbox::row(&state, &found);
            row.more.push(More {
                label: "call",
                text: codec::command(&found.kind, &found.args),
                ..More::default()
            });
            answer.rows.push(row);
            ("proposals-show", vec![id.clone()])
        }
        ProposalVerb::Drop { id } => {
            let found = existing(&state, id)?;
            answer.rows.push(inbox::row(&state, &found));
            super::remove(&state, id)?;
            answer.notes.push(format!(
                "proposal {id} dropped; nothing was written to the KB"
            ));
            ("proposals-drop", vec![id.clone()])
        }
        ProposalVerb::Accept { .. } => return Err(KbError::Usage("unreachable".into())),
    };
    let frame = frame(request, name, args);
    session.emit(&answer, &frame)
}

fn frame(request: &Request, verb: &str, args: Vec<String>) -> Frame {
    Frame {
        verb: verb.to_owned(),
        args,
        budget: request.budget,
        cursor: request.cursor.clone(),
        json: request.json,
    }
}

fn existing(state: &Path, id: &str) -> Result<Proposal> {
    super::load(state, id)?.ok_or_else(|| {
        KbError::Refused(format!(
            "no pending proposal {id} (see `writfold kb proposals list`)"
        ))
    })
}

/// Validate the call as `record` would (planning only: nothing is
/// written), then queue it unless the same call is queued, or the
/// session is at [`CAP`].
fn propose(
    session: &Session<'_>,
    state: &Path,
    p: &Propose,
    probe: write::Probe,
    now: i64,
) -> Result<Answer> {
    let (kind, args) = codec::encode(&p.write)?;
    match &p.write {
        Write::Ticket(t) => write::record::ticket(session, t).map(drop)?,
        Write::Decision(d) => write::record::decision(session, d).map(drop)?,
        Write::Link(l) => write::record::link(session, l).map(drop)?,
        _ => {
            return Err(KbError::Usage(
                "propose takes ticket, decision or link".into(),
            ));
        }
    }
    if let Some(id) = &p.session
        && !crate::draft::valid_session(id)
    {
        return Err(KbError::Usage(format!("{id:?} is not a session ID")));
    }
    let run = match &p.run {
        Some(named) => Some(write::run::target(session, Some(named))?.id),
        None => runs::current_of(session)
            .filter(|id| crate::index::query::record(&session.conn, id).is_ok_and(|r| r.is_some())),
    };
    let proposal = Proposal {
        id: super::digest(kind, &args),
        kind: kind.to_owned(),
        args,
        session: p.session.clone(),
        run,
        created: crate::time::stamp(now),
    };
    let mut answer = Answer::default();
    if let Some(found) = super::load(state, &proposal.id)? {
        answer.rows.push(inbox::row(state, &found));
        answer
            .notes
            .push("the same call is already proposed; nothing was written".into());
        return Ok(answer);
    }
    let held = super::of_session(state, p.session.as_deref());
    if held >= CAP {
        let who = p.session.as_deref().unwrap_or("(no session)");
        return Err(KbError::Refused(format!(
            "session {who} already holds {held} pending proposals, the cap is {CAP}: accept or drop one first (`writfold kb proposals list`)"
        )));
    }
    let saved = super::save(state, &proposal, probe)?;
    answer.rows.push(inbox::row(state, &proposal));
    if saved == Saved::Existing {
        answer
            .notes
            .push("the same call is already proposed; nothing was written".into());
    } else {
        answer.notes.push(format!(
            "queued locally (untracked; lost with this clone). Accept with `writfold kb proposals accept {}` when the user asks",
            proposal.id
        ));
    }
    Ok(answer)
}

/// Every citation token and evidence locator in the arguments that is not
/// fresh, as `<verdict> <target>`.
fn not_fresh(session: &Session<'_>, proposal: &Proposal) -> Result<Vec<String>> {
    let mut cites = Vec::new();
    for value in codec::values(&proposal.args) {
        let found: Vec<_> = tokens(&value)
            .into_iter()
            .filter(|c| !c.illustrative)
            .collect();
        let found = if found.is_empty() {
            locator(&value, 1).into_iter().collect()
        } else {
            found
        };
        for c in found {
            cites.push(Cite::from_citation("proposal", cites.len(), c));
        }
    }
    let checked = stale::check(session, &cites, false)?;
    Ok(cites
        .iter()
        .zip(checked)
        .filter(|(_, checked)| checked.verdict != Verdict::Fresh)
        .map(|(cite, checked)| {
            let why = match &checked.verdict {
                Verdict::Unverifiable(why) => format!("unverifiable ({why})"),
                other => other.name().to_owned(),
            };
            format!("{why} {}", crate::verbs::stale::target(&cite.raw))
        })
        .collect())
}

/// Replay a proposal through the ordinary write path. A stale citation or
/// any refusal of the write keeps the proposal; it is removed only after
/// the record is published.
fn accept(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    state: &Path,
    id: &str,
    now: i64,
) -> Result<Output> {
    let proposal = existing(state, id)?;
    let write = codec::decode(&proposal.kind, &proposal.args)?;
    let stale = {
        let session = Session::open(ws, Some(tool), false, now)?;
        not_fresh(&session, &proposal)?
    };
    if !stale.is_empty() {
        return Err(KbError::Refused(format!(
            "proposal {id} holds a citation that is not fresh ({}); re-cite and propose again, or drop it. The proposal is kept",
            stale.join("; ")
        )));
    }
    let replay = Request {
        verb: Verb::Write(write.clone()),
        ..request.clone()
    };
    let mut output = write::execute(ws, &replay, &write, now)?;
    super::remove(state, id)?;
    if !request.json {
        let note = format!("# note: proposal {id} accepted and removed from the queue\n");
        let at = output
            .text
            .trim_end_matches('\n')
            .rfind('\n')
            .map_or(0, |i| i + 1);
        output.text.insert_str(at, &note);
    }
    Ok(output)
}
