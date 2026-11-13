//! `gate cover <gate> <item> supported REF… | unsupported --reason TEXT |
//! not-applicable --reason TEXT`, with `--by WHO`: record one cover row
//! ([`super::cover`]) about one item of a checklist gate on the current
//! subject. Every reference of a `supported` row is grounded now
//! ([`crate::gate::ground`]), against the same gates a later read grounds
//! it against; one that is not is refused, so a row never starts out
//! false. The batch form ([`super::batch`]) records several rows through
//! the same plan ([`record_all`]): one snapshot, all rows or none.
use super::cover::{self, Cited};
use crate::Request;
use crate::emit::{Answer, Output};
use crate::error::{KbError, Result};
use crate::gate::ground::{self, Context};
use crate::gate::statement::check_text;
use crate::gate::status::{self, Filings};
use crate::gate::{def, subject};
use crate::index::query::Rec;
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::write::{Op, atomic, unique};
use crate::{runs, time};
use writfold_core::assess::Grounding;
use writfold_core::checklist::Outcome;

/// One cover row to record.
#[derive(Clone, Debug)]
pub struct Cover {
    pub gate: String,
    pub item: String,
    pub outcome: Outcome,
    /// The references a `supported` row cites.
    pub references: Vec<String>,
    pub reason: Option<String>,
    pub note: Option<String>,
    pub by: String,
    pub run: Option<String>,
    pub repo: Option<std::path::PathBuf>,
}

/// The arguments each outcome takes: references for `supported`, a
/// reason for the others.
fn shaped(c: &Cover) -> Result<(Option<String>, Option<String>)> {
    let reason = c
        .reason
        .as_deref()
        .map(|r| check_text("--reason", r))
        .transpose()?;
    let note = c
        .note
        .as_deref()
        .map(|n| check_text("--note", n))
        .transpose()?;
    match (c.outcome, &reason, c.references.is_empty()) {
        (Outcome::Supported, None, false) => Ok((None, note)),
        (Outcome::Supported, Some(_), _) => Err(KbError::Usage(
            "gate cover … supported cites references; --reason is for unsupported and not-applicable".into(),
        )),
        (Outcome::Supported, None, true) => Err(KbError::Usage(
            "gate cover … supported needs at least one reference (REF…)".into(),
        )),
        (_, None, _) => Err(KbError::Usage(format!(
            "gate cover … {} needs --reason TEXT",
            c.outcome.name()
        ))),
        (_, Some(_), false) => Err(KbError::Usage(format!(
            "gate cover … {} takes a reason, not references",
            c.outcome.name()
        ))),
        (_, Some(_), true) => Ok((reason, note)),
    }
}

/// Ground every reference, refusing the first that is not grounded.
fn grounded(cx: &Context<'_, '_>, references: &[String]) -> Result<Vec<Cited>> {
    let mut out = Vec::new();
    for reference in references {
        let r = ground::resolve(cx, reference)?;
        if r.grounding == Grounding::Ungrounded {
            return Err(KbError::Refused(format!(
                "reference {reference:?} is not grounded ({}): a cover row cites only evidence that stands now (a waiver is never evidence)",
                r.standing
            )));
        }
        out.push(Cited {
            reference: reference.clone(),
            grounding: r.grounding,
            standing: r.standing,
            path: r.path,
        });
    }
    Ok(out)
}

/// The checklist definition of `c.gate` on `subject`, refusing any other
/// class.
fn definition(ws: &Workspace, subject: &subject::Subject, c: &Cover) -> Result<def::Definition> {
    let d = def::find_repository(&ws.root, Some(&subject.top), &c.gate)?;
    if d.checklist.is_none() {
        return Err(KbError::Refused(format!(
            "gate {} is a {} gate: gate cover records items of checklist gates",
            c.gate, d.class
        )));
    }
    Ok(d)
}

/// Refuse an item the checklist does not declare.
fn item(d: &def::Definition, c: &Cover) -> Result<()> {
    let Some(spec) = d.checklist.as_ref() else {
        return Err(KbError::Refused(format!(
            "gate {} is not a checklist",
            c.gate
        )));
    };
    if spec.item(&c.item).is_none() {
        let ids: Vec<&str> = spec.items.iter().map(|i| i.id.as_str()).collect();
        return Err(KbError::Refused(format!(
            "gate {} has no item {}: its items are {}",
            c.gate,
            c.item,
            ids.join(", ")
        )));
    }
    Ok(())
}

/// Turns an error about the row at an index into one that names it.
pub(super) type Blame<'a> = &'a dyn Fn(usize, KbError) -> KbError;

