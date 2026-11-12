//! `cite make <repo> <path> <a-b>`: print a code citation token for lines
//! of a committed file. Only committed bytes are cited, at the commit of
//! `origin/HEAD` (else `HEAD`). A path whose work-tree bytes differ from
//! `HEAD` is refused unless `--allow-dirty`, which adds `dirty=<digest of
//! the work-tree bytes>`. Nothing is written; the token is printed for the
//! caller to place.
use crate::emit::{Answer, Frame, Output, Row};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::stale::repos;
use crate::time;
use crate::workspace::{CANCELLED, GitTool, Workspace, budgets};
use crate::{Request, write::atomic::digest};
use std::path::Path;
use writfold_domain::GitObjectId;
use writfold_runtime::git::{
    GitExecutable, KbRepository, kb_batch_check, kb_commit, kb_config, pinned_blobs,
};
use writfold_sources::citations::{iter_cite_tokens, normalize_span};

#[derive(Clone, Debug, Default)]
pub struct CiteMake {
    pub repo: String,
    pub path: String,
    pub first: u32,
    pub last: u32,
    pub allow_dirty: bool,
    pub sym: Option<String>,
    pub ttl: Option<String>,
}

/// A token value: bare, or double-quoted when it holds whitespace, is
/// empty or starts with a quote. A quote or the closing bracket inside is
/// refused.
fn value(key: &str, text: &str) -> Result<String> {
    if text.contains(['"', '\u{27e7}']) || text.contains(['\n', '\r']) {
        return Err(KbError::Usage(format!(
            "{key} holds a character a token cannot carry"
        )));
    }
    Ok(if text.is_empty() || text.contains(char::is_whitespace) {
        format!("\"{text}\"")
    } else {
        text.to_owned()
    })
}

/// The blob at `<commit>:<path>` and its bytes, if the path holds a blob.
fn blob(
    git: &GitExecutable<'_>,
    dir: &Path,
    commit: &str,
    path: &str,
) -> Result<Option<(GitObjectId, Vec<u8>)>> {
    let reads = KbRepository {
        git_dir: dir,
        work_tree: None,
    };
    let names = [format!("{commit}:{path}")];
    let found = kb_batch_check(git, reads, &names, budgets()?, &CANCELLED)?;
    let Some(Some((id, kind))) = found.into_iter().next() else {
        return Ok(None);
    };
    if kind != "blob" {
        return Ok(None);
    }
    let bytes = pinned_blobs(git, dir, std::slice::from_ref(&id), budgets()?, &CANCELLED)?;
    Ok(bytes.into_iter().next().map(|b| (id, b)))
}

/// The committed lines a citation pins: commit, blob, span digest, and the
/// work-tree digest when the file differs from `HEAD`.
struct Pinned {
    rev: GitObjectId,
    blob: GitObjectId,
    span: String,
    dirty: Option<String>,
}

fn pinned(git: &GitExecutable<'_>, dir: &Path, path: &str, make: &CiteMake) -> Result<Pinned> {
    let git_dir = repos::git_dir(dir)
        .ok_or_else(|| KbError::Refused(format!("{} is not a repository", dir.display())))?;
    let reads = KbRepository {
        git_dir: &git_dir,
        work_tree: None,
    };
    let rev = match kb_commit(
        git,
        reads,
        "refs/remotes/origin/HEAD",
        budgets()?,
        &CANCELLED,
    )? {
        Some(rev) => rev,
        None => kb_commit(git, reads, "HEAD", budgets()?, &CANCELLED)?
            .ok_or_else(|| KbError::Refused("the repository has no commit".into()))?,
    };
    let (blob_id, bytes) = blob(git, &git_dir, rev.as_str(), path)?
        .ok_or_else(|| KbError::Refused(format!("{path} is not a file at {}", rev.as_str())))?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.lines().collect();
    let (a, b) = (make.first as usize, make.last as usize);
    let window = lines.get(a - 1..b).ok_or_else(|| {
        KbError::Usage(format!(
            "{path} has {} lines at {}",
            lines.len(),
            rev.as_str()
        ))
    })?;
    let span = normalize_span(window);
    let dirty = dirty(git, &git_dir, dir, path)?;
    if dirty.is_some() && !make.allow_dirty {
        return Err(KbError::Refused(format!(
            "{path} differs from HEAD in the work tree; commit it, or pass --allow-dirty to cite the committed bytes"
        )));
    }
    Ok(Pinned {
        rev,
        blob: blob_id,
        span,
        dirty,
    })
}

