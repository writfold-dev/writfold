//! `gate run <id>`: observe the subject, run the command, observe again,
//! and record one receipt in the run through the write protocol. The
//! retained output tails and the command's `{artifacts}` directory stay
//! local, under `<KB git dir>/writfold/gates/<run>/<receipt>/`: logs can be
//! large and can hold secrets, so only their digests are tracked.
use super::receipt::{self, Facts};
use super::{def, exec, status, subject};
use crate::Request;
use crate::emit::{Answer, Output};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::write::{Op, atomic};
use crate::{runs, time};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use writfold_core::gate::{Outcome, outcome};

/// The local directory of one receipt's tails and artifacts.
pub fn local_dir(ws: &Workspace, run: &str, stem: &str) -> Option<PathBuf> {
    let repo = ws.repo.as_ref()?;
    Some(
        repo.git_dir
            .join("writfold")
            .join("gates")
            .join(run)
            .join(stem),
    )
}

pub(super) fn name(id: &str, now: i64) -> String {
    let stamp = time::stamp(now);
    let compact: String = stamp.chars().filter(|c| *c != '-' && *c != ':').collect();
    let mut salt = Sha256::new();
    salt.update(format!("{stamp}\0{}\0{id}", std::process::id()).as_bytes());
    salt.update(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
            .to_le_bytes(),
    );
    format!(
        "{id}-{compact}-{}",
        &crate::index::hex(&salt.finalize())[..6]
    )
}

/// One more than the highest `seq` among the run's valid receipts of
/// `gate` on `repo`: the judging order, with no clock in it.
fn next_seq(root: &Path, run_dir: &str, run: &str, repo: &str, gate: &str) -> u64 {
    let filings = status::Filings::read(root, run_dir, run);
    filings
        .valid(repo)
        .iter()
        .filter(|(_, r)| r.gate == gate)
        .map(|(_, r)| r.seq)
        .max()
        .unwrap_or(0)
        + 1
}

/// What a `gate run` is about once its prelude is done.
struct Target<'a> {
    rec: &'a crate::index::query::Rec,
    subject: &'a subject::Subject,
    definition: &'a def::Definition,
    seq: u64,
}

pub(super) fn run(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    id: &str,
    named: Option<&str>,
    repo: Option<&Path>,
    now: i64,
) -> Result<Output> {
    let session = Session::open(ws, Some(tool), false, now)?;
    let rec = super::target(&session, named)?;
    if runs::state(&session, &rec) == "closed" {
        return Err(KbError::Refused(format!(
            "run {} is closed; reopen it with status {} --set open",
            rec.id, rec.id
        )));
    }
    let subject = super::locate(&session, &rec, repo, &request.cwd)?.ok_or_else(|| {
        KbError::Refused("no subject repository: run inside one, or pass --repo DIR".into())
    })?;
    let definition = def::find_repository(&ws.root, Some(&subject.top), id)?;
    if definition.attested() {
        return Err(KbError::Refused(format!(
            "gate {id} is attested: it runs nothing; record a statement with gate attest {id} --statement TEXT --by WHO"
        )));
    }
    if definition.is_checklist() {
        return Err(KbError::Refused(format!(
            "gate {id} is a checklist: it runs nothing; record each item with gate cover {id} <item> supported REF…|not-applicable --reason TEXT --by WHO"
        )));
    }
    if definition.assessed.is_some() {
        drop(session);
        return super::assessed::run(ws, tool, request, &rec, &subject, &definition, now);
    }
    let seq = next_seq(&ws.root, &runs::dir(&rec), &rec.id, &subject.name, id);
    let target = Target {
        rec: &rec,
        subject: &subject,
        definition: &definition,
        seq,
    };
    if definition.run_record() {
        let (records, window) = run_records(&session, &rec)?;
        drop(session);
        return judged_records(ws, tool, request, &target, (&records, window), now);
    }
    drop(session);
    command(ws, tool, request, &target, now)
}

/// The run's records and the window its run-record gates are judged over.
fn run_records(
    session: &Session<'_>,
    rec: &crate::index::query::Rec,
) -> Result<(crate::plan::records::Records, Window)> {
    let records = crate::plan::records::observe(session, rec)?;
    let graph = crate::plan::fold::read(&session.ws.root, &runs::dir(rec), &rec.id);
    let window = crate::plan::window(&graph, &records);
    Ok((records, window))
}

type Window = (
    Option<crate::plan::event::Baseline>,
    crate::plan::event::Baseline,
);

/// Write one receipt through the write protocol.
fn publish(ws: &Workspace, request: &Request, path: &str, bytes: Vec<u8>) -> Result<()> {
    let state = ws
        .repo
        .as_ref()
        .map(|r| r.git_dir.join("writfold"))
        .ok_or_else(|| KbError::Refused("gates need a KB under Git".into()))?;
    let ops = [Op::Create {
        path: path.to_owned(),
        bytes,
    }];
    atomic::apply(&ws.root, &state, &ops, request.probe)
}

