use super::PathError;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// A private, validated relative OS path, not a UTF-8 public identity.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RelativePath(PathBuf);

impl RelativePath {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, PathError> {
        let path = path.as_ref();
        let bytes = path.as_os_str().as_bytes();
        if bytes.starts_with(b"/") {
            return Err(PathError::Absolute);
        }
        if bytes.contains(&0) {
            return Err(PathError::Nul);
        }
        for component in bytes.split(|byte| *byte == b'/') {
            match component {
                b"" => return Err(PathError::EmptyComponent),
                b"." | b".." => return Err(PathError::DotComponent),
                _ => {}
            }
        }
        Ok(Self(path.to_owned()))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}
