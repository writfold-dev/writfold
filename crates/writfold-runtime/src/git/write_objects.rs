use super::preparation::GitManifestEntry;
use super::{GitError, objects::Session};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use writfold_domain::{GitObjectFormat, GitObjectId};

#[derive(Default)]
pub(super) struct Tree<'a> {
    blobs: BTreeMap<Vec<u8>, &'a GitObjectId>,
    trees: BTreeMap<Vec<u8>, Tree<'a>>,
}

impl<'a> Tree<'a> {
    pub(super) fn entries(entries: &'a [GitManifestEntry]) -> Result<Self, GitError> {
        let mut tree = Self::default();
        for entry in entries {
            let parts: Vec<_> = entry
                .path
                .as_path()
                .as_os_str()
                .as_bytes()
                .split(|b| *b == b'/')
                .collect();
            if parts.len() > 64 || parts[0] == b"manifest" {
                return Err(GitError::Malformed("reserved manifest path or tree depth"));
            }
            tree.insert(&parts, &entry.blob)?;
        }
        Ok(tree)
    }

    fn insert(&mut self, parts: &[&[u8]], blob: &'a GitObjectId) -> Result<(), GitError> {
        let name = parts[0];
        if parts.len() == 1 {
            if self.trees.contains_key(name) || self.blobs.insert(name.to_vec(), blob).is_some() {
                return Err(GitError::Malformed(
                    "duplicate or overlapping manifest path",
                ));
            }
        } else {
            if self.blobs.contains_key(name) {
                return Err(GitError::Malformed("overlapping manifest path"));
            }
            self.trees
                .entry(name.to_vec())
                .or_default()
                .insert(&parts[1..], blob)?;
        }
        Ok(())
    }

    pub(super) fn write(
        &self,
        session: &Session<'_, '_>,
        manifest: Option<&GitObjectId>,
    ) -> Result<GitObjectId, GitError> {
        let mut input = Vec::new();
        if let Some(blob) = manifest {
            record(&mut input, b"manifest", blob, false);
        }
        for (name, blob) in &self.blobs {
            record(&mut input, name, blob, false);
        }
        for (name, tree) in &self.trees {
            record(&mut input, name, &tree.write(session, None)?, true);
        }
        if input.len() as u64 > session.budgets.stdout.get() {
            return Err(GitError::Budget {
                stream: "tree",
                limit: session.budgets.stdout.get(),
            });
        }
        let output = command(session, &[OsStr::new("mktree"), OsStr::new("-z")], &input)?;
        let object = parse(session, &output)?;
        session.object(&object, "tree", session.budgets.stdout)?;
        Ok(object)
    }
}

fn record(output: &mut Vec<u8>, name: &[u8], object: &GitObjectId, tree: bool) {
    output.extend_from_slice(if tree {
        b"040000 tree "
    } else {
        b"100644 blob "
    });
    output.extend_from_slice(object.as_str().as_bytes());
    output.push(b'\t');
    output.extend_from_slice(name);
    output.push(0);
}

pub(super) fn hash(
    session: &Session<'_, '_>,
    kind: &str,
    bytes: &[u8],
) -> Result<GitObjectId, GitError> {
    let output = command(
        session,
        &[
            OsStr::new("hash-object"),
            OsStr::new("-w"),
            OsStr::new("-t"),
            OsStr::new(kind),
            OsStr::new("--stdin"),
            OsStr::new("--no-filters"),
        ],
        bytes,
    )?;
    let object = parse(session, &output)?;
    let limit = if kind == "blob" {
        session.budgets.blob
    } else {
        session.budgets.stdout
    };
    if session.object(&object, kind, limit)? != bytes {
        return Err(GitError::ObjectMismatch);
    }
    Ok(object)
}

pub(super) fn update(
    session: &Session<'_, '_>,
    name: &str,
    new: &GitObjectId,
    old: Option<&GitObjectId>,
) -> Result<(), GitError> {
    let zero = match session.repository.format() {
        GitObjectFormat::Sha1 => "0000000000000000000000000000000000000000",
        GitObjectFormat::Sha256 => {
            "0000000000000000000000000000000000000000000000000000000000000000"
        }
    };
    let output = command(
        session,
        &[
            OsStr::new("update-ref"),
            OsStr::new("--no-deref"),
            OsStr::new(name),
            OsStr::new(new.as_str()),
            OsStr::new(old.map_or(zero, GitObjectId::as_str)),
        ],
        &[],
    )?;
    if !output.is_empty() {
        return Err(GitError::Malformed("update-ref output"));
    }
    Ok(())
}

fn parse(session: &Session<'_, '_>, output: &[u8]) -> Result<GitObjectId, GitError> {
    let text = output
        .strip_suffix(b"\n")
        .and_then(|b| std::str::from_utf8(b).ok())
        .ok_or(GitError::Malformed("written object identity"))?;
    GitObjectId::new(session.repository.format(), text.to_owned())
        .map_err(|_| GitError::Malformed("written object identity"))
}

fn command(session: &Session<'_, '_>, args: &[&OsStr], input: &[u8]) -> Result<Vec<u8>, GitError> {
    super::writer_profile::check(session.repository)?;
    let output = session.command(args, input, session.budgets)?;
    super::writer_profile::check(session.repository)?;
    session.check_stop()?;
    Ok(output)
}
