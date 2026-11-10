//! Stale reference-lock reconciliation for the exclusive cooperating writer.
//!
//! Native `git update-ref` writes `<ref>.lock` and renames it over `<ref>`. A
//! process killed between the two leaves the lock behind, and every later
//! update of that ref then fails with "File exists". Under this writer's
//! exclusive lease no cooperating writer can hold a reference lock, and the
//! supervisor has reaped every process group it started before returning, so a
//! remaining `refs/**/*.lock` is abandoned: its update never happened (the ref
//! still holds its old value) and can never happen. Removing it changes no ref.
use super::{GitError, GitWriterLease, writer::relative, writer_profile};
use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, Dir, FileType, Mode, OFlags, fstat, fsync, openat, unlinkat};

const DEPTH: usize = 64;

impl GitWriterLease<'_> {
    /// Remove abandoned reference locks under `refs/`; returns how many. Call
    /// only while reconciling an outcome this lease's own killed process left.
    pub fn clear_stale_ref_locks(&self) -> Result<u32, GitError> {
        self.check()?;
        let refs = self.writer.repository.root.open_child(&relative("refs")?)?;
        let mut removed = 0;
        clear(refs.descriptor(), 0, &mut removed)?;
        writer_profile::check(&self.writer.repository)?;
        self.check()?;
        Ok(removed)
    }
}

fn clear(directory: &OwnedFd, depth: usize, removed: &mut u32) -> Result<(), GitError> {
    if depth > DEPTH {
        return Err(GitError::UnsupportedProfile("reference depth"));
    }
    let mut stale = Vec::new();
    for entry in Dir::read_from(directory)? {
        let entry = entry?;
        let name = entry.file_name().to_owned();
        if matches!(name.to_bytes(), b"." | b"..") {
            continue;
        }
        let file = openat(
            directory,
            name.as_c_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )?;
        match FileType::from_raw_mode(fstat(&file)?.st_mode) {
            FileType::Directory => clear(&file, depth + 1, removed)?,
            FileType::RegularFile if name.to_bytes().ends_with(b".lock") => stale.push(name),
            FileType::RegularFile => {}
            _ => return Err(GitError::UnsupportedProfile("nonregular reference")),
        }
    }
    for name in &stale {
        unlinkat(directory, name.as_c_str(), AtFlags::empty())?;
        *removed += 1;
    }
    if !stale.is_empty() {
        fsync(directory)?;
    }
    Ok(())
}
