//! `propose ticket|decision|link …` and `proposals list|show|accept|drop`.
use super::{CAP, Pending, Proposal, Saved, Store, Stores, codec, inbox};
use crate::emit::{Answer, Frame, More, Output};
use crate::error::{KbError, Result};
use crate::parse::cite::{locator, tokens};
use crate::session::Session;
use crate::stale::{self, Cite, Verdict};
use crate::workspace::{GitTool, Workspace};
use crate::write::{self, Write};
use crate::{Request, Verb, runs};

/// `propose`: a `record` call to queue.
#[derive(Clone, Debug)]
pub struct Propose {
    pub write: Write,
    /// `--session`, else `$WRITFOLD_SESSION` (read by the CLI).
    pub session: Option<String>,
    /// `--run`, else the current-run pointer.
    pub run: Option<String>,
    /// `--tracked` or `--local`; `None`: tracked when the marker
    /// `.writfold/proposals/.tracked` exists, else local.
    pub store: Option<Store>,
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
    let stores = Stores::of(ws).ok_or_else(|| {
        KbError::Refused(
            "proposals need a KB under Git (the queue lives in its Git directory)".into(),
        )
    })?;
    let tool = GitTool::locate()?;
    if let ProposalVerb::Accept { id } = verb {
        return accept(ws, &tool, request, &stores, id, now);
    }
    let session = Session::open(ws, Some(&tool), false, now)?;
    let mut answer = Answer::default();
    let (name, args) = match verb {
        ProposalVerb::Propose(p) => {
            answer = propose(&session, &stores, p, request.probe, now)?;
            ("propose", Vec::new())
        }
        ProposalVerb::List => {
            let all = stores.pending();
            answer.rows = all.iter().map(inbox::row).collect();
            if all.is_empty() {
                answer.notes.push("no pending proposals".into());
            }
            answer.counts.push(("proposals", all.len()));
            ("proposals-list", Vec::new())
        }
        ProposalVerb::Show { id } => {
            let found = existing(&stores, id)?;
            let mut row = inbox::row(&found);
            row.more.push(More {
                label: "call",
                text: codec::command(&found.proposal.kind, &found.proposal.args),
                ..More::default()
            });
            answer.rows.push(row);
            ("proposals-show", vec![id.clone()])
        }
        ProposalVerb::Drop { id } => {
            let found = existing(&stores, id)?;
            answer.rows.push(inbox::row(&found));
            stores.remove(&found)?;
            answer.notes.push(format!(
                "proposal {id} dropped; nothing was written to the KB ({} removed)",
                copies(&found)
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

/// What removing `found` removes: its one copy, or all of them.
fn copies(found: &Pending) -> String {
    match found.copies.as_slice() {
        [one] => format!("its {} copy", one.store.label()),
        all => format!("all {} copies", all.len()),
    }
}

fn existing(stores: &Stores, id: &str) -> Result<Pending> {
    stores.find(id)?.ok_or_else(|| {
        KbError::Refused(format!(
            "no pending proposal {id} (see `writfold kb proposals list`)"
        ))
    })
}

/// Validate the call as `record` would (planning only: nothing is
/// written), then queue it unless the chosen store already holds the same
/// call, or the session is at [`CAP`] (counted over both stores).
fn propose(
    session: &Session<'_>,
    stores: &Stores,
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
    let store = p.store.unwrap_or(if stores.tracked_by_default() {
        Store::Tracked
    } else {
        Store::Local
    });
    let mut answer = Answer::default();
    if stores.holds(store, &proposal.id) {
        if let Some(found) = stores.find(&proposal.id)? {
            answer.rows.push(inbox::row(&found));
        }
        answer
            .notes
            .push("the same call is already proposed; nothing was written".into());
        return Ok(answer);
    }
    let held = super::of_session(stores, p.session.as_deref());
    if held >= CAP {
        let who = p.session.as_deref().unwrap_or("(no session)");
        return Err(KbError::Refused(format!(
            "session {who} already holds {held} pending proposals, the cap is {CAP}: accept or drop one first (`writfold kb proposals list`)"
        )));
    }
    let (saved, file) = stores.save(store, &proposal, probe)?;
    if let Some(found) = stores.find(&proposal.id)? {
        answer.rows.push(inbox::row(&found));
    }
    let id = &proposal.id;
    answer.notes.push(match (saved, store) {
        (Saved::Existing, _) => "the same call is already proposed; nothing was written".into(),
        (Saved::New, Store::Local) => format!(
            "queued locally (untracked; lost with this clone). Accept with `writfold kb proposals accept {id}` when the user asks"
        ),
        (Saved::New, Store::Tracked) => format!(
            "queued in the work tree at {} (tracked: commit it with the KB; it is not a record). Accept with `writfold kb proposals accept {id}` when the user asks",
            file.display()
        ),
    });
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
/// any refusal of the write keeps the proposal; every copy (local and
/// tracked) is removed only after the record is published. A removed
/// tracked copy is left for the user to commit, like the record.
fn accept(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    stores: &Stores,
    id: &str,
    now: i64,
) -> Result<Output> {
    let found = existing(stores, id)?;
    let proposal = &found.proposal;
    let write = codec::decode(&proposal.kind, &proposal.args)?;
    let stale = {
        let session = Session::open(ws, Some(tool), false, now)?;
        not_fresh(&session, proposal)?
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
    stores.remove(&found)?;
    if !request.json {
        let note = format!(
            "# note: proposal {id} accepted and removed from the queue ({})\n",
            copies(&found)
        );
        let at = output
            .text
            .trim_end_matches('\n')
            .rfind('\n')
            .map_or(0, |i| i + 1);
        output.text.insert_str(at, &note);
    }
    Ok(output)
}
