use super::DoltError;
use crate::fs::{ContainedRoot, FsError, RelativePath};
use rustix::io::{Errno, pread};
use writfold_domain::ByteCount;

/// Dolt creates an empty global object even for SELECT-only use. Admit that
/// semantic state, including JSON whitespace, but no settings or credentials.
/// Stream bounded bytes; never allocate or expose configuration contents.
pub(super) fn empty_global(root: &ContainedRoot, limit: ByteCount) -> Result<(), DoltError> {
    let target = root.pin(&RelativePath::new("config_global.json").map_err(FsError::from)?)?;
    let opened = match target.open_read() {
        Ok(opened) => opened,
        Err(FsError::Missing) => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if opened.size() > limit.get() {
        return Err(DoltError::Budget {
            stream: "global configuration",
            limit: limit.get(),
        });
    }
    let mut buffer = [0_u8; 512];
    let mut offset = 0;
    let mut state = 0;
    while offset < opened.size() {
        let count = (opened.size() - offset).min(buffer.len() as u64) as usize;
        let read = match pread(opened.descriptor(), &mut buffer[..count], offset) {
            Err(Errno::INTR) => continue,
            result => result.map_err(FsError::from)?,
        };
        if read == 0 {
            return Err(FsError::ChangedSource.into());
        }
        for &byte in &buffer[..read] {
            if matches!(byte, b' ' | b'\t' | b'\r' | b'\n') {
                continue;
            }
            state = match (state, byte) {
                (0, b'{') => 1,
                (1, b'}') => 2,
                _ => {
                    return Err(DoltError::UnsupportedProfile(
                        "global configuration must be an empty JSON object",
                    ));
                }
            };
        }
        offset += read as u64;
    }
    opened.check_unchanged()?;
    if state != 2 {
        return Err(DoltError::UnsupportedProfile(
            "global configuration must be an empty JSON object",
        ));
    }
    Ok(())
}
