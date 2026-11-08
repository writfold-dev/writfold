//! Read-only helpers for parent-prepared disposable native fixtures.
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use writfold_domain::ByteCount;
use writfold_runtime::fs::{ContainedRoot, PinnedFile, RelativePath};

pub type Result<T = (), E = Box<dyn Error>> = std::result::Result<T, E>;
pub fn count(value: u64) -> Result<ByteCount> {
    Ok(ByteCount::new(value)?)
}
pub fn pin(path: &Path) -> Result<PinnedFile> {
    if !path.is_absolute() {
        return Err("absolute retained tool path required".into());
    }
    let parent = path.parent().ok_or("tool parent missing")?;
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or("tool name missing")?;
    Ok(ContainedRoot::open(parent)?.pin(&RelativePath::new(name)?)?)
}

pub fn reference_snapshot(repo: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    let root = repo.join(".git/refs");
    let mut pending = vec![root.clone()];
    let mut result = BTreeMap::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                pending.push(path);
            } else {
                result.insert(path.strip_prefix(&root)?.to_owned(), fs::read(path)?);
            }
        }
    }
    Ok(result)
}