/// A run-record gate: evaluate its requirements over the window and record
/// the verdict as a receipt bound to the window's run-record digest. No
/// command runs; a pass is exit 0, a miss exit 1 with its findings.
fn judged_records(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    t: &Target<'_>,
    (records, (begin, end)): (&crate::plan::records::Records, Window),
    now: i64,
) -> Result<Output> {
    use crate::plan::records;
    let verdict = records::evaluate(&t.definition.require, begin.as_ref(), &end, records);
    let digest = records::digest(begin.as_ref(), &end, records);
    let git = tool.admit()?;
    let head = subject::snapshot(&git, t.subject, &|_| false)?;
    let code = if verdict.pass { 0 } else { 1 };
    let empty = || writfold_runtime::process::Tail {
        bytes: Vec::new(),
        total: 0,
    };
    let evaluation = exec::Evaluation {
        ending: writfold_core::gate::Ending::Exited(code),
        exit_code: Some(code),
        signal: None,
        timed_out: false,
        error: None,
        duration_ms: 0,
        stdout: empty(),
        stderr: empty(),
    };
    let scoped = subject::Scoped {
        digest,
        paths: verdict.evidence,
        dirty: 0,
        dirty_digest: None,
    };
    let stem = name(&t.definition.id, now);
    let path = format!("{}/{}/{stem}.json", runs::dir(t.rec), receipt::DIR);
    let bytes = receipt::bytes(&Facts {
        definition: t.definition,
        run: &t.rec.id,
        repo: &t.subject.name,
        commit: head.commit.as_str(),
        scoped: &scoped,
        evaluation: &evaluation,
        subject_moved: false,
        recorded: &time::stamp(now),
        seq: t.seq,
    });
    publish(ws, request, &path, bytes)?;
    let mut notes: Vec<String> = verdict.findings.clone();
    notes.push(format!(
        "judged over the run's own records ({} counted); the receipt goes stale when they change",
        verdict.evidence
    ));
    let done = Done {
        id: &t.definition.id,
        path: &path,
        local: None,
        outcome: outcome(evaluation.ending, false),
        moved: false,
        evaluation: &evaluation,
        now,
    };
    answer(ws, tool, request, (t.rec, t.subject), done, notes)
}

/// A command gate: observe the subject, run the command, observe again,
/// and record the receipt.
fn command(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    t: &Target<'_>,
    now: i64,
) -> Result<Output> {
    let (rec, subject, definition) = (t.rec, t.subject, t.definition);
    let git = tool.admit()?;
    let patterns = definition.patterns();
    let wanted = |p: &str| writfold_core::scope::in_scope(&patterns, p);
    let before = subject::snapshot(&git, subject, &wanted)?;
    let scoped = before.scoped(&patterns);
    let stem = name(&definition.id, now);
    let local = local_dir(ws, &rec.id, &stem)
        .ok_or_else(|| KbError::Refused("gates need a KB under Git".into()))?;
    let artifacts = local.join("artifacts");
    std::fs::create_dir_all(&artifacts)?;
    let evaluation = exec::run(definition, &subject.top, &artifacts);
    std::fs::write(local.join("stdout.tail"), &evaluation.stdout.bytes)?;
    std::fs::write(local.join("stderr.tail"), &evaluation.stderr.bytes)?;
    let after = subject::snapshot(&git, subject, &wanted)?;
    let moved = after.commit != before.commit || after.scoped(&patterns) != scoped;
    let bytes = receipt::bytes(&Facts {
        definition,
        run: &rec.id,
        repo: &subject.name,
        commit: before.commit.as_str(),
        scoped: &scoped,
        evaluation: &evaluation,
        subject_moved: moved,
        recorded: &time::stamp(now),
        seq: t.seq,
    });
    let path = format!("{}/{}/{stem}.json", runs::dir(rec), receipt::DIR);
    publish(ws, request, &path, bytes)?;
    let done = Done {
        id: &definition.id,
        path: &path,
        local: Some(&local),
        outcome: outcome(evaluation.ending, moved),
        moved,
        evaluation: &evaluation,
        now,
    };
    answer(ws, tool, request, (rec, subject), done, Vec::new())
}

struct Done<'a> {
    id: &'a str,
    path: &'a str,
    local: Option<&'a Path>,
    outcome: Outcome,
    moved: bool,
    evaluation: &'a exec::Evaluation,
    now: i64,
}

/// The gate's row as `gate status` now derives it, with what was recorded.
fn answer(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    (rec, subject): (&crate::index::query::Rec, &subject::Subject),
    done: Done<'_>,
    notes: Vec<String>,
) -> Result<Output> {
    let session = Session::open(ws, Some(tool), false, done.now)?;
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("gates need Git".into()))?;
    // The gate just run is shown even when the run's plan does not owe it.
    let mut filed = status::Filings::for_run(&session, rec);
    filed.owed.gates = None;
    let report = status::judge(&session, subject, &filed)?;
    let mut answer = Answer::default();
    let rows = status::rows(git, subject, &report, &filed)?;
    answer.rows = rows
        .into_iter()
        .filter(|r| r.kind == "gate" && r.id == done.id)
        .collect();
    let state = report
        .gates
        .iter()
        .find(|j| j.definition.id == done.id)
        .map_or("unknown", |j| j.state.name());
    if state != "satisfied" {
        answer.counts.push((
            match state {
                "failed" => "failed",
                "stale" => "stale",
                _ => "unknown",
            },
            1,
        ));
    }
    let e = done.evaluation;
    answer.notes.push(format!(
        "recorded {} ({} in {} ms)",
        done.path,
        done.outcome.name(),
        e.duration_ms
    ));
    if let Some(why) = &e.error {
        answer.notes.push(format!("the command did not run: {why}"));
    }
    if e.timed_out {
        answer
            .notes
            .push("the command ran past its timeout and was stopped".into());
    }
    if done.moved {
        answer
            .notes
            .push("in-scope paths changed while the command ran: the outcome is unknown".into());
    }
    if let Some(local) = done.local {
        answer.notes.push(format!(
            "output tails and artifacts (local, untracked): {}",
            local.display()
        ));
    }
    answer.notes.extend(notes);
    answer
        .notes
        .push("written to the KB work tree; commit it to share it".into());
    let frame = super::frame(
        request,
        "gate-run",
        vec![done.id.to_owned(), rec.id.clone()],
    );
    session.emit(&answer, &frame)
}
