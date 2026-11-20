//! `observe … --snapshot FILE [--store NAME]` (stores-next spec, section 2):
//! keep the bytes of a look in a pinned attached store and cite them from
//! the observation. Writfold never commits, so one command is run twice
//! with the user's commit between. The first copies a `FILE` from outside
//! the store to the default layout ([`crate::store::snapshot::layout`]; a
//! `FILE` already in the store's work tree stays where it is), writes the
//! provenance file beside it, records nothing and prints the commands. The
//! second, once both are committed, records the observation with a
//! `snapshot` citation of the committed file.
//!
//! What is recorded, and the only answer under which files are written into
//! the store, is [`writfold_core::store::snapshot`] (proved); a recorded
//! citation reads fresh under the pin (`snapshot_record_is_fresh`). Reading
//! the file, `HEAD` and `HEAD:<path>`, the create-new writes and the
//! printed commands are tested, not proved.
use super::artifact::whole;
use super::cite::{Fields, code_token, seen};
use super::span::{Blob, blobs};
use crate::error::{KbError, Result};
use crate::observe::Observation;
use crate::session::Session;
use crate::store::snapshot::{SUFFIX, layout, named, provenance, quote, work_tree};
use crate::store::{Freshness, Store, Stores};
use crate::workspace::{CANCELLED, budgets};
use std::path::{Path, PathBuf};
use writfold_core::observe::embeds;
use writfold_core::store::{AtPin, Commit, Snapshot, SnapshotFacts, Withheld, snapshot};
use writfold_runtime::git::{GitExecutable, KbRepository, kb_anchored, kb_commit};

/// Snapshots larger than this are refused: Git keeps every version.
pub const MAX_BYTES: u64 = 5 * 1024 * 1024;

fn refused(text: impl Into<String>) -> KbError {
    KbError::Refused(text.into())
}

/// The store `--store` names, or the only declared one.
fn pick<'a>(stores: &'a Stores, name: Option<&str>) -> Result<Option<&'a Store>> {
    if let Some(name) = name {
        return Ok(stores.get(name));
    }
    let all: Vec<&Store> = stores.iter().collect();
    match all.as_slice() {
        [one] => Ok(Some(one)),
        [] => Ok(None),
        _ => Err(refused(
            "--snapshot: several stores are declared; name one with --store",
        )),
    }
}

/// The file's canonical path and bytes: a regular file of at most 5 MiB.
fn read(file: &str) -> Result<(PathBuf, Vec<u8>)> {
    let path = Path::new(file);
    let meta = std::fs::symlink_metadata(path)
        .map_err(|e| KbError::Usage(format!("--snapshot {file}: {e}")))?;
    if !meta.is_file() {
        return Err(KbError::Usage(format!(
            "--snapshot {file}: not a regular file (a symbolic link is refused)"
        )));
    }
    if meta.len() > MAX_BYTES {
        return Err(KbError::Usage(format!(
            "--snapshot {file}: {} bytes, over the 5 MiB limit",
            meta.len()
        )));
    }
    Ok((std::fs::canonicalize(path)?, std::fs::read(path)?))
}

/// Write `bytes` at `path` unless a file is there: a synced temporary file
/// linked into place, so a crash leaves at most an untracked file.
fn create(path: &Path, bytes: &[u8]) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let dir = path
        .parent()
        .ok_or_else(|| refused("--snapshot: path has no parent"))?;
    std::fs::create_dir_all(dir)?;
    let temp = dir.join(format!(".writfold-snapshot-{}.tmp", std::process::id()));
    std::fs::write(&temp, bytes)?;
    std::fs::File::open(&temp)?.sync_all()?;
    let linked = std::fs::hard_link(&temp, path);
    std::fs::remove_file(&temp)?;
    linked?;
    super::atomic::sync_dir(dir)
}

/// What `HEAD:<path>` holds against the bytes whose SHA-256 is `sha`, the
/// worse of the file and its provenance file.
fn at(held: Option<&Blob>, sha: &str, prov: Option<&Blob>) -> AtPin {
    let file = match held {
        None => AtPin::Nothing,
        Some((_, b)) if super::atomic::digest(b) == sha => AtPin::Cited,
        Some(_) => AtPin::Other,
    };
    let prov = match prov.map(|(_, b)| named(b)) {
        None => AtPin::Nothing,
        Some(Some(s)) if s == sha => AtPin::Cited,
        Some(_) => AtPin::Other,
    };
    match (file, prov) {
        (AtPin::Nothing, _) | (_, AtPin::Nothing) => AtPin::Nothing,
        (AtPin::Other, _) | (_, AtPin::Other) => AtPin::Other,
        (AtPin::Cited, AtPin::Cited) => AtPin::Cited,
    }
}

