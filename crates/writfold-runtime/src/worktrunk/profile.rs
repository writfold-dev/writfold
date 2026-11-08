use super::{Limits, RepositoryIdentity, Result, Source, WorktrunkError};
use crate::fs::{ContainedRoot, FsError, ReadExtent, RelativePath};
use crate::process;
use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, Dir, FileType, Mode, OFlags, fstat, openat, statat};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use writfold_domain::GitObjectFormat;

pub(super) fn relative(path: &str) -> Result<RelativePath> {
    RelativePath::new(path)
        .map_err(FsError::from)
        .map_err(Into::into)
}
pub(super) fn absolute(path: &Path) -> Result<()> {
    let text = path
        .to_str()
        .ok_or(WorktrunkError::Profile("UTF8 absolute path"))?;
    if !path.is_absolute()
        || text.len() > 4096
        || !text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/-_.".contains(&b))
        || text.split('/').any(|part| matches!(part, "." | ".."))
    {
        return Err(WorktrunkError::Profile("canonical portable absolute path"));
    }
    Ok(())
}
pub(super) fn root(path: &Path) -> Result<ContainedRoot> {
    absolute(path)?;
    let root = ContainedRoot::open(path)?;
    root.check_private()?;
    if std::fs::canonicalize(path)? != path {
        return Err(WorktrunkError::Profile("canonical root required"));
    }
    Ok(root)
}
pub(super) fn identity(root: &ContainedRoot) -> Result<RepositoryIdentity> {
    root.check_current()?;
    let stat = fstat(root.descriptor())?;
    Ok(RepositoryIdentity {
        device: stat.st_dev,
        inode: stat.st_ino,
    })
}
pub(super) fn absent(root: &ContainedRoot, name: &str) -> Result<()> {
    match statat(root.descriptor(), name, AtFlags::SYMLINK_NOFOLLOW) {
        Err(rustix::io::Errno::NOENT) => Ok(()),
        Err(error) => Err(error.into()),
        Ok(_) => Err(WorktrunkError::Conflict),
    }
}

pub(super) fn check(
    source: &Source,
    limits: Limits,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<[u8; 32]> {
    process::check_stop(deadline, cancelled)?;
    source.root.check_private()?;
    source.metadata.check_private()?;
    source.locks.check_private()?;
    let read = source.target.read(limits.metadata)?;
    if read.extent() != ReadExtent::Complete {
        return Err(WorktrunkError::Profile("config byte budget"));
    }
    config(read.bytes(), source.format)?;
    // All project hook/config profiles remain unsupported, including untracked
    // files. Empty WORKTRUNK_PROJECT_CONFIG_PATH is an additional defense.
    match source
        .root
        .pin(&relative(".config/wt.toml")?)
        .and_then(|file| file.open_read().map(|_| ()))
    {
        Err(FsError::Missing) => {}
        Err(error) => return Err(error.into()),
        Ok(()) => return Err(WorktrunkError::Profile("project Worktrunk config")),
    }
    for name in [
        "commondir",
        "gitdir",
        "shallow",
        "config.worktree",
        "packed-refs",
        "info/grafts",
        "info/attributes",
        "info/sparse-checkout",
        "objects/info/alternates",
        "objects/info/http-alternates",
    ] {
        absent(&source.metadata, name)?;
    }
    let packs = source.metadata.open_child(&relative("objects/pack")?)?;
    for entry in Dir::read_from(packs.descriptor())? {
        process::check_stop(deadline, cancelled)?;
        if !matches!(entry?.file_name().to_bytes(), b"." | b"..") {
            return Err(WorktrunkError::Profile("loose objects only"));
        }
    }
    let mut remaining = limits.metadata.get();
    walk(
        source.metadata.descriptor(),
        0,
        &mut remaining,
        deadline,
        cancelled,
    )?;
    fingerprint(&source.root, limits, deadline, cancelled)
}

/// Separate write-side config grammar. The immutable Git reader is unchanged.
/// Worktrunk's own inert history/cache writes are admitted, not arbitrary Git
/// config. Remotes, includes, attributes, aliases, filters and hook paths fail.
pub(super) fn config(bytes: &[u8], format: GitObjectFormat) -> Result<()> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| WorktrunkError::Profile("config encoding"))?;
    let mut section = "";
    let (mut version, mut object_format, mut bare) = (None, None, None);
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            section = match line {
                "[core]" => "core",
                "[extensions]" => "extensions",
                "[worktrunk]" => "worktrunk",
                "[worktrunk \"hints\"]" => "hints",
                _ => return Err(WorktrunkError::Profile("config section")),
            };
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or(WorktrunkError::Profile("config syntax"))?;
        let (key, value) = (key.trim(), value.trim());
        match section {
            "core" if key.eq_ignore_ascii_case("repositoryformatversion") && version.is_none() => {
                version = Some(value);
            }
            "core" if key.eq_ignore_ascii_case("bare") && bare.is_none() && value == "false" => {
                bare = Some(value);
            }
            "core"
                if ["filemode", "logallrefupdates"]
                    .iter()
                    .any(|k| key.eq_ignore_ascii_case(k))
                    && matches!(value, "true" | "false") => {}
            "extensions" if key.eq_ignore_ascii_case("objectformat") && object_format.is_none() => {
                object_format = Some(value);
            }
            "worktrunk"
                if matches!(key, "history" | "history-path" | "default-branch")
                    && !value.is_empty()
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b)) => {}
            "hints"
                if !key.is_empty()
                    && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    && value.parse::<u64>().is_ok() => {}
            _ => return Err(WorktrunkError::Profile("config key/value")),
        }
    }
    if bare != Some("false")
        || !matches!(
            (format, version, object_format),
            (GitObjectFormat::Sha1, Some("0"), None)
                | (GitObjectFormat::Sha1, Some("1"), Some("sha1"))
                | (GitObjectFormat::Sha256, Some("1"), Some("sha256"))
        )
    {
        return Err(WorktrunkError::Profile("repository format"));
    }
    Ok(())
}

