use super::identity::digest;
use super::temporary::Temporary;
use super::{BudgetKind, FileIdentity, FsError, LockedFile, OpenedFile, WriteError};
use rustix::fs::fsync;
use writfold_domain::{ByteCount, ContentDigest};

/// Constructed only after complete file writes, file fsync, publication and
/// parent fsync succeed. This is not a hardware power-loss guarantee.
#[derive(Debug)]
pub struct SyncedWrite {
    digest: ContentDigest,
    bytes: usize,
    identity: FileIdentity,
}

impl SyncedWrite {
    pub fn digest(&self) -> &ContentDigest {
        &self.digest
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn identity(&self) -> FileIdentity {
        self.identity
    }
}

impl LockedFile<'_> {
    /// Exclusive creation: a pre-existing destination is never replaced.
    pub fn create(&mut self, content: &[u8], limit: ByteCount) -> Result<SyncedWrite, WriteError> {
        check_write_budget(content, limit)?;
        self.publish(content, None)
    }

    /// Revalidate the full expected content under this managed writer lock.
    /// `None` means absent. A truncated current file cannot authorize a write.
    /// All cooperating writers must choose this same namespace and logical key.
    pub fn compare_and_write(
        &mut self,
        expected: Option<&ContentDigest>,
        content: &[u8],
        read_limit: ByteCount,
        write_limit: ByteCount,
    ) -> Result<SyncedWrite, WriteError> {
        check_write_budget(content, write_limit)?;
        self.ensure_lock()?;
        let previous = self.expected_source(expected, read_limit)?;
        self.publish(content, previous.as_ref())
    }

    fn expected_source(
        &self,
        expected: Option<&ContentDigest>,
        read_limit: ByteCount,
    ) -> Result<Option<OpenedFile<'_>>, FsError> {
        let opened = match self.target.open_read() {
            Ok(opened) => opened,
            Err(FsError::Missing) => {
                return if expected.is_none() {
                    Ok(None)
                } else {
                    Err(FsError::StaleRevision {
                        expected: expected.cloned(),
                        actual: None,
                    })
                };
            }
            Err(error) => return Err(error),
        };
        if opened.stamp.links != 1 {
            return Err(FsError::HardLinked);
        }
        let current = opened.revision(read_limit)?;
        if expected != Some(&current) {
            return Err(FsError::StaleRevision {
                expected: expected.cloned(),
                actual: Some(current),
            });
        }
        Ok(Some(opened))
    }

    fn publish(
        &self,
        content: &[u8],
        previous: Option<&OpenedFile<'_>>,
    ) -> Result<SyncedWrite, WriteError> {
        self.ensure_lock()?;
        let digest = digest(content)?;
        let mut temporary = Temporary::create(self.target)?;
        temporary.fill_and_sync(content)?;
        let identity = temporary.identity()?;
        self.ensure_lock()?;
        if let Some(previous) = previous {
            // Hold the old inode across the final check and rename. This detects
            // observed edits; the lock, not this check, serializes cooperators.
            previous.ensure_unchanged()?;
            temporary.publish_replace(&self.target.name)?;
        } else {
            temporary.publish_new(&self.target.name)?;
        }
        let synchronized = temporary
            .remove_owned()
            .and_then(|()| fsync(&self.target.parent).map_err(FsError::from));
        if let Err(source) = synchronized {
            return Err(WriteError::PublishedButDurabilityUnknown {
                digest,
                bytes: content.len(),
                source,
            });
        }
        Ok(SyncedWrite {
            digest,
            bytes: content.len(),
            identity,
        })
    }
}

fn check_write_budget(content: &[u8], limit: ByteCount) -> Result<(), FsError> {
    if content.len() as u64 > limit.get() {
        return Err(FsError::Budget {
            kind: BudgetKind::Write,
            limit: limit.get(),
        });
    }
    Ok(())
}