/// The store's `HEAD`, whether a ref reaches it, and the file and its
/// provenance file there.
fn head(
    git: &GitExecutable<'_>,
    dir: &Path,
    rel: &str,
) -> Result<(Commit, Option<String>, Vec<Option<Blob>>)> {
    let reads = KbRepository {
        git_dir: dir,
        work_tree: None,
    };
    let Some(id) = kb_commit(git, reads, "HEAD", budgets()?, &CANCELLED)? else {
        return Ok((Commit::Absent, None, vec![None, None]));
    };
    let commit = match kb_anchored(git, reads, &id, budgets()?, &CANCELLED)? {
        true => Commit::Anchored,
        false => Commit::Unanchored,
    };
    let prov = format!("{rel}{SUFFIX}");
    let held = blobs(git, dir, id.as_str(), &[rel, &prov])?;
    Ok((commit, Some(id.as_str().to_owned()), held))
}

/// One snapshot as found on this machine.
struct Found<'a> {
    store: Option<&'a Store>,
    git_dir: Option<PathBuf>,
    tree: Option<PathBuf>,
    rel: String,
    copied: bool,
    bytes: Vec<u8>,
    sha: String,
}

fn find<'a>(
    session: &Session<'_>,
    stores: &'a Stores,
    o: &Observation,
    file: &str,
    name: Option<&str>,
) -> Result<Found<'a>> {
    let (given, bytes) = read(file)?;
    let sha = super::atomic::digest(&bytes);
    let store = pick(stores, name)?;
    let git_dir = store.and_then(|s| crate::store::resolve(Some(session.ws), s).git_dir);
    let tree = git_dir.as_deref().and_then(work_tree);
    let inside = tree.as_ref().and_then(|t| given.strip_prefix(t).ok());
    let (rel, copied) = match inside {
        Some(rel) => (rel.to_string_lossy().into_owned(), false),
        None => {
            let base = given.file_name().map(|n| n.to_string_lossy().into_owned());
            let base =
                base.ok_or_else(|| KbError::Usage(format!("--snapshot {file}: no file name")))?;
            (layout(o, &base, &sha), true)
        }
    };
    if !crate::stale::code_path(&rel) || rel.ends_with(SUFFIX) {
        return Err(KbError::Usage(format!(
            "--snapshot {file}: {rel:?} cannot be cited in the store"
        )));
    }
    Ok(Found {
        store,
        git_dir,
        tree,
        rel,
        copied,
        bytes,
        sha,
    })
}

/// Whether the work tree still holds the bytes its provenance file names.
fn intact(f: &Found<'_>) -> Result<bool> {
    let Some(tree) = &f.tree else { return Ok(true) };
    if f.copied
        && let Ok(there) = std::fs::read(tree.join(&f.rel))
        && super::atomic::digest(&there) != f.sha
    {
        return Err(refused(format!(
            "--snapshot: {} already holds other bytes; nothing was written or recorded",
            tree.join(&f.rel).display()
        )));
    }
    Ok(
        match std::fs::read(tree.join(format!("{}{SUFFIX}", f.rel))) {
            Ok(prov) => named(&prov).as_deref() == Some(f.sha.as_str()),
            Err(_) => true,
        },
    )
}

