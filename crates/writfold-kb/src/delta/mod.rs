//! `kb delta`: what differs in the KB since a baseline, a commit of the KB
//! repository (`--since REV`) or a local mark (`--mark NAME`, then
//! `--since-mark NAME`). It says what differs, not who wrote it, and never
//! reads a clock: a baseline is exact on every clone. Nothing written is
//! never reported as success (exit 2).
mod classify;
pub mod mark;

use crate::Request;
use crate::emit::{Answer, Frame, Output, Row};
use crate::error::{KbError, Result};
use crate::gate::subject::{self, Snapshot, Subject};
use crate::session::Session;
use crate::workspace::{CANCELLED, GitTool, Workspace, budgets};
use mark::Mark;
use std::collections::BTreeSet;
use writfold_runtime::git::{
    GitExecutable, KbRepository, kb_batch_check, kb_commit, kb_diff_tree, kb_is_ancestor,
};

/// One `delta` form.
#[derive(Clone, Debug)]
pub enum DeltaVerb {
    Mark(String),
    SinceMark(String),
    Since(String),
}

fn kb_subject(ws: &Workspace) -> Result<(Subject, String)> {
    let repo = ws
        .repo
        .as_ref()
        .ok_or_else(|| KbError::Refused("delta needs a KB under Git".into()))?;
    let subject = Subject {
        top: repo.work_tree.clone(),
        git_dir: repo.git_dir.clone(),
        name: "kb".into(),
    };
    Ok((subject, repo.prefix.clone()))
}

fn reads(subject: &Subject) -> KbRepository<'_> {
    KbRepository {
        git_dir: &subject.git_dir,
        work_tree: Some(&subject.top),
    }
}

/// The KB as it is now, as a mark named `name`.
pub fn take(ws: &Workspace, git: &GitExecutable<'_>, name: &str, now: i64) -> Result<Mark> {
    let (subject, prefix) = kb_subject(ws)?;
    let snap = subject::snapshot(git, &subject, &|p| p.starts_with(&prefix))?;
    Ok(Mark {
        name: name.to_owned(),
        head: snap.commit.as_str().to_owned(),
        taken: crate::time::stamp(now),
        dirty: snap.dirty().to_vec(),
    })
}

/// The session's mark at session start, kept when it exists (a resume or
/// a compaction keeps the first). Best effort: any failure writes nothing.
pub fn session_mark(ws: &Workspace, session: &str, now: i64) -> Result<()> {
    let state = ws
        .repo
        .as_ref()
        .map(|r| r.git_dir.join("writfold"))
        .ok_or_else(|| KbError::Refused("no KB Git directory".into()))?;
    if mark::exists(&state, session) || !crate::draft::valid_session(session) {
        return Ok(());
    }
    let tool = GitTool::locate()?;
    let git = tool.admit()?;
    let taken = take(ws, &git, session, now)?;
    mark::write(&state, &taken, now, crate::write::no_probe)
}

/// The changed repository paths under the KB, with `added`, `modified` or
/// `deleted`, comparing `base` with now.
fn differ(
    git: &GitExecutable<'_>,
    subject: &Subject,
    prefix: &str,
    base: &Mark,
    snap: &Snapshot,
) -> Result<Vec<(String, &'static str)>> {
    let head = snap.commit.as_str();
    let refused = || KbError::Refused(format!("commit {} is not in this clone", base.head));
    let committed = kb_diff_tree(
        git,
        reads(subject),
        &base.head,
        head,
        ".",
        budgets()?,
        &CANCELLED,
    )?
    .ok_or_else(refused)?;
    let mut paths: BTreeSet<String> = committed.into_iter().collect();
    paths.extend(snap.dirty().iter().map(|(p, _)| p.clone()));
    paths.extend(base.dirty.iter().map(|(p, _)| p.clone()));
    paths.retain(|p| p.starts_with(prefix));
    let paths: Vec<String> = paths.into_iter().collect();
    let names: Vec<String> = paths.iter().map(|p| format!("{}:{p}", base.head)).collect();
    let then = kb_batch_check(git, reads(subject), &names, budgets()?, &CANCELLED)?;
    let mut out = Vec::new();
    for (path, at_base) in paths.iter().zip(then) {
        let before = match base.dirty.iter().find(|(p, _)| p == path) {
            Some((_, id)) => id.clone(),
            None => at_base
                .filter(|(_, kind)| kind == "blob")
                .map(|(id, _)| id.as_str().to_owned()),
        };
        let after = match snap.dirty().iter().find(|(p, _)| p == path) {
            Some((_, id)) => id.clone(),
            None => snap.committed(path).map(str::to_owned),
        };
        let status = match (&before, &after) {
            (None, Some(_)) => "added",
            (Some(_), None) => "deleted",
            (Some(a), Some(b)) if a != b => "modified",
            _ => continue,
        };
        out.push((path.clone(), status));
    }
    Ok(out)
}

