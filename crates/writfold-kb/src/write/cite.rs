//! `cite make <repo> <path> <a-b>`: print a code citation token for lines
//! of a committed file. Only committed bytes are cited, at the commit of
//! `origin/HEAD` (else `HEAD`). A path whose work-tree bytes differ from
//! `HEAD` is refused unless `--allow-dirty`, which adds `dirty=<digest of
//! the work-tree bytes>`. Nothing is written; the token is printed for the
//! caller to place.
//!
//! The span reading and token rendering are shared with the Tuft adapter
//! (`code_token`, [`spans`]), so both produce the same tokens.
use crate::emit::{Answer, Frame, Output, Row};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::stale::repos;
use crate::time;
use crate::workspace::{CANCELLED, GitTool, Workspace, budgets};
use crate::{Request, write::atomic::digest};
use std::path::Path;
use writfold_domain::GitObjectId;
use writfold_runtime::git::{GitExecutable, KbRepository, kb_commit, kb_config};
use writfold_sources::citations::iter_cite_tokens;

pub use super::span::{Lines, Span, Spanned, blobs, spans};

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

/// The commit a citation pins: `origin/HEAD`, else `HEAD`.
fn cited_commit(git: &GitExecutable<'_>, git_dir: &Path) -> Result<GitObjectId> {
    let reads = KbRepository {
        git_dir,
        work_tree: None,
    };
    match kb_commit(
        git,
        reads,
        "refs/remotes/origin/HEAD",
        budgets()?,
        &CANCELLED,
    )? {
        Some(rev) => Ok(rev),
        None => kb_commit(git, reads, "HEAD", budgets()?, &CANCELLED)?
            .ok_or_else(|| KbError::Refused("the repository has no commit".into())),
    }
}

/// The committed lines a citation pins: commit, the span read there, and
/// the work-tree digest when the file differs from `HEAD`.
struct Pinned {
    rev: GitObjectId,
    spanned: Spanned,
    dirty: Option<String>,
}

fn pinned(git: &GitExecutable<'_>, dir: &Path, path: &str, make: &CiteMake) -> Result<Pinned> {
    let git_dir = repos::git_dir(dir)
        .ok_or_else(|| KbError::Refused(format!("{} is not a repository", dir.display())))?;
    let rev = cited_commit(git, &git_dir)?;
    let want = Span {
        path: path.to_owned(),
        lines: Lines::Range(make.first, make.last),
    };
    let spanned = spans(git, &git_dir, rev.as_str(), std::slice::from_ref(&want))?
        .pop()
        .ok_or_else(|| KbError::Usage("no span was read".into()))?;
    let dirty = dirty(git, &git_dir, dir, path)?;
    if dirty.is_some() && !make.allow_dirty {
        return Err(KbError::Refused(format!(
            "{path} differs from HEAD in the work tree; commit it, or pass --allow-dirty to cite the committed bytes"
        )));
    }
    Ok(Pinned {
        rev,
        spanned,
        dirty,
    })
}

/// The fields of one code citation.
pub struct Fields<'a> {
    pub repo: &'a str,
    pub rev: &'a str,
    pub spanned: &'a Spanned,
    pub sym: Option<&'a str>,
    pub seen: &'a str,
    pub ttl: Option<&'a str>,
    pub url: &'a str,
    pub dirty: Option<&'a str>,
}

/// A code citation token in the field order the KB's tokens use
/// (`kind=code repo rev path L blob span [sym] seen [ttl] url [dirty]`),
/// checked to parse back.
pub fn code_token(f: &Fields<'_>) -> Result<String> {
    let s = f.spanned;
    let mut fields = vec![
        ("kind", "code".to_owned()),
        ("repo", f.repo.to_owned()),
        ("rev", f.rev.to_owned()),
        ("path", s.path.clone()),
        ("L", format!("{}-{}", s.first, s.last)),
        ("blob", s.blob.clone()),
        ("span", s.span.clone()),
    ];
    fields.extend(f.sym.map(|v| ("sym", v.to_owned())));
    fields.push(("seen", f.seen.to_owned()));
    fields.extend(f.ttl.map(|v| ("ttl", v.to_owned())));
    fields.push(("url", f.url.to_owned()));
    fields.extend(f.dirty.map(|v| ("dirty", v.to_owned())));
    token(&fields)
}

/// `seen=`: the instant, to the minute.
pub fn seen(now: i64) -> String {
    format!("{}Z", &time::stamp(now)[..16])
}

/// `url=`: the repository's origin URL, else its directory.
pub fn url(git: &GitExecutable<'_>, git_dir: &Path, dir: &Path) -> Result<String> {
    let reads = KbRepository {
        git_dir,
        work_tree: None,
    };
    Ok(
        kb_config(git, reads, "remote.origin.url", budgets()?, &CANCELLED)?
            .unwrap_or_else(|| dir.to_string_lossy().into_owned()),
    )
}

/// The token from its fields, checked to parse back.
pub(crate) fn token(fields: &[(&str, String)]) -> Result<String> {
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
    let dir = repos::locate(Some(session.ws), &make.repo, &request.cwd).ok_or_else(|| {
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
    let url = url(&git, &git_dir, &dir)?;
    let text = code_token(&Fields {
        repo: &name,
        rev: pin.rev.as_str(),
        spanned: &pin.spanned,
        sym: make.sym.as_deref(),
        seen: &seen(now),
        ttl: make.ttl.as_deref(),
        url: &url,
        dirty: pin.dirty.as_deref(),
    })?;
    let (a, b) = (make.first, make.last);
    let answer = Answer {
        rows: vec![Row {
            kind: "cite",
            id: format!("code:{name}:{path}:{a}-{b}"),
            text,
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
    let committed = blobs(git, git_dir, "HEAD", &[path])?
        .pop()
        .flatten()
        .map(|(_, b)| b);
    let current = std::fs::read(dir.join(path)).ok();
    Ok(match (committed, current) {
        (Some(a), Some(b)) if a == b => None,
        (_, Some(b)) => Some(digest(&b)[..12].to_owned()),
        (_, None) => Some("missing".into()),
    })
}
