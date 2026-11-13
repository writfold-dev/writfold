//! Read-only, descriptor-relative work-tree search. No symlink is followed.
use super::search::{Collector, Found};
use super::{CANCELLED, MAX_SCOPE_BYTES, budgets, files};
use crate::trial::Result;
use std::fmt::Write as _;
use std::path::Path;
use tuft::emit::{Emitted, Format};
use tuft::ir::FindPlan;
use tuft::view::Stop;
use writfold_domain::ByteCount;
use writfold_domain::citation::Coverage;
use writfold_runtime::fs::{ContainedRoot, ReadExtent, RelativePath};
use writfold_runtime::git::{GitExecutable, KbRepository, kb_ls_files, pinned_blobs, pinned_tree};

pub(super) fn find(
    executable: &GitExecutable<'_>,
    repo: &Path,
    git_dir: &Path,
    plan: &FindPlan,
) -> Result<(Found, usize)> {
    let repo = std::fs::canonicalize(repo)?;
    let root = ContainedRoot::open(&repo)?;
    let repository = KbRepository {
        git_dir,
        work_tree: Some(&repo),
    };
    let mut paths = kb_ls_files(executable, repository, ".", budgets()?, &CANCELLED)?;
    paths.retain(|path| tuft::scope::matches(&plan.scope, path));
    paths.sort();
    paths.dedup();
    let total = paths.len() as u128;
    let mut collector = Collector::new(plan);
    let (mut examined, mut skipped, mut consumed) = (0, 0, 0);
    let mut snapshot = String::new();
    let mut stop = None;
    for path in paths {
        let content = match read(&root, &path, MAX_SCOPE_BYTES - consumed) {
            Ok(content) => content,
            Err(_) => {
                skipped += 1;
                let _ = writeln!(snapshot, "{}:{path}:unreadable", path.len());
                continue;
            }
        };
        consumed += content.len() as u64;
        let _ = writeln!(
            snapshot,
            "{}:{path}:{}",
            path.len(),
            tuft::digest_hex(&content)
        );
        let matches = files::matches(plan, &content)?;
        if !collector.add(&path, &content, &matches) {
            stop = Some(Stop::HitLimit);
            break;
        }
        examined += 1;
    }
    root.check_current()?;
    let mut found = collector.finish(Coverage::Measured { examined, total }, stop);
    found.report.snapshot = Some(tuft::digest_hex(snapshot.as_bytes()));
    mark_dirty(executable, git_dir, plan, &mut found)?;
    Ok((found, skipped))
}

fn read(root: &ContainedRoot, path: &str, remaining: u64) -> Result<Vec<u8>> {
    if path.contains('\u{fffd}') {
        return Err("unsupported non-UTF-8 path".into());
    }
    let relative = RelativePath::new(path)?;
    let file = root.pin(&relative)?;
    let read = file.read(ByteCount::new(remaining)?)?;
    if read.extent() != ReadExtent::Complete {
        return Err("file exceeds remaining scope budget".into());
    }
    Ok(read.into_bytes())
}

fn mark_dirty(
    executable: &GitExecutable<'_>,
    git_dir: &Path,
    plan: &FindPlan,
    found: &mut Found,
) -> Result<()> {
    let pin = super::commit(&plan.commit)?;
    let entries = pinned_tree(executable, git_dir, &pin, budgets()?, &CANCELLED)?;
    let entries: Vec<_> = entries
        .into_iter()
        .filter(|entry| found.files.contains_key(&entry.path))
        .collect();
    let ids: Vec<_> = entries.iter().map(|entry| entry.blob.clone()).collect();
    let contents = pinned_blobs(executable, git_dir, &ids, budgets()?, &CANCELLED)?;
    found.dirty.extend(found.files.keys().cloned());
    for (entry, bytes) in entries.into_iter().zip(contents) {
        if found.files.get(&entry.path) == Some(&bytes) {
            found.dirty.remove(&entry.path);
        }
    }
    Ok(())
}

pub(super) fn ensure_external_state(repo: &Path, git_dir: &Path, state: &Path) -> Result<()> {
    let repo = std::fs::canonicalize(repo)?;
    let state = std::fs::canonicalize(state)?;
    if state.starts_with(&repo) && !state.starts_with(git_dir) {
        return Err("working-tree evidence state must be outside source files (use Git metadata or an external directory)".into());
    }
    Ok(())
}

pub(super) fn annotate(emitted: &mut Emitted, dirty: usize, skipped: usize) -> Result<()> {
    let metadata = format!(" source=working-tree dirty={dirty} skipped={skipped}");
    emitted.trailer.push_str(&metadata);
    match emitted.format {
        Format::Compact => {}
        Format::Verbose => {
            let end = emitted.view.find('\n').ok_or("missing verbose envelope")?;
            emitted.view.insert_str(end, &metadata);
        }
        Format::Json => {
            let body = emitted
                .view
                .strip_suffix('}')
                .ok_or("missing JSON object")?;
            emitted.view = format!(
                "{body},\"working_tree\":{{\"dirty_files\":{dirty},\"skipped_files\":{skipped}}}}}"
            );
        }
    }
    Ok(())
}
