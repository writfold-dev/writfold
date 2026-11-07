use super::{GitBudgets, GitError, GitExecutable, GitRepository, traversal};
use crate::fs::RelativePath;
use crate::process::{self, ProcessLimits};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use writfold_domain::{ByteCount, ContentDigest, GitObjectId};

/// Exact retained bytes and identity, without freshness or validation authority.
#[derive(Debug)]
pub struct GitBlob {
    commit: GitObjectId,
    blob: GitObjectId,
    path: RelativePath,
    bytes: Vec<u8>,
    digest: ContentDigest,
}

impl GitBlob {
    pub fn commit(&self) -> &GitObjectId {
        &self.commit
    }
    pub fn blob(&self) -> &GitObjectId {
        &self.blob
    }
    pub fn path(&self) -> &RelativePath {
        &self.path
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn digest(&self) -> &ContentDigest {
        &self.digest
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// A cryptographically checked commit/path-to-blob binding, not read authority.
#[derive(Debug)]
pub struct GitEntry {
    commit: GitObjectId,
    blob: GitObjectId,
    path: RelativePath,
}

impl GitEntry {
    pub fn commit(&self) -> &GitObjectId {
        &self.commit
    }
    pub fn blob(&self) -> &GitObjectId {
        &self.blob
    }
    pub fn path(&self) -> &RelativePath {
        &self.path
    }
}

impl GitRepository {
    pub fn lookup_blob(
        &self,
        executable: &GitExecutable<'_>,
        commit: &GitObjectId,
        path: &RelativePath,
        budgets: GitBudgets,
        cancelled: &AtomicBool,
    ) -> Result<GitEntry, GitError> {
        let session = Session::new(self, executable, budgets, cancelled)?;
        let blob = traversal::lookup(&session, commit, path)?;
        Ok(GitEntry {
            commit: commit.clone(),
            blob,
            path: path.clone(),
        })
    }

    /// Never resolves a symbolic revision, follows a tree symlink, or uses HEAD.
    /// Paths are checked native bytes; no lossy UTF-8 conversion is performed.
    pub fn read_blob(
        &self,
        executable: &GitExecutable<'_>,
        commit: &GitObjectId,
        path: &RelativePath,
        expected_blob: &GitObjectId,
        budgets: GitBudgets,
        cancelled: &AtomicBool,
    ) -> Result<GitBlob, GitError> {
        let session = Session::new(self, executable, budgets, cancelled)?;
        if expected_blob.format() != self.format() {
            return Err(GitError::ObjectFormat);
        }
        let actual = traversal::lookup(&session, commit, path)?;
        if &actual != expected_blob {
            return Err(GitError::ObjectMismatch);
        }
        let bytes = session.object(expected_blob, "blob", budgets.blob)?;
        let digest = content_digest(&bytes)?;
        process::check_stop(session.deadline, cancelled)?;
        Ok(GitBlob {
            commit: commit.clone(),
            blob: expected_blob.clone(),
            path: path.clone(),
            bytes,
            digest,
        })
    }
}

pub(super) struct Session<'a, 'file> {
    pub(super) repository: &'a GitRepository,
    executable: &'a GitExecutable<'file>,
    pub(super) budgets: GitBudgets,
    deadline: Instant,
    cancelled: &'a AtomicBool,
}

impl<'a, 'file> Session<'a, 'file> {
    pub(super) fn new(
        repository: &'a GitRepository,
        executable: &'a GitExecutable<'file>,
        budgets: GitBudgets,
        cancelled: &'a AtomicBool,
    ) -> Result<Self, GitError> {
        let deadline = Instant::now()
            .checked_add(budgets.timeout)
            .ok_or(GitError::Malformed("deadline overflow"))?;
        process::check_stop(deadline, cancelled)?;
        repository.check_profile()?;
        Ok(Self {
            repository,
            executable,
            budgets,
            deadline,
            cancelled,
        })
    }

    pub(super) fn check_stop(&self) -> Result<(), GitError> {
        process::check_stop(self.deadline, self.cancelled)?;
        Ok(())
    }

    pub(super) fn object(
        &self,
        object: &GitObjectId,
        kind: &str,
        limit: ByteCount,
    ) -> Result<Vec<u8>, GitError> {
        if object.format() != self.repository.format() {
            return Err(GitError::ObjectFormat);
        }
        let actual = self.command(
            &[
                OsStr::new("cat-file"),
                OsStr::new("-t"),
                OsStr::new(object.as_str()),
            ],
            &[],
            self.budgets,
        )?;
        if actual.strip_suffix(b"\n") != Some(kind.as_bytes()) {
            return Err(GitError::ObjectType);
        }
        let size = self.command(
            &[
                OsStr::new("cat-file"),
                OsStr::new("-s"),
                OsStr::new(object.as_str()),
            ],
            &[],
            self.budgets,
        )?;
        let size = parse_size(&size)?;
        if size > limit.get() {
            return Err(GitError::Budget {
                stream: "object",
                limit: limit.get(),
            });
        }
        let stdout = ByteCount::new(self.budgets.stdout.get().min(size))
            .map_err(|_| GitError::Malformed("byte count"))?;
        let bytes = self.command(
            &[
                OsStr::new("cat-file"),
                OsStr::new(kind),
                OsStr::new(object.as_str()),
            ],
            &[],
            GitBudgets {
                stdout,
                ..self.budgets
            },
        )?;
        if bytes.len() as u64 != size {
            return Err(GitError::Malformed("object size"));
        }
        self.verify_hash(object, kind, &bytes)?;
        Ok(bytes)
    }

    fn verify_hash(&self, object: &GitObjectId, kind: &str, bytes: &[u8]) -> Result<(), GitError> {
        let actual = self.command(
            &[
                OsStr::new("hash-object"),
                OsStr::new("-t"),
                OsStr::new(kind),
                OsStr::new("--stdin"),
                OsStr::new("--no-filters"),
            ],
            bytes,
            self.budgets,
        )?;
        if actual.strip_suffix(b"\n") != Some(object.as_str().as_bytes()) {
            return Err(GitError::ObjectMismatch);
        }
        Ok(())
    }

    pub(super) fn command(
        &self,
        args: &[&OsStr],
        input: &[u8],
        budgets: GitBudgets,
    ) -> Result<Vec<u8>, GitError> {
        let mut command = super::command::command(self.executable, &self.repository.path())?;
        command.args(args);
        let output = process::run(
            &mut command,
            input,
            ProcessLimits::new(budgets.stdout, budgets.stderr),
            self.deadline,
            self.cancelled,
            self.executable.supervisor(),
        )?;
        self.executable.path()?;
        Ok(output.stdout)
    }
}

fn parse_size(bytes: &[u8]) -> Result<u64, GitError> {
    let digits = bytes
        .strip_suffix(b"\n")
        .ok_or(GitError::Malformed("size terminator"))?;
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return Err(GitError::Malformed("size digits"));
    }
    std::str::from_utf8(digits)
        .ok()
        .and_then(|value| value.parse().ok())
        .ok_or(GitError::Malformed("size overflow"))
}

fn content_digest(bytes: &[u8]) -> Result<ContentDigest, GitError> {
    use std::fmt::Write;
    let hash = Sha256::digest(bytes);
    let mut text = String::with_capacity(64);
    for byte in hash {
        write!(&mut text, "{byte:02x}").map_err(|_| GitError::Malformed("digest"))?;
    }
    text.parse()
        .map_err(|_| GitError::Malformed("content digest"))
}
