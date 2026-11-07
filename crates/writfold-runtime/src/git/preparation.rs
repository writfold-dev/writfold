use super::{
    GitBudgets, GitError, GitExecutable, GitWriteError, GitWriterLease, objects::Session,
    traversal, write_objects, writer::relative, writer_profile,
};
use crate::fs::RelativePath;
use std::sync::atomic::AtomicBool;
use writfold_domain::{GitObjectId, PlanDigest};

/// Exact regular blob bindings in addition to the reserved `manifest` blob.
#[derive(Debug)]
pub struct GitManifestEntry {
    pub path: RelativePath,
    pub blob: GitObjectId,
}

/// Borrowed plan material and expected predecessor; not approval authority.
#[derive(Debug)]
pub struct GitPrepareInput<'a> {
    operation: &'a PlanDigest,
    predecessor: Option<&'a GitObjectId>,
    manifest: &'a [u8],
    entries: &'a [GitManifestEntry],
}

impl<'a> GitPrepareInput<'a> {
    pub fn new(
        operation: &'a PlanDigest,
        predecessor: Option<&'a GitObjectId>,
        manifest: &'a [u8],
        entries: &'a [GitManifestEntry],
    ) -> Self {
        Self {
            operation,
            predecessor,
            manifest,
            entries,
        }
    }
}

/// Retained native preparation, never a publication/approval receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitPrepared {
    pub(super) operation: PlanDigest,
    pub(super) commit: GitObjectId,
    pub(super) manifest: GitObjectId,
    pub(super) predecessor: Option<GitObjectId>,
    pub(super) repository: (u64, u64),
}

impl GitPrepared {
    pub fn operation(&self) -> &PlanDigest {
        &self.operation
    }
    pub fn commit(&self) -> &GitObjectId {
        &self.commit
    }
    pub fn manifest(&self) -> &GitObjectId {
        &self.manifest
    }
    pub fn predecessor(&self) -> Option<&GitObjectId> {
        self.predecessor.as_ref()
    }
}

impl GitWriterLease<'_> {
    /// A successful call establishes a permanent blob retention ref. On failure,
    /// any written objects are left intact, including objects preceding Unknown.
    pub fn write_blob(
        &self,
        executable: &GitExecutable<'_>,
        bytes: &[u8],
        budgets: GitBudgets,
        cancelled: &AtomicBool,
    ) -> Result<GitObjectId, GitWriteError> {
        let session = self
            .session(executable, budgets, cancelled)
            .map_err(GitWriteError::Unissued)?;
        check_blob_budget(bytes, budgets).map_err(GitWriteError::Unissued)?;
        let object =
            write_objects::hash(&session, "blob", bytes).map_err(GitWriteError::Unknown)?;
        let retained = (|| {
            let name = blob_ref(&object);
            match writer_profile::resolve(session.repository, &name)? {
                None => write_objects::update(&session, &name, &object, None)?,
                Some(existing) if existing == object => {}
                Some(_) => return Err(GitError::ObjectMismatch),
            }
            self.check()?;
            session.check_stop()?;
            Ok(object)
        })();
        retained.map_err(GitWriteError::Unknown)
    }

    /// Deterministic raw commit (fixed service identity/epoch, one exact parent).
    /// The input binds the caller's retained plan. This primitive neither
    /// interprets manifest authorization nor invents grants.
    pub fn prepare(
        &self,
        executable: &GitExecutable<'_>,
        input: GitPrepareInput<'_>,
        budgets: GitBudgets,
        cancelled: &AtomicBool,
    ) -> Result<GitPrepared, GitWriteError> {
        let GitPrepareInput {
            operation,
            predecessor,
            manifest,
            entries,
        } = input;
        let session = self
            .session(executable, budgets, cancelled)
            .map_err(GitWriteError::Unissued)?;
        let tree = write_objects::Tree::entries(entries).map_err(GitWriteError::Unissued)?;
        check_blob_budget(manifest, budgets).map_err(GitWriteError::Unissued)?;
        if let Some(existing) =
            writer_profile::resolve(session.repository, &prepared_ref(operation))
                .map_err(GitWriteError::Unissued)?
        {
            return Err(GitWriteError::PreparedExists(existing));
        }
        preflight(&session, predecessor, entries).map_err(GitWriteError::Unissued)?;
        let prepared = (|| {
            let manifest = write_objects::hash(&session, "blob", manifest)?;
            let root = tree.write(&session, Some(&manifest))?;
            let bytes = commit_bytes(&root, predecessor, operation);
            let commit = write_objects::hash(&session, "commit", bytes.as_bytes())?;
            if traversal::lookup(&session, &commit, &relative("manifest")?)? != manifest {
                return Err(GitError::ObjectMismatch);
            }
            write_objects::update(&session, &prepared_ref(operation), &commit, None)?;
            self.check()?;
            session.check_stop()?;
            Ok(GitPrepared {
                operation: operation.clone(),
                commit,
                manifest,
                predecessor: predecessor.cloned(),
                repository: self.identity()?,
            })
        })();
        prepared.map_err(GitWriteError::Unknown)
    }
}

fn preflight(
    session: &Session<'_, '_>,
    predecessor: Option<&GitObjectId>,
    entries: &[GitManifestEntry],
) -> Result<(), GitError> {
    if let Some(parent) = predecessor {
        session.object(parent, "commit", session.budgets.stdout)?;
    }
    for entry in entries {
        session.object(&entry.blob, "blob", session.budgets.blob)?;
        if writer_profile::resolve(session.repository, &blob_ref(&entry.blob))?.as_ref()
            != Some(&entry.blob)
        {
            return Err(GitError::UnsupportedProfile("unretained manifest blob"));
        }
    }
    Ok(())
}

fn check_blob_budget(bytes: &[u8], budgets: GitBudgets) -> Result<(), GitError> {
    if bytes.len() as u64 > budgets.blob.get().min(budgets.stdout.get()) {
        return Err(GitError::Budget {
            stream: "blob",
            limit: budgets.blob.get().min(budgets.stdout.get()),
        });
    }
    Ok(())
}

pub(super) fn prepared_ref(operation: &PlanDigest) -> String {
    format!("refs/kctx/prepared/{}", operation.as_str())
}

fn blob_ref(object: &GitObjectId) -> String {
    format!("refs/kctx/objects/{}", object.as_str())
}

pub(super) fn commit_bytes(
    tree: &GitObjectId,
    parent: Option<&GitObjectId>,
    operation: &PlanDigest,
) -> String {
    let parent = parent
        .map(|id| format!("parent {}\n", id.as_str()))
        .unwrap_or_default();
    format!(
        "tree {}\n{parent}author Writfold <writfold@invalid> 0 +0000\ncommitter Writfold <writfold@invalid> 0 +0000\n\nwritfold-operation {}\n",
        tree.as_str(),
        operation.as_str()
    )
}
