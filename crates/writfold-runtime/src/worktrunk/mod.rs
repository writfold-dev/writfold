//! Conservative Worktrunk v0.81.0 native creation, not owner/grant authority.
//!
//! Linux procfs, admitted ELF/loader bytes, stable private administrative roots,
//! cooperating writers and process-group/subreaper discipline are assumptions.
//! This is not a hostile same-uid filesystem sandbox. No shell, network, ambient
//! configuration, Git replacement for Worktrunk, retry or uncertain-tree cleanup.
//! Only standalone, loose-object repositories with the closed config grammar are
//! supported. Project config, symlinks, submodules and special files are refused.
//! User/system configs are empty, project config is additionally disabled, all
//! Worktrunk hooks are disabled and Git hooks/filters/helpers are closed off.
//! The caller must retain a durable claim BEFORE create; this API grants none.
//! Repository and destination leases cover preflight, issue and independent
//! observation, including descendant cleanup. Unknown is never a no-effect claim.
//! Successful bindings describe live native facts, not durable publication.
//!
//! Roots must have portable absolute paths and private 0700 ownership; new
//! native files/directories must not be group/other-writable (use umask 077 or
//! 022). No packed refs/objects, alternates, sparse/shallow repositories, remotes,
//! includes or checkout attribute files are admitted. Existing Worktrunk history
//! and numeric hint counters are inert allowlisted metadata, not hook authority.
//! Pin source: v0.81.0 / e42f0f6fef2fdbd98fc92e5cd6c3cbea55b65b38. In that source,
//! empty WORKTRUNK_PROJECT_CONFIG_PATH disables project discovery, the explicit
//! system path selects only empty config, and switch's verify=false gates both
//! foreground execution and background hook plans. These assumptions still need
//! the parent's retained-binary smoke; bytes alone do not qualify behavior.
//! Interrupted creation and reconciliation are not provided by this component.
//! Every successful-process observation failure retains its bounded stdout in
//! Unknown; other native failures retain the process error (including bounded
//! stderr for nonzero exits). Neither is permission to retry or delete artifacts.
mod command;
mod create;
mod integrity;
mod profile;
#[cfg(test)]
mod tests;
mod tools;
mod tree;

use crate::fs::{ContainedRoot, FsError, ManagedLocks, PinnedFile};
use crate::git::GitError;
use crate::process::ProcessError;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
pub use tools::Worktrunk;
use writfold_domain::{ByteCount, GitObjectFormat, GitObjectId};

/// The only admitted artifact identity; matching bytes is not qualification.
pub const WORKTRUNK_SHA256: &str =
    "1f3d97a5c263f87345d03a1e439f5d53efa5a468e8bde10e724cbd9e5e79de9b";
pub const WORKTRUNK_BYTES: u64 = 29_099_264;

#[derive(Debug, Error)]
pub enum WorktrunkError {
    #[error(transparent)]
    Filesystem(#[from] FsError),
    #[error(transparent)]
    Git(#[from] GitError),
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("unsupported Worktrunk native profile: {0}")]
    Profile(&'static str),
    #[error("native observation differs from requested binding: {0}")]
    Observation(&'static str),
    #[error("destination or branch is already occupied")]
    Conflict,
}
impl From<rustix::io::Errno> for WorktrunkError {
    fn from(value: rustix::io::Errno) -> Self {
        Self::Io(value.into())
    }
}

#[derive(Debug, Error)]
pub enum CreateError {
    #[error("worktree creation was not issued: {0}")]
    Unissued(#[source] WorktrunkError),
    #[error("worktree creation may have occurred; retain and reconcile: {reason}")]
    Unknown {
        #[source]
        reason: WorktrunkError,
        /// Available when Worktrunk completed successfully but verification failed.
        worktrunk_stdout: Option<Vec<u8>>,
    },
}
pub type Result<T, E = WorktrunkError> = std::result::Result<T, E>;

/// A deliberately narrow branch profile: one portable alphanumeric/hyphen/
/// underscore component, starting with an ASCII letter. No revision expressions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Branch(String);
impl Branch {
    pub fn new(value: &str) -> Result<Self> {
        check_name(value)?;
        Ok(Self(value.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn check_name(value: &str) -> Result<()> {
    if value == "HEAD"
        || value.len() > 128
        || !value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(WorktrunkError::Profile("single-component branch"));
    }
    Ok(())
}

/// Immutable requested native binding; admission remains separate from authority.
#[derive(Clone, Copy, Debug)]
pub struct CreateRequest<'a> {
    pub leaf: &'a crate::fs::RelativePath,
    pub branch: &'a Branch,
    pub base: &'a GitObjectId,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub stdout: ByteCount,
    pub stderr: ByteCount,
    pub metadata: ByteCount,
    pub checkout: ByteCount,
    pub timeout: Duration,
}

/// Native device/inode identity of the actual standalone common directory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RepositoryIdentity {
    pub device: u64,
    pub inode: u64,
}

/// Observed facts only. Raw bounded Worktrunk stdout is retained, not decoded as
/// authority. Git independently establishes all returned binding fields.
#[derive(Debug)]
pub struct Created {
    pub repository: RepositoryIdentity,
    pub common_directory: PathBuf,
    pub path: PathBuf,
    pub branch: Branch,
    pub base: GitObjectId,
    pub worktrunk_stdout: Vec<u8>,
}

/// Preprovision `.git/writfold-locks` (0700) and keep it permanent. All source
/// writers/recovery, including GitWriter, share its `knowledge` lease.
#[derive(Debug)]
pub struct Source {
    path: PathBuf,
    root: ContainedRoot,
    metadata: ContainedRoot,
    locks: ManagedLocks,
    target: PinnedFile,
    format: GitObjectFormat,
}
impl Source {
    pub fn open(path: &Path, format: GitObjectFormat) -> Result<Self> {
        let root = profile::root(path)?;
        let metadata = root.open_child(&profile::relative(".git")?)?;
        metadata.check_private()?;
        let locks =
            ManagedLocks::from_root(metadata.open_child(&profile::relative("writfold-locks")?)?);
        locks.check_private()?;
        let target = metadata.pin(&profile::relative("config")?)?;
        Ok(Self {
            path: path.to_owned(),
            root,
            metadata,
            locks,
            target,
            format,
        })
    }
}

/// A private destination parent with permanent `writfold-locks` (0700).
/// Its single `worktrunk` lease serializes all destination allocation/recovery.
#[derive(Debug)]
pub struct Destinations {
    path: PathBuf,
    root: ContainedRoot,
    locks: ManagedLocks,
    target: PinnedFile,
}
impl Destinations {
    pub fn open(path: &Path) -> Result<Self> {
        let root = profile::root(path)?;
        let locks =
            ManagedLocks::from_root(root.open_child(&profile::relative("writfold-locks")?)?);
        locks.check_private()?;
        let target = root.pin(&profile::relative("allocation")?)?;
        Ok(Self {
            path: path.to_owned(),
            root,
            locks,
            target,
        })
    }
}