/// The token, in the field order the KB's tokens use, checked to parse back.
fn token(fields: &[(&str, String)]) -> Result<String> {
    let mut parts = Vec::new();
    for (key, text) in fields {
        parts.push(format!("{key}={}", value(key, text)?));
    }
    let token = format!("\u{27e6}cite {}\u{27e7}", parts.join(" "));
    let parsed = iter_cite_tokens(&token)
        .next()
        .and_then(|t| t.fields().ok().map(|_| t.complete()));
    if parsed != Some(true) {
        return Err(KbError::Usage("the token does not parse back".into()));
    }
    Ok(token)
}

fn checked(make: &CiteMake) -> Result<&str> {
    let path = make.path.trim_start_matches("./");
    if path.is_empty() || path.starts_with('/') || path.split('/').any(|p| p == "..") {
        return Err(KbError::Usage(format!(
            "{path:?} is not a path inside the repository"
        )));
    }
    if make.first == 0 || make.last < make.first {
        return Err(KbError::Usage(
            "the range must be a-b with 1 <= a <= b".into(),
        ));
    }
    if let Some(ttl) = &make.ttl
        && crate::time::ttl(Some(ttl)).is_none()
    {
        return Err(KbError::Usage("--ttl is <N>d, <N>h or none".into()));
    }
    Ok(path)
}

pub fn make(
    ws: &Workspace,
    tool: &GitTool,
    request: &Request,
    make: &CiteMake,
    now: i64,
) -> Result<Output> {
    let session = Session::open(ws, Some(tool), false, now)?;
    let git = tool.admit()?;
    let path = checked(make)?;
    let dir = repos::locate(&session, &make.repo, &request.cwd).ok_or_else(|| {
        KbError::Refused(format!(
            "no local repository {:?} (map it in <git dir>/writfold/repos)",
            make.repo
        ))
    })?;
    let pin = pinned(&git, &dir, path, make)?;
    let name = if make.repo.contains('/') || make.repo.starts_with('.') {
        dir.file_name()
            .map_or_else(|| make.repo.clone(), |n| n.to_string_lossy().into_owned())
    } else {
        make.repo.clone()
    };
    let git_dir = repos::git_dir(&dir).unwrap_or_else(|| dir.clone());
    let reads = KbRepository {
        git_dir: &git_dir,
        work_tree: None,
    };
    let url = kb_config(&git, reads, "remote.origin.url", budgets()?, &CANCELLED)?
        .unwrap_or_else(|| dir.to_string_lossy().into_owned());
    let seen = time::stamp(now);
    let (a, b) = (make.first, make.last);
    let mut fields = vec![
        ("kind", "code".to_owned()),
        ("repo", name.clone()),
        ("rev", pin.rev.as_str().to_owned()),
        ("path", path.to_owned()),
        ("L", format!("{a}-{b}")),
        ("blob", pin.blob.as_str().to_owned()),
        ("span", pin.span),
    ];
    fields.extend(make.sym.iter().map(|s| ("sym", s.clone())));
    fields.push(("seen", format!("{}Z", &seen[..16])));
    fields.extend(make.ttl.iter().map(|t| ("ttl", t.clone())));
    fields.push(("url", url));
    fields.extend(pin.dirty.map(|d| ("dirty", d)));
    let answer = Answer {
        rows: vec![Row {
            kind: "cite",
            id: format!("code:{name}:{path}:{a}-{b}"),
            text: token(&fields)?,
            ..Row::default()
        }],
        notes: vec![format!(
            "cites {name}@{}:{path}:{a}-{b}",
            &pin.rev.as_str()[..12]
        )],
        ..Answer::default()
    };
    let frame = Frame {
        verb: "cite-make".into(),
        args: vec![name, path.to_owned(), format!("{a}-{b}")],
        budget: request.budget,
        cursor: request.cursor.clone(),
        json: request.json,
    };
    session.emit(&answer, &frame)
}

/// `Some(<12 hex of the work-tree bytes>)` when the work-tree file differs
/// from `HEAD:path` (or is missing); `None` when equal or bare.
fn dirty(
    git: &GitExecutable<'_>,
    git_dir: &Path,
    dir: &Path,
    path: &str,
) -> Result<Option<String>> {
    if repos::git_dir(dir).as_deref() == Some(dir) {
        return Ok(None);
    }
    let committed = blob(git, git_dir, "HEAD", path)?.map(|(_, b)| b);
    let current = std::fs::read(dir.join(path)).ok();
    Ok(match (committed, current) {
        (Some(a), Some(b)) if a == b => None,
        (_, Some(b)) => Some(digest(&b)[..12].to_owned()),
        (_, None) => Some("missing".into()),
    })
}
