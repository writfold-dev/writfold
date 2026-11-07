use super::NativeError;
use crate::fs::{ContainedRoot, OpenedFile, PinnedFile};
use crate::process::ProcessSupervisor;
use rustix::fd::{AsRawFd, OwnedFd};
use rustix::fs::{AtFlags, Mode, OFlags, Stat, fstat, fsync, openat, statat};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use writfold_domain::{ByteCount, ContentDigest};

/// Pinned native ELF; byte identity is separate from backend generation identity.
#[derive(Debug)]
pub struct NativeExecutable<'a> {
    opened: OpenedFile<'a>,
    pub(super) supervisor: &'a ProcessSupervisor,
}

impl<'a> NativeExecutable<'a> {
    pub fn admit(
        file: &'a PinnedFile,
        expected: &ContentDigest,
        limit: ByteCount,
        supervisor: &'a ProcessSupervisor,
    ) -> Result<Self, NativeError> {
        supervisor.ensure_current()?;
        let opened = file.open_read()?;
        if &opened.revision(limit)? != expected {
            return Err(NativeError::ExecutableMismatch);
        }
        let mut header = [0; 4];
        if opened.read_prefix(&mut header)? != 4 || header != *b"\x7fELF" {
            return Err(NativeError::Input("native ELF required"));
        }
        Ok(Self { opened, supervisor })
    }

    pub(super) fn check(&self) -> Result<(), NativeError> {
        self.opened.check_unchanged()?;
        self.supervisor.ensure_current()?;
        Ok(())
    }

    pub(super) fn command(&self, root: &NativeRoot) -> Result<Command, NativeError> {
        self.check()?;
        root.check()?;
        let mut command = Command::new(descriptor_path(self.opened.descriptor()));
        command
            .env_clear()
            .current_dir(descriptor_path(root.root.descriptor()));
        command
            .env("PATH", "")
            .env("HOME", "/nonexistent")
            .env("XDG_CONFIG_HOME", "/nonexistent");
        command
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .env("TZ", "UTC")
            .env("TOKIO_WORKER_THREADS", "2")
            .env("OMP_NUM_THREADS", "2")
            .env("OPENBLAS_NUM_THREADS", "2");
        Ok(command)
    }
}

/// A fixed canonical local URI with a live descriptor and checked pathname.
/// Native storage resolves that URI by pathname, not through this descriptor.
/// The administrative owner MUST keep the root and its ancestors immutable
/// throughout native ownership. Checks detect observed drift, not rename ABA
/// between checks; non-cooperating namespace mutation is unsupported.
#[derive(Debug)]
pub struct NativeRoot {
    pub(super) root: ContainedRoot,
    pub(super) uri: String,
}

impl NativeRoot {
    /// Adopt a caller-prepared, exclusively coordinated local native root.
    /// This does not unlock stale native admission or certify its contents.
    pub fn prepared(path: impl AsRef<Path>) -> Result<Self, NativeError> {
        Self::bind(ContainedRoot::open(path)?)
    }

    pub(super) fn bind(root: ContainedRoot) -> Result<Self, NativeError> {
        root.check_current()?;
        let canonical = std::fs::canonicalize(descriptor_path(root.descriptor()))?;
        // A deliberately narrow file-URI profile avoids interpretation differences
        // (percent decoding, query/fragment components, and non-UTF8 paths).
        let path = canonical
            .to_str()
            .ok_or(NativeError::Input("UTF-8 root required"))?;
        if !path.starts_with('/')
            || !path
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"/._-".contains(&c))
        {
            return Err(NativeError::Input(
                "root must use plain canonical file-URI characters",
            ));
        }
        let canonical_root = ContainedRoot::open(&canonical)?;
        let original = fstat(root.descriptor()).map_err(crate::fs::FsError::from)?;
        let bound = fstat(canonical_root.descriptor()).map_err(crate::fs::FsError::from)?;
        if original.st_dev != bound.st_dev || original.st_ino != bound.st_ino {
            return Err(crate::fs::FsError::ChangedSource.into());
        }
        root.check_current()?;
        Ok(Self {
            root: canonical_root,
            uri: format!("file://{path}"),
        })
    }

    pub fn canonical_uri(&self) -> &str {
        &self.uri
    }

    pub(super) fn check(&self) -> Result<(), NativeError> {
        self.root.check_current()?;
        Ok(())
    }
}

/// Explicit actor/token binding. Debug deliberately does not reveal tokens.
pub struct Credential {
    pub(super) actor: String,
    pub(super) token: String,
}

