use super::{GitError, objects::Session};
use crate::fs::RelativePath;
use std::fmt::Write;
use std::os::unix::ffi::OsStrExt;
use writfold_domain::{GitObjectFormat, GitObjectId};

pub(super) fn lookup(
    session: &Session<'_, '_>,
    commit: &GitObjectId,
    path: &RelativePath,
) -> Result<GitObjectId, GitError> {
    let bytes = session.object(commit, "commit", session.budgets.stdout)?;
    let first = bytes
        .split(|byte| *byte == b'\n')
        .next()
        .and_then(|line| line.strip_prefix(b"tree "))
        .ok_or(GitError::Malformed("commit root tree"))?;
    let text = std::str::from_utf8(first).map_err(|_| GitError::Malformed("tree identity"))?;
    let mut object = GitObjectId::new(commit.format(), text.to_owned())
        .map_err(|_| GitError::Malformed("tree identity"))?;
    let mut components = path
        .as_path()
        .as_os_str()
        .as_bytes()
        .split(|byte| *byte == b'/')
        .peekable();
    while let Some(component) = components.next() {
        let tree = session.object(&object, "tree", session.budgets.stdout)?;
        let (mode, next) = tree_entry(&tree, component, commit.format())?;
        let terminal = components.peek().is_none();
        if (terminal && !matches!(mode, b"100644" | b"100755")) || (!terminal && mode != b"40000") {
            return Err(GitError::ObjectType);
        }
        object = next;
    }
    // Lookup returns only a checked binding. Reading also checks the blob bytes.
    Ok(object)
}

fn tree_entry<'a>(
    bytes: &'a [u8],
    name: &[u8],
    format: GitObjectFormat,
) -> Result<(&'a [u8], GitObjectId), GitError> {
    let width = match format {
        GitObjectFormat::Sha1 => 20,
        GitObjectFormat::Sha256 => 32,
    };
    let mut remaining = bytes;
    let mut found = None;
    while !remaining.is_empty() {
        let nul = remaining
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(GitError::Malformed("tree record terminator"))?;
        let metadata = &remaining[..nul];
        let separator = metadata
            .iter()
            .position(|byte| *byte == b' ')
            .ok_or(GitError::Malformed("tree record mode"))?;
        let mode = &metadata[..separator];
        let entry_name = &metadata[separator + 1..];
        if entry_name.is_empty()
            || entry_name.contains(&b'/')
            || matches!(entry_name, b"." | b"..")
            || !matches!(
                mode,
                b"40000" | b"100644" | b"100755" | b"120000" | b"160000"
            )
        {
            return Err(GitError::Malformed("tree entry"));
        }
        remaining = &remaining[nul + 1..];
        let raw = remaining
            .get(..width)
            .ok_or(GitError::Malformed("tree object width"))?;
        if entry_name == name {
            if found.is_some() {
                return Err(GitError::Malformed("duplicate tree path"));
            }
            let mut hex = String::with_capacity(width * 2);
            for byte in raw {
                write!(&mut hex, "{byte:02x}").map_err(|_| GitError::Malformed("tree identity"))?;
            }
            let object =
                GitObjectId::new(format, hex).map_err(|_| GitError::Malformed("tree identity"))?;
            found = Some((mode, object));
        }
        remaining = &remaining[width..];
    }
    found.ok_or(GitError::MissingPath)
}
