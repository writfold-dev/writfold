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

fn name(id: &str, now: i64) -> String {
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
    let run_dir = runs::dir(&rec);
    drop(session);
    let definition = def::find_repository(&ws.root, Some(&subject.top), id)?;
    if definition.attested() {
        return Err(KbError::Refused(format!(
            "gate {id} is attested: it runs nothing; record a statement with gate attest {id} --statement TEXT --by WHO"
        )));
    }
    let git = tool.admit()?;
    let patterns = definition.patterns();
    let wanted = |p: &str| writfold_core::gate::in_scope(&patterns, p);
    let before = subject::snapshot(&git, &subject, &wanted)?;
    let scoped = before.scoped(&patterns);
    let stem = name(id, now);
    let local = local_dir(ws, &rec.id, &stem)
        .ok_or_else(|| KbError::Refused("gates need a KB under Git".into()))?;
    let artifacts = local.join("artifacts");
    std::fs::create_dir_all(&artifacts)?;
    let evaluation = exec::run(&definition, &subject.top, &artifacts);
    std::fs::write(local.join("stdout.tail"), &evaluation.stdout.bytes)?;
    std::fs::write(local.join("stderr.tail"), &evaluation.stderr.bytes)?;
    let after = subject::snapshot(&git, &subject, &wanted)?;
    let moved = after.commit != before.commit || after.scoped(&patterns) != scoped;
    let bytes = receipt::bytes(&Facts {
        definition: &definition,
        run: &rec.id,
        repo: &subject.name,
        commit: before.commit.as_str(),
        scoped: &scoped,
        evaluation: &evaluation,
        subject_moved: moved,
        recorded: &time::stamp(now),
    });
    let path = format!("{run_dir}/{}/{stem}.json", receipt::DIR);
    let state = ws
        .repo
        .as_ref()
        .map(|r| r.git_dir.join("writfold"))
        .ok_or_else(|| KbError::Refused("gates need a KB under Git".into()))?;
    let ops = [Op::Create {
        path: path.clone(),
        bytes,
    }];
    atomic::apply(&ws.root, &state, &ops, request.probe)?;
    answer(
        ws,
        tool,
        request,
        &rec,
        &subject,
        Done {
            id,
            path: &path,
            local: &local,
            outcome: outcome(evaluation.ending, moved),
            moved,
            evaluation: &evaluation,
            now,
        },
    )
}

struct Done<'a> {
    id: &'a str,
    path: &'a str,
    local: &'a Path,
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
    rec: &crate::index::query::Rec,
    subject: &subject::Subject,
    done: Done<'_>,
) -> Result<Output> {
    let session = Session::open(ws, Some(tool), false, done.now)?;
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("gates need Git".into()))?;
    let filed = status::Filings::read(&ws.root, &runs::dir(rec), &rec.id);
    let report = status::judge(git, &ws.root, subject, &filed)?;
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
    answer.notes.push(format!(
        "output tails and artifacts (local, untracked): {}",
        done.local.display()
    ));
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
