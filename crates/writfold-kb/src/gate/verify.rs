//! `gate verify <receipt>`: re-run a receipt's recorded argv at its
//! recorded commit, in a temporary detached worktree of the subject, and
//! say whether the outcome agrees. The current definition is not read: the
//! receipt is checked against itself. A receipt taken on a dirty subject
//! cannot be reproduced from a commit and is reported unverifiable. Nothing
//! is written to the KB; the worktree is removed afterwards.
use super::receipt::{self, Receipt};
use super::{exec, subject};
use crate::Request;
use crate::emit::{Answer, Output, Row};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::stale::repos;
use crate::time;
use crate::workspace::{CANCELLED, GitTool, Workspace, budgets};
use std::path::{Path, PathBuf};
use writfold_core::gate::Outcome;
use writfold_runtime::git::{
    GitExecutable, KbRepository, gate_worktree_add, gate_worktree_remove, kb_commit,
};

/// The receipt file: as given (relative to the KB root, else to the
/// working directory), its run and its name.
fn file(ws: &Workspace, cwd: &Path, given: &str) -> Result<(PathBuf, String, String)> {
    let path = [ws.root.join(given), cwd.join(given)]
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| KbError::Refused(format!("no receipt file {given}")))?;
    let parts: Vec<String> = path
        .components()
        .rev()
        .take(3)
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    match parts.as_slice() {
        [name, dir, run] if dir == receipt::DIR => Ok((path.clone(), run.clone(), name.clone())),
        _ => Err(KbError::Refused(format!(
            "{given} is not runs/<run>/{}/<receipt>.json",
            receipt::DIR
        ))),
    }
}

/// The Git directory of the receipt's repository: `--repo`, else the KB's
/// repository map or search roots, else the repository here when its name
/// matches.
fn repository(
    ws: &Workspace,
    git: &GitExecutable<'_>,
    r: &Receipt,
    repo: Option<&Path>,
    cwd: &Path,
) -> Result<PathBuf> {
    if repo.is_none()
        && let Some(found) = repos::resolve_in(Some(ws), &r.repo, None)
    {
        return Ok(found);
    }
    match subject::locate(ws, git, repo, cwd, None)? {
        Some(s) if repo.is_some() || s.name == r.repo => Ok(s.git_dir),
        _ => Err(KbError::Refused(format!(
            "repository {} is not local: pass --repo DIR, or map it with the KB's repository map",
            r.repo
        ))),
    }
}

pub(super) fn verify(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    given: &str,
    repo: Option<&Path>,
    now: i64,
) -> Result<Output> {
    let (path, run, name) = file(ws, &request.cwd, given)?;
    let text = std::fs::read_to_string(&path)?;
    let r = receipt::parse(&text, &run, &name)
        .map_err(|why| KbError::Refused(format!("invalid receipt {given}: {why}")))?;
    let git = tool.admit()?;
    let git_dir = repository(ws, &git, &r, repo, &request.cwd)?;
    let stem = name.trim_end_matches(".json");
    let shown = path
        .strip_prefix(&ws.root)
        .map_or_else(|_| path.display().to_string(), |p| p.display().to_string());
    let (status, text, flag) = check(ws, &git, &git_dir, &r, &run, stem, now)?;
    let mut answer = Answer::default();
    answer.rows.push(Row {
        kind: "verify",
        id: stem.to_owned(),
        status: Some(status.into()),
        text,
        path: Some(shown),
        line: Some(1),
        flags: flag.iter().map(|f| (*f).to_owned()).collect(),
        ..Row::default()
    });
    if let Some(name) = flag {
        answer.counts.push((name, 1));
    }
    answer.notes.push(
        "re-run in a temporary worktree at the recorded commit; nothing was written to the KB"
            .into(),
    );
    let session = Session::open(ws, Some(tool), false, now)?;
    let frame = super::frame(request, "gate-verify", vec![stem.to_owned()]);
    session.emit(&answer, &frame)
}

type Checked = (&'static str, String, Option<&'static str>);