/// The `snapshot` citation token for an observation, or the refusal: after
/// writing the files into the store when it says the snapshot is not yet
/// committed. `again` is the `observe` command to run after the commit,
/// without `--snapshot`.
pub fn token(
    session: &Session<'_>,
    o: &Observation,
    file: &str,
    name: Option<&str>,
    again: &str,
) -> Result<String> {
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| refused("--snapshot needs Git"))?;
    let stores = crate::store::read(&session.ws.root)?;
    let f = find(session, &stores, o, file, name)?;
    let facts = SnapshotFacts {
        embeds: embeds(o.status),
        declared: f.store.is_some(),
        pinned: f.store.is_some_and(|s| s.freshness == Freshness::Pinned),
        local: f.tree.is_some(),
        intact: intact(&f)?,
    };
    let (commit, rev, held) = match &f.git_dir {
        Some(dir) if f.tree.is_some() => head(git, dir, &f.rel)?,
        _ => (Commit::Absent, None, vec![None, None]),
    };
    let (blob, prov) = (
        held.first().cloned().flatten(),
        held.get(1).cloned().flatten(),
    );
    match snapshot(&facts, &commit, &at(blob.as_ref(), &f.sha, prov.as_ref())) {
        Snapshot::Record => {
            let (Some(store), Some(rev), Some(blob)) = (f.store, rev, blob) else {
                return Err(refused("--snapshot: the committed file could not be read"));
            };
            code_token(&Fields {
                repo: &store.name,
                rev: &rev,
                spanned: &whole(&f.rel, &blob),
                sym: None,
                seen: &seen(session.now),
                ttl: None,
                url: &store.url,
                dirty: None,
            })
        }
        Snapshot::Withhold(Withheld::NotCommitted) => Err(stage(o, &f, again)?),
        Snapshot::Withhold(why) => Err(withheld(&why, &f, name)),
    }
}

/// Write the file (when copied) and its provenance into the store's work
/// tree, and refuse with the commands that commit them.
fn stage(o: &Observation, f: &Found<'_>, again: &str) -> Result<KbError> {
    let (Some(tree), Some(store)) = (&f.tree, f.store) else {
        return Ok(refused("--snapshot: the store has no work tree here"));
    };
    let base = Path::new(&f.rel)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if f.copied {
        create(&tree.join(&f.rel), &f.bytes)?;
    }
    let prov = format!("{}{SUFFIX}", f.rel);
    create(
        &tree.join(&prov),
        &provenance(o, &base, &f.sha, f.bytes.len()),
    )?;
    let add = match (f.copied, Path::new(&f.rel).parent()) {
        (true, Some(dir)) => vec![dir.to_string_lossy().into_owned()],
        _ => vec![f.rel.clone(), prov],
    };
    let add: Vec<String> = add.iter().map(|p| quote(p)).collect();
    let git = format!("git -C {}", quote(&tree.display().to_string()));
    let message = quote(&format!("snapshot: {} {}", o.source, o.locator));
    let path = quote(&tree.join(&f.rel).display().to_string());
    Ok(refused(format!(
        "the snapshot is not committed; nothing was recorded\n  stored: {}:{} ({} bytes)\n  next: {git} add -- {}\n  next: {git} commit -m {message} -- {}\n  next: {git} push\n  next: {again} --snapshot {path}",
        store.name,
        f.rel,
        f.bytes.len(),
        add.join(" "),
        add.join(" ")
    )))
}

fn withheld(why: &Withheld, f: &Found<'_>, name: Option<&str>) -> KbError {
    let store = f
        .store
        .map_or_else(|| name.unwrap_or("").to_owned(), |s| s.name.clone());
    refused(match why {
        Withheld::NoCitation => {
            "--snapshot is for ok and partial looks: a look that found nothing embeds no citation"
                .to_owned()
        }
        Withheld::NotAStore if name.is_some() => format!(
            "--snapshot: no store {store} is declared ({}/{store}.toml)",
            crate::store::DIR
        ),
        Withheld::NotAStore => format!("--snapshot: no store is declared ({}/)", crate::store::DIR),
        Withheld::NotPinned => format!(
            "--snapshot: store {store} is declared freshness = \"head\"; a snapshot is kept only in a pinned store"
        ),
        Withheld::NotLocal => format!(
            "--snapshot: store {store} has no clone with a work tree here (writfold kb repos)"
        ),
        Withheld::Damaged => format!(
            "--snapshot: {} no longer matches its provenance file; restore it or take a new snapshot",
            f.rel
        ),
        Withheld::OtherBytes => format!(
            "--snapshot: the committed {} differs from the snapshot's bytes (a Git filter or an edit); commit the bytes as taken",
            f.rel
        ),
        Withheld::Unanchored => {
            format!("--snapshot: no branch or tag reaches store {store}'s HEAD; put it on a branch")
        }
        Withheld::NotCommitted => "--snapshot: not committed".to_owned(),
    })
}
