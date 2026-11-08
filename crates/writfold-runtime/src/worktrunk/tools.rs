use super::{Result, WORKTRUNK_BYTES, WORKTRUNK_SHA256, WorktrunkError, profile};
use crate::fs::{ContainedRoot, PinnedFile, ReadExtent};
use crate::git::GitExecutable;
use std::fs::OpenOptions;
use std::os::unix::fs::{OpenOptionsExt, symlink};
use std::path::{Path, PathBuf};
use writfold_domain::{ByteCount, ContentDigest};

/// Owns a fresh private control directory; never removes it automatically.
/// The only PATH entry contains `git`, a symlink to the still-open admitted ELF
/// descriptor in this process, not a copied binary or executable shell wrapper.
#[derive(Debug)]
pub struct Worktrunk<'a, 'file> {
    pub(super) tool: GitExecutable<'a>,
    pub(super) git: &'a GitExecutable<'file>,
    pub(super) controls: PathBuf,
    root: ContainedRoot,
    config: PinnedFile,
}
impl<'a, 'file> Worktrunk<'a, 'file> {
    /// Exact v0.81.0 bytes are mandatory. `controls` must be a fresh absolute
    /// path outside source/destination trees. Failed admission retains artifacts.
    pub fn admit(
        file: &'a PinnedFile,
        git: &'a GitExecutable<'file>,
        controls: &Path,
    ) -> Result<Self> {
        let digest: ContentDigest = WORKTRUNK_SHA256
            .parse()
            .map_err(|_| WorktrunkError::Profile("tool digest"))?;
        let limit =
            ByteCount::new(WORKTRUNK_BYTES).map_err(|_| WorktrunkError::Profile("tool bytes"))?;
        let tool = GitExecutable::admit(file, &digest, limit, git.supervisor())?;
        profile::absolute(controls)?;
        let root = ContainedRoot::create(controls)?;
        root.check_private()?;
        // No ambient executable can be found through this directory. The sole
        // symlink is deliberate and checked separately from repository profiles.
        symlink(git.path()?, controls.join("git"))?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(controls.join("empty.toml"))?
            .sync_all()?;
        ContainedRoot::create(controls.join("home"))?;
        let config = root.pin(&profile::relative("empty.toml")?)?;
        let adapter = Self {
            tool,
            git,
            controls: controls.to_owned(),
            root,
            config,
        };
        adapter.check()?;
        Ok(adapter)
    }

    pub(super) fn check(&self) -> Result<()> {
        self.root.check_private()?;
        self.tool.path()?;
        let git = self.git.path()?;
        if std::fs::read_link(self.controls.join("git"))? != git {
            return Err(WorktrunkError::Profile("Git child binding changed"));
        }
        self.config.check_private()?;
        let empty = self
            .config
            .read(ByteCount::new(1).map_err(|_| WorktrunkError::Profile("config budget"))?)?;
        if empty.extent() != ReadExtent::Complete || !empty.bytes().is_empty() {
            return Err(WorktrunkError::Profile("nonempty control config"));
        }
        self.root
            .open_child(&profile::relative("home")?)?
            .check_private()?;
        Ok(())
    }
}