fn check(
    ws: &Workspace,
    git: &GitExecutable<'_>,
    git_dir: &Path,
    r: &Receipt,
    run: &str,
    stem: &str,
    now: i64,
) -> Result<Checked> {
    let unverifiable = |why: String| Ok(("unverifiable", why, Some("unverifiable")));
    if r.dirty > 0 {
        return unverifiable(format!(
            "the receipt's subject had {} dirty in-scope paths; no commit reproduces it",
            r.dirty
        ));
    }
    let reads = KbRepository {
        git_dir,
        work_tree: None,
    };
    let Some(commit) = kb_commit(git, reads, &r.commit, budgets()?, &CANCELLED)? else {
        return unverifiable(format!("commit {} is not in this clone", r.commit));
    };
    let unique = format!(
        "writfold-gate-verify-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    );
    let tree = std::env::temp_dir().join(unique);
    gate_worktree_add(git, git_dir, &tree, &commit, budgets()?, &CANCELLED)?;
    let outcome = rerun(ws, git, &tree, r, run, stem, now);
    let removed = gate_worktree_remove(git, git_dir, &tree, budgets()?, &CANCELLED);
    if removed.is_err() {
        let _ = std::fs::remove_dir_all(&tree);
    }
    let (again, ended, ms, scope_same) = outcome?;
    let recorded = format!("recorded {} {}", r.outcome.name(), receipt::ended(r));
    let rerun = format!("re-run {} {ended} in {ms} ms", again.name());
    let at = format!("at {}@{}", r.repo, r.commit.get(..12).unwrap_or(&r.commit));
    let mut text = format!("{recorded}; {rerun} {at}");
    if !scope_same {
        text.push_str("; the commit's in-scope content differs from the receipt's");
    }
    Ok(match (r.outcome, again) {
        (Outcome::Unknown, _) | (_, Outcome::Unknown) => {
            ("inconclusive", text, Some("unverifiable"))
        }
        (a, b) if a == b && scope_same => ("agree", text, None),
        _ => ("disagree", text, Some("disagree")),
    })
}

fn rerun(
    ws: &Workspace,
    git: &GitExecutable<'_>,
    tree: &Path,
    r: &Receipt,
    run: &str,
    stem: &str,
    now: i64,
) -> Result<(Outcome, String, u64, bool)> {
    let linked = crate::workspace::find_repo(tree)?
        .ok_or_else(|| KbError::Refused("the verification worktree is not a work tree".into()))?;
    let at = subject::Subject {
        top: linked.work_tree,
        git_dir: linked.git_dir,
        name: r.repo.clone(),
    };
    let patterns: Vec<&str> = r.patterns.iter().map(String::as_str).collect();
    let wanted = |p: &str| writfold_core::gate::in_scope(&patterns, p);
    let scoped = subject::snapshot(git, &at, &wanted)?.scoped(&patterns);
    let compact: String = time::stamp(now)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let local = super::run::local_dir(ws, run, &format!("{stem}.verify-{compact}"))
        .ok_or_else(|| KbError::Refused("gates need a KB under Git".into()))?;
    let artifacts = local.join("artifacts");
    std::fs::create_dir_all(&artifacts)?;
    // `{gate}` is the definition's directory: in the verification worktree
    // for a repository definition (as of the recorded commit), in the KB
    // for a KB definition (as it stands now).
    let gate_dir = match r.origin.as_str() {
        "repo" => tree.join(super::def::DIR),
        _ => ws.root.join(super::def::DIR),
    };
    let places = super::places::Places {
        gate: &gate_dir,
        artifacts: Some(&artifacts),
        candidate: None,
    };
    let e = exec::execute(
        &r.gate,
        &r.argv,
        &r.workdir,
        r.timeout_seconds,
        tree,
        &places,
    );
    std::fs::write(local.join("stdout.tail"), &e.stdout.bytes)?;
    std::fs::write(local.join("stderr.tail"), &e.stderr.bytes)?;
    let after = subject::snapshot(git, &at, &wanted)?.scoped(&patterns);
    let outcome = writfold_core::gate::outcome(e.ending, after != scoped);
    let ended = match (e.exit_code, e.timed_out, &e.error) {
        (Some(code), _, _) => format!("exit={code}"),
        (_, true, _) => "timeout".into(),
        (_, _, Some(_)) => "error".into(),
        _ => "signal".into(),
    };
    Ok((outcome, ended, e.duration_ms, scoped.digest == r.scope))
}