fn entry(parent: &OwnedFd, name: &std::ffi::CStr) -> Result<(OwnedFd, FileType)> {
    let file = openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )?;
    let stat = fstat(&file)?;
    let kind = FileType::from_raw_mode(stat.st_mode);
    if stat.st_uid != rustix::process::geteuid().as_raw()
        || stat.st_mode & 0o022 != 0
        || !matches!(kind, FileType::Directory | FileType::RegularFile)
        || (kind == FileType::RegularFile && stat.st_nlink != 1)
    {
        return Err(WorktrunkError::Profile(
            "linked/special/foreign/writable repository entry",
        ));
    }
    Ok((file, kind))
}
fn charge(remaining: &mut u64, count: usize) -> Result<()> {
    *remaining = remaining
        .checked_sub(count as u64)
        .ok_or(WorktrunkError::Profile("filesystem byte/name budget"))?;
    Ok(())
}
fn walk(
    parent: &OwnedFd,
    depth: usize,
    remaining: &mut u64,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<()> {
    process::check_stop(deadline, cancelled)?;
    if depth > 64 {
        return Err(WorktrunkError::Profile("filesystem depth"));
    }
    for item in Dir::read_from(parent)? {
        process::check_stop(deadline, cancelled)?;
        let item = item?;
        let name = item.file_name();
        if matches!(name.to_bytes(), b"." | b"..") {
            continue;
        }
        charge(remaining, name.to_bytes().len() + 1)?;
        let (file, kind) = entry(parent, name)?;
        if kind == FileType::Directory {
            walk(&file, depth + 1, remaining, deadline, cancelled)?;
        }
    }
    Ok(())
}

pub(super) fn check_output(
    root: &ContainedRoot,
    limits: Limits,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<()> {
    process::check_stop(deadline, cancelled)?;
    root.check_current()?;
    let stat = fstat(root.descriptor())?;
    if stat.st_uid != rustix::process::geteuid().as_raw() || stat.st_mode & 0o022 != 0 {
        return Err(WorktrunkError::Profile("output owner/permissions"));
    }
    walk(
        root.descriptor(),
        0,
        &mut limits.metadata.get(),
        deadline,
        cancelled,
    )
}

pub(super) fn fingerprint(
    root: &ContainedRoot,
    limits: Limits,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<[u8; 32]> {
    let mut names = limits.metadata.get();
    let mut bytes = limits.checkout.get();
    let mut hash = Sha256::new();
    hash_tree(
        root.descriptor(),
        0,
        &mut names,
        &mut bytes,
        &mut hash,
        deadline,
        cancelled,
    )?;
    Ok(hash.finalize().into())
}
fn hash_tree(
    parent: &OwnedFd,
    depth: usize,
    names: &mut u64,
    bytes: &mut u64,
    hash: &mut Sha256,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<()> {
    process::check_stop(deadline, cancelled)?;
    if depth > 64 {
        return Err(WorktrunkError::Profile("source depth"));
    }
    // Directory enumeration order is not a stable identity after native writes.
    let mut entries = Vec::new();
    for item in Dir::read_from(parent)? {
        process::check_stop(deadline, cancelled)?;
        let item = item?;
        let name = item.file_name();
        if matches!(name.to_bytes(), b"." | b"..") || (depth == 0 && name.to_bytes() == b".git") {
            continue;
        }
        charge(names, name.to_bytes().len() + 1)?;
        entries.push(name.to_owned());
    }
    entries.sort();
    for name in entries {
        process::check_stop(deadline, cancelled)?;
        hash.update((name.to_bytes().len() as u64).to_le_bytes());
        hash.update(name.to_bytes());
        let (file, kind) = entry(parent, &name)?;
        let stat = fstat(&file)?;
        hash.update(stat.st_mode.to_le_bytes());
        if kind == FileType::Directory {
            hash_tree(&file, depth + 1, names, bytes, hash, deadline, cancelled)?;
        } else {
            hash.update(stat.st_size.to_le_bytes());
            let mut file = std::fs::File::from(file);
            let mut buffer = [0_u8; 8192];
            loop {
                process::check_stop(deadline, cancelled)?;
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                charge(bytes, count)?;
                hash.update(&buffer[..count]);
            }
        }
        hash.update([0xff]);
    }
    Ok(())
}