impl Credential {
    pub fn new(actor: &str, token: &str) -> Result<Self, NativeError> {
        if actor.is_empty()
            || actor.len() > 128
            || !actor
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        {
            return Err(NativeError::Input("invalid actor"));
        }
        if token.len() < 32
            || token.len() > 4096
            || !token
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-._~+/=".contains(&c))
        {
            return Err(NativeError::Input(
                "bearer token must be 32..4096 safe ASCII bytes",
            ));
        }
        Ok(Self {
            actor: actor.to_owned(),
            token: token.to_owned(),
        })
    }

    pub fn actor(&self) -> &str {
        &self.actor
    }
}

pub(super) struct Credentials {
    root: ContainedRoot,
    file: PrivateFile,
    entries: Vec<Credential>,
}

impl Credentials {
    pub(super) fn create(path: &Path, entries: Vec<Credential>) -> Result<Self, NativeError> {
        if entries.is_empty() || entries.len() > 32 {
            return Err(NativeError::Input("provide 1..32 explicit credentials"));
        }
        let mut tokens = serde_json::Map::new();
        for (index, entry) in entries.iter().enumerate() {
            if entries[..index].iter().any(|old| old.token == entry.token)
                || tokens
                    .insert(
                        entry.actor.clone(),
                        serde_json::Value::String(entry.token.clone()),
                    )
                    .is_some()
            {
                return Err(NativeError::Input("duplicate actor or bearer token"));
            }
        }
        let root = ContainedRoot::create(path)?;
        let bytes = serde_json::to_vec(&tokens)?;
        let file = PrivateFile::create(&root, "tokens.json", &bytes)?;
        Ok(Self {
            root,
            file,
            entries,
        })
    }

    pub(super) fn path(&self) -> PathBuf {
        descriptor_path(&self.file.fd)
    }

    pub(super) fn token(&self, actor: &str) -> Result<&str, NativeError> {
        self.entries
            .iter()
            .find(|entry| entry.actor == actor)
            .map(|entry| entry.token.as_str())
            .ok_or(NativeError::Input("actor not in admitted credential set"))
    }

    pub(super) fn check(&self) -> Result<(), NativeError> {
        self.root.check_current()?;
        self.file.check(&self.root)
    }
}

/// Retain both the private inode and its name; no mutable pathname is passed
/// as a credential source to the child.
pub(super) struct PrivateFile {
    fd: OwnedFd,
    name: &'static str,
    stamp: Stat,
}

impl PrivateFile {
    pub(super) fn create(
        root: &ContainedRoot,
        name: &'static str,
        bytes: &[u8],
    ) -> Result<Self, NativeError> {
        root.check_current()?;
        let fd = openat(
            root.descriptor(),
            name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(crate::fs::FsError::from)?;
        let mut file = File::from(fd);
        file.write_all(bytes)?;
        file.sync_all()?;
        fsync(root.descriptor()).map_err(crate::fs::FsError::from)?;
        let fd = OwnedFd::from(file);
        let stamp = fstat(&fd).map_err(crate::fs::FsError::from)?;
        let bound = Self { fd, name, stamp };
        bound.check(root)?;
        Ok(bound)
    }

    pub(super) fn check(&self, root: &ContainedRoot) -> Result<(), NativeError> {
        let named = statat(root.descriptor(), self.name, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(crate::fs::FsError::from)?;
        let opened = fstat(&self.fd).map_err(crate::fs::FsError::from)?;
        if !same_stamp(&self.stamp, &named)
            || !same_stamp(&self.stamp, &opened)
            || opened.st_nlink != 1
        {
            return Err(crate::fs::FsError::ChangedSource.into());
        }
        Ok(())
    }
}

fn same_stamp(a: &Stat, b: &Stat) -> bool {
    a.st_dev == b.st_dev
        && a.st_ino == b.st_ino
        && a.st_size == b.st_size
        && a.st_mode == b.st_mode
        && a.st_uid == b.st_uid
        && a.st_gid == b.st_gid
        && a.st_nlink == b.st_nlink
        && a.st_mtime == b.st_mtime
        && a.st_mtime_nsec == b.st_mtime_nsec
        && a.st_ctime == b.st_ctime
        && a.st_ctime_nsec == b.st_ctime_nsec
}

pub(super) fn descriptor_path(fd: &OwnedFd) -> PathBuf {
    PathBuf::from(format!(
        "/proc/{}/fd/{}",
        std::process::id(),
        fd.as_raw_fd()
    ))
}
