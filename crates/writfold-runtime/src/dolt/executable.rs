use super::DoltError;
use crate::fs::{OpenedFile, PinnedFile};
use crate::process::ProcessSupervisor;
use rustix::fd::{AsRawFd, OwnedFd};
use std::path::PathBuf;
use writfold_domain::{ByteCount, DoltToolGeneration};

/// Exact native ELF identity from an external qualification profile, not proof
/// that an arbitrary executable chosen by a caller is an authorized tool.
#[derive(Debug)]
pub struct DoltExecutable<'a> {
    opened: OpenedFile<'a>,
    generation: DoltToolGeneration,
    supervisor: &'a ProcessSupervisor,
}

impl<'a> DoltExecutable<'a> {
    pub fn admit(
        file: &'a PinnedFile,
        expected: &DoltToolGeneration,
        byte_limit: ByteCount,
        supervisor: &'a ProcessSupervisor,
    ) -> Result<Self, DoltError> {
        supervisor.ensure_current()?;
        let opened = file.open_read()?;
        opened.check_unchanged()?;
        if opened.size() > byte_limit.get() {
            return Err(DoltError::Budget {
                stream: "executable",
                limit: byte_limit.get(),
            });
        }
        if opened.revision(byte_limit)?.as_str() != expected.as_str() {
            return Err(DoltError::ExecutableMismatch);
        }
        let mut header = [0_u8; 4];
        if opened.read_prefix(&mut header)? != header.len() || header != *b"\x7fELF" {
            return Err(DoltError::UnsupportedProfile(
                "ELF native executable required",
            ));
        }
        Ok(Self {
            opened,
            generation: expected.clone(),
            supervisor,
        })
    }

    pub fn generation(&self) -> &DoltToolGeneration {
        &self.generation
    }

    pub(super) fn path(&self) -> Result<PathBuf, DoltError> {
        self.opened.check_unchanged()?;
        Ok(descriptor_path(self.opened.descriptor()))
    }

    pub(super) fn supervisor(&self) -> &ProcessSupervisor {
        self.supervisor
    }
}

pub(super) fn descriptor_path(fd: &OwnedFd) -> PathBuf {
    PathBuf::from(format!(
        "/proc/{}/fd/{}",
        std::process::id(),
        fd.as_raw_fd()
    ))
}