/// The rows, counts and notes for the paths that differ from `base`.
fn report(session: &Session<'_>, changed: &[(String, &'static str)], prefix: &str) -> Answer {
    let mut answer = Answer::default();
    let mut counts = [("records", 0), ("run", 0), ("config", 0), ("other", 0)];
    for (path, status) in changed {
        let local = path.strip_prefix(prefix).unwrap_or(path);
        let class = classify::classify(&session.conn, local);
        if let Some(c) = counts.iter_mut().find(|(s, _)| *s == class.section) {
            c.1 += 1;
        }
        answer.rows.push(Row {
            kind: "delta",
            id: class.id,
            status: Some((*status).into()),
            text: class.kind,
            path: Some(local.to_owned()),
            section: Some(class.section),
            ..Row::default()
        });
    }
    answer.counts = vec![("changed", changed.len())];
    answer.counts.extend(counts);
    answer
}

/// The base of a comparison and how the note names it.
fn baseline(
    git: &GitExecutable<'_>,
    subject: &Subject,
    state: &std::path::Path,
    verb: &DeltaVerb,
) -> Result<(Mark, String)> {
    match verb {
        DeltaVerb::Mark(name) | DeltaVerb::SinceMark(name) => {
            let m = mark::read(state, name)?;
            let said = format!(
                "mark {name} (HEAD {} and {} dirty files, taken {})",
                short(&m.head),
                m.dirty.len(),
                m.taken
            );
            Ok((m, said))
        }
        DeltaVerb::Since(rev) => {
            let commit =
                kb_commit(git, reads(subject), rev, budgets()?, &CANCELLED)?.ok_or_else(|| {
                    KbError::Refused(format!("{rev} names no commit of the KB repository"))
                })?;
            let said = format!("{rev} ({})", commit.as_str());
            let head = commit.as_str().to_owned();
            Ok((
                Mark {
                    head,
                    ..Mark::default()
                },
                said,
            ))
        }
    }
}

/// Answer one `delta` form.
pub fn execute(ws: &Workspace, request: &Request, verb: &DeltaVerb, now: i64) -> Result<Output> {
    let tool = GitTool::locate()?;
    let session = Session::open(ws, Some(&tool), false, now)?;
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("delta needs Git".into()))?;
    let (subject, prefix) = kb_subject(ws)?;
    let state = subject.git_dir.join("writfold");
    if let DeltaVerb::Mark(name) = verb {
        let taken = take(ws, git, name, now)?;
        mark::write(&state, &taken, now, request.probe)?;
        let mut answer = Answer::default();
        answer.notes.push(format!(
            "marked {name}: HEAD {} and {} dirty files; local, never committed",
            short(&taken.head),
            taken.dirty.len()
        ));
        return session.emit(&answer, &frame(request, "delta-mark", name));
    }
    let (base, said) = baseline(git, &subject, &state, verb)?;
    let base_id =
        kb_commit(git, reads(&subject), &base.head, budgets()?, &CANCELLED)?.ok_or_else(|| {
            KbError::Refused(format!(
                "the base commit {} is not in this clone",
                base.head
            ))
        })?;
    let snap = subject::snapshot(git, &subject, &|p| p.starts_with(&prefix))?;
    let changed = differ(git, &subject, &prefix, &base, &snap)?;
    let mut answer = report(&session, &changed, &prefix);
    answer.notes.push(format!("compared with {said}"));
    if base.head != snap.commit.as_str() {
        let behind = !kb_is_ancestor(
            git,
            reads(&subject),
            &base_id,
            &snap.commit,
            budgets()?,
            &CANCELLED,
        )?;
        answer.notes.push(match behind {
            true => "HEAD is not a descendant of the base: the difference includes commits on both sides".into(),
            false => "commits made since the base are included, whoever made them; a delta says what differs, not who wrote it".into(),
        });
    }
    if changed.is_empty() {
        answer
            .notes
            .push(format!("nothing in the KB differs from {said}"));
    }
    let name = match verb {
        DeltaVerb::Since(rev) => rev.clone(),
        DeltaVerb::SinceMark(n) | DeltaVerb::Mark(n) => n.clone(),
    };
    let mut output = session.emit(&answer, &frame(request, "delta", &name))?;
    output.exit = if changed.is_empty() { 2 } else { 0 };
    Ok(output)
}

fn short(id: &str) -> &str {
    id.get(..12).unwrap_or(id)
}

fn frame(request: &Request, verb: &str, arg: &str) -> Frame {
    Frame {
        verb: verb.into(),
        args: vec![arg.to_owned()],
        budget: request.budget,
        cursor: request.cursor.clone(),
        json: request.json,
    }
}
