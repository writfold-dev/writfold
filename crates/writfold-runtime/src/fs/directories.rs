use super::root::open_directory_at;
use super::{ContainedRoot, FileIdentity, FsError, PathError, RelativePath};
use rustix::fs::{Mode, fstat, fsync, mkdirat};

impl ContainedRoot {
    /// Create exactly one direct child, initially 0700. Never creates ancestors
    /// and never accepts an existing entry. The parent descriptor is retained;
    /// observed pathname checks are not prevention of races by other owners.
    /// The parent root must still check its own ancestor bindings after return.
    pub fn create_child(&self, path: &RelativePath) -> Result<Self, FsError> {
        let mut parts = path.as_path().components();
        let name = parts.next().ok_or(PathError::EmptyComponent)?.as_os_str();
        if parts.next().is_some() {
            return Err(PathError::MultipleComponents.into());
        }
        self.check_current()?;
        let parent = rustix::io::dup(&self.directory).map_err(FsError::from)?;
        mkdirat(&parent, name, Mode::RWXU).map_err(FsError::from)?;
        let directory = open_directory_at(&parent, name)?;
        let identity = FileIdentity::of(&fstat(&directory).map_err(FsError::from)?);
        let child = Self {
            directory,
            bindings: vec![(parent, name.to_owned(), identity)],
        };
        child.sync()?;
        self.check_current()?;
        Ok(child)
    }

    /// Explicitly synchronize this directory and its immediate named parent.
    /// Read-only callers need not invoke this. Reopening a child left by a failed
    /// creation does not, by itself, establish durable directory retention.
    pub fn sync(&self) -> Result<(), FsError> {
        self.check_current()?;
        fsync(&self.directory).map_err(FsError::RootDurabilityUnknown)?;
        if let Some((parent, _, _)) = self.bindings.last() {
            fsync(parent).map_err(FsError::RootDurabilityUnknown)?;
        }
        self.check_current()
    }
}
