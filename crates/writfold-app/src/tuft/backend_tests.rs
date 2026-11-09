//! The read-only pinned-Git-tree backend against the B0 boundary's rules:
//! pinned history, digest binding, scope, measured paging, cursor binding,
//! typed refusal of deferred modes and of writes.
use super::tests::{fixture, git};
use super::{CANCELLED, MAX_SCOPE_BYTES, budgets, commit};
use crate::trial::{Result, Trial};
use std::fs;
use writfold_domain::GitObjectId;
use writfold_domain::citation::Coverage;
use writfold_store::backend::git::GitTreeBackend;
use writfold_store::backend::{
    Capability, ExactRead, HeadName, LookupField, LookupRequest, MatchMode, Order, RecordKey,
    Refusal, StorageBackend, WriteRequest,
};

type Backend<'a> = GitTreeBackend<'a>;

fn key(path: &str) -> Result<RecordKey> {
    Ok(RecordKey::new(path.to_owned())?)
}

fn substring(needle: &str, limit: usize) -> Result<LookupRequest> {
    Ok(LookupRequest::new(
        LookupField::Content,
        MatchMode::Substring,
        Order::Key,
        needle.to_owned(),
        limit,
    )?)
}

#[test]
fn git_tree_backend_keeps_the_boundary_rules() -> Result<()> {
    let f = fixture()?;
    fs::write(f.repo.join("src/lib.rs"), "changed\n")?;
    git(&f.repo, &["commit", "-qam", "second"])?;
    let after = commit(&git(&f.repo, &["rev-parse", "HEAD"])?)?;
    let trial = Trial::open(&f.state)?;
    let executable = trial.executable()?;
    let git_dir = fs::canonicalize(f.repo.join(".git"))?;
    let scope = "src/".to_owned();
    let mut backend = Backend::new(&executable, git_dir, scope, budgets()?, MAX_SCOPE_BYTES);
    let before = commit(&f.commit)?;
    reads(&backend, &before)?;
    paging(&backend, &before)?;
    cursors(&backend, &before, &after)?;
    refusals(&mut backend, &before)
}

/// Pinned history, digest binding, missing and out-of-scope keys, missing pin.
fn reads(backend: &Backend<'_>, before: &GitObjectId) -> Result<()> {
    let read = backend.read_exact(
        before,
        &ExactRead::new(key("src/lib.rs")?, None),
        &CANCELLED,
    )?;
    require(
        read.content() == b"let a = 1;\nlet a = 1;\nrun(1);\n",
        "pinned bytes",
    )?;
    require(read.pin() == before, "pin")?;
    let wrong = ExactRead::new(key("src/m0.rs")?, Some(read.digest().clone()));
    let refused = backend.read_exact(before, &wrong, &CANCELLED).err();
    require(refused == Some(Refusal::DigestMismatch), "digest binding")?;
    for absent in ["src/absent.rs", "docs/notes.md"] {
        let request = ExactRead::new(key(absent)?, None);
        let refused = backend.read_exact(before, &request, &CANCELLED).err();
        require(
            refused == Some(Refusal::NotFound),
            "absent or out-of-scope key",
        )?;
    }
    let missing = commit(&"1".repeat(40))?;
    let page = backend.lookup(&missing, &substring("let", 10)?, None, &CANCELLED);
    require(matches!(page, Err(Refusal::Pin(_))), "missing pin refused")?;
    Ok(())
}

/// Paging reproduces the unpaged result and ends complete over the scope.
fn paging(backend: &Backend<'_>, before: &GitObjectId) -> Result<()> {
    let whole = backend.lookup(before, &substring("let", 100)?, None, &CANCELLED)?;
    require(whole.hits().len() == 6, "hit count")?;
    let complete = Coverage::Measured {
        examined: 6,
        total: 6,
    };
    require(
        whole.coverage() == complete,
        "complete coverage of the scope",
    )?;
    let one = substring("let", 1)?;
    let mut cursor = None;
    let mut paged = Vec::new();
    loop {
        let page = backend.lookup(before, &one, cursor.as_ref(), &CANCELLED)?;
        paged.extend(page.hits().iter().map(|h| h.key().clone()));
        let Some(next) = page.next() else {
            require(page.coverage().complete(), "final page complete")?;
            break;
        };
        require(
            !page.coverage().complete(),
            "cursor only with partial coverage",
        )?;
        cursor = Some(next.clone());
    }
    let unpaged: Vec<_> = whole.hits().iter().map(|h| h.key().clone()).collect();
    require(paged == unpaged, "paged hits")?;
    Ok(())
}

/// A cursor resumes only its own pin and request.
fn cursors(backend: &Backend<'_>, before: &GitObjectId, after: &GitObjectId) -> Result<()> {
    let one = substring("let", 1)?;
    let first = backend.lookup(before, &one, None, &CANCELLED)?;
    let issued = first.next().ok_or("no cursor")?;
    let moved = backend.lookup(after, &one, Some(issued), &CANCELLED).err();
    require(
        moved == Some(Refusal::CursorMismatch),
        "cursor at another pin refused",
    )?;
    let other = substring("run", 1)?;
    let changed = backend
        .lookup(before, &other, Some(issued), &CANCELLED)
        .err();
    require(
        changed == Some(Refusal::CursorMismatch),
        "cursor for another request refused",
    )?;
    Ok(())
}

/// Deferred modes and writes are typed refusals.
fn refusals(backend: &mut Backend<'_>, before: &GitObjectId) -> Result<()> {
    for (mode, order, capability) in [
        (MatchMode::Fuzzy, Order::Key, Capability::Fuzzy),
        (MatchMode::Semantic, Order::Key, Capability::Semantic),
        (MatchMode::Substring, Order::Ranked, Capability::Ranking),
    ] {
        let request = LookupRequest::new(LookupField::Content, mode, order, "let".into(), 10)?;
        let refused = backend.lookup(before, &request, None, &CANCELLED).err();
        require(
            refused == Some(Refusal::Unsupported(capability)),
            "deferred mode refused",
        )?;
    }
    let write = WriteRequest {
        head: HeadName::new("main".into())?,
        expected_old: before.clone(),
        changes: Vec::new(),
    };
    let refused = backend.admit_write(&write, &CANCELLED).err();
    require(
        refused == Some(Refusal::Unavailable(Capability::Write)),
        "write refused as Unavailable",
    )
}

fn require(holds: bool, what: &str) -> Result<()> {
    if holds {
        Ok(())
    } else {
        Err(format!("backend rule violated: {what}").into())
    }
}