/// Each row's bytes, in order: every row's evidence grounded against the
/// run's other gates on one subject snapshot, its `seq` one more than the
/// highest of its gate and item. `blame` names the row an error is about.
fn rows(
    session: &Session<'_>,
    (rec, subject): (&Rec, &subject::Subject),
    d: &def::Definition,
    covers: &[Cover],
    (now, blame): (i64, Blame<'_>),
) -> Result<Vec<Vec<u8>>> {
    let mut filings = Filings::for_run(session, rec);
    filings.owed.gates = None;
    let (plain, _) = status::judge_plain(session, subject, &filings)?;
    let cx = Context {
        session,
        subject,
        plain: &plain,
        filings: &filings,
        run_dir: &filings.run_dir,
    };
    let content = plain.snapshot.scoped(&d.patterns()).digest;
    let commit = plain.snapshot.commit.as_str().to_owned();
    let mut out = Vec::new();
    for (index, c) in covers.iter().enumerate() {
        let one = || -> Result<Vec<u8>> {
            item(d, c)?;
            let (reason, note) = shaped(c)?;
            let by = check_text("--by", &c.by)?;
            let evidence = grounded(&cx, &c.references)?;
            let rows = super::rows_of(&filings.covers, &c.gate, &c.item, &subject.name);
            let seq = rows.iter().map(|(_, r)| r.seq).max().unwrap_or(0) + 1;
            Ok(cover::bytes(&cover::Cover {
                gate: c.gate.clone(),
                item: c.item.clone(),
                outcome: c.outcome,
                reason,
                note,
                by,
                run: rec.id.clone(),
                repo: subject.name.clone(),
                commit: commit.clone(),
                content: content.clone(),
                definition: d.digest.clone(),
                evidence,
                seq,
                recorded: time::stamp(now),
            }))
        };
        out.push(one().map_err(|e| blame(index, e))?);
    }
    Ok(out)
}

pub fn record(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    c: &Cover,
    now: i64,
) -> Result<Output> {
    shaped(c)?;
    check_text("--by", &c.by)?;
    record_all(ws, tool, request, std::slice::from_ref(c), (now, &|_, e| e))
}

/// Record `covers`, all of one gate, run and repository, as one write
/// plan: every row is shaped and grounded on one subject snapshot before
/// any file is written, so one refused row writes nothing.
pub(super) fn record_all(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    covers: &[Cover],
    (now, blame): (i64, Blame<'_>),
) -> Result<Output> {
    let first = covers
        .first()
        .ok_or_else(|| KbError::Refused("no cover rows to record".into()))?;
    let session = Session::open(ws, Some(tool), false, now)?;
    let rec = crate::gate::target(&session, first.run.as_deref())?;
    if runs::state(&session, &rec) == "closed" {
        return Err(KbError::Refused(format!("run {} is closed", rec.id)));
    }
    let subject = crate::gate::locate(&session, &rec, first.repo.as_deref(), &request.cwd)?
        .ok_or_else(|| {
            KbError::Refused("no subject repository: run inside one, or pass --repo DIR".into())
        })?;
    let d = definition(ws, &subject, first)?;
    let bytes = rows(&session, (&rec, &subject), &d, covers, (now, blame))?;
    drop(session);
    let mut ops = Vec::new();
    let mut paths = Vec::new();
    for (c, bytes) in covers.iter().zip(bytes) {
        let path = format!(
            "{}/{}/{}.{}-{}.json",
            runs::dir(&rec),
            cover::DIR,
            c.gate,
            c.item,
            unique(now, &format!("{}.{}", c.gate, c.item))
        );
        paths.push(path.clone());
        ops.push(Op::Create { path, bytes });
    }
    let state = ws
        .repo
        .as_ref()
        .map(|r| r.git_dir.join("writfold"))
        .ok_or_else(|| KbError::Refused("gates need a KB under Git".into()))?;
    atomic::apply(&ws.root, &state, &ops, request.probe)?;
    answer(ws, tool, request, (&rec, &subject), (covers, &paths), now)
}

fn answer(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    (rec, subject): (&Rec, &subject::Subject),
    (covers, paths): (&[Cover], &[String]),
    now: i64,
) -> Result<Output> {
    let session = Session::open(ws, Some(tool), false, now)?;
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("gates need Git".into()))?;
    let gate = covers.first().map(|c| c.gate.clone()).unwrap_or_default();
    let mut filings = Filings::for_run(&session, rec);
    filings.owed.gates = None;
    let report = status::judge(&session, subject, &filings)?;
    let mut answer = Answer {
        rows: status::rows(git, subject, &report, &filings)?
            .into_iter()
            .filter(|r| r.kind == "gate" && r.id == gate)
            .collect(),
        ..Answer::default()
    };
    for (c, path) in covers.iter().zip(paths) {
        answer.notes.push(format!(
            "recorded {path} ({} {}: a judgement with checked evidence, not a check)",
            c.item,
            c.outcome.name()
        ));
    }
    answer
        .notes
        .push("written to the KB work tree; commit it to share it".into());
    let frame = crate::gate::frame(request, "gate-cover", vec![gate, rec.id.clone()]);
    session.emit(&answer, &frame)
}
