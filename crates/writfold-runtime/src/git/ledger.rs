//! Ledger record retention and conditional head selection in the writer
//! repository (`writfold-local-ledger/1`).
//!
//! Each record is one commit whose tree holds exactly one blob, `record`, with
//! the caller's canonical record bytes; its single parent is the predecessor
//! record's commit. A stream head is the loose ref
//! `refs/writfold/ledger/<hex stream>`, moved only by Git's atomic expected-old
//! update. Record meaning, history validation and authorization belong to the
//! caller: this module retains and selects bytes, nothing more.
use super::{
    GitBudgets, GitError, GitExecutable, GitWriteError, GitWriterLease, objects::Session,
    write_objects, writer_profile,
};
use std::sync::atomic::AtomicBool;
use writfold_domain::{GitObjectId, StreamId};

const RECORD: &str = "record";
const MESSAGE: &str = "writfold-ledger-record/1\n";

/// One retained record commit, as read back from native objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerCommit {
    pub commit: GitObjectId,
    pub parent: Option<GitObjectId>,
    pub record: Vec<u8>,
}

pub fn ledger_ref(stream: &StreamId) -> String {
    let hex: String = stream
        .as_str()
        .bytes()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("refs/writfold/ledger/{hex}")
}

impl GitWriterLease<'_> {
    /// The stream's current head commit, or `None` for a stream never selected.
    pub fn ledger_head(
        &self,
        executable: &GitExecutable<'_>,
        stream: &StreamId,
        budgets: GitBudgets,
        cancelled: &AtomicBool,
    ) -> Result<Option<GitObjectId>, GitError> {
        let session = self.session(executable, budgets, cancelled)?;
        let head = writer_profile::resolve(session.repository, &ledger_ref(stream))?;
        if let Some(head) = &head {
            session.object(head, "commit", budgets.stdout)?;
        }
        self.check()?;
        Ok(head)
    }

    /// Read one record commit and verify it has exactly this profile's shape.
    pub fn ledger_commit(
        &self,
        executable: &GitExecutable<'_>,
        commit: &GitObjectId,
        budgets: GitBudgets,
        cancelled: &AtomicBool,
    ) -> Result<LedgerCommit, GitError> {
        let session = self.session(executable, budgets, cancelled)?;
        let read = read_commit(&session, commit)?;
        self.check()?;
        Ok(read)
    }

    /// Retain `record` as a new commit on `expected` and move the stream head
    /// from `expected` to it in one atomic expected-old update. A head that is
    /// not `expected` refuses before anything is written to the ref. Any error
    /// after the update was issued is Unknown: reconcile, never blindly retry.
    pub fn ledger_select(
        &self,
        executable: &GitExecutable<'_>,
        stream: &StreamId,
        expected: Option<&GitObjectId>,
        record: &[u8],
        budgets: GitBudgets,
        cancelled: &AtomicBool,
    ) -> Result<GitObjectId, GitWriteError> {
        let session = self
            .session(executable, budgets, cancelled)
            .map_err(GitWriteError::Unissued)?;
        let name = ledger_ref(stream);
        let actual =
            writer_profile::resolve(session.repository, &name).map_err(GitWriteError::Unissued)?;
        if actual.as_ref() != expected {
            return Err(GitWriteError::Conflict { actual });
        }
        let commit = write_commit(&session, expected, record).map_err(GitWriteError::Unissued)?;
        write_objects::update(&session, &name, &commit, expected)
            .map_err(GitWriteError::Unknown)?;
        self.check().map_err(GitWriteError::Unknown)?;
        let observed =
            writer_profile::resolve(session.repository, &name).map_err(GitWriteError::Unknown)?;
        if observed.as_ref() != Some(&commit) {
            return Err(GitWriteError::Unknown(GitError::ObjectMismatch));
        }
        Ok(commit)
    }
}

fn write_commit(
    session: &Session<'_, '_>,
    parent: Option<&GitObjectId>,
    record: &[u8],
) -> Result<GitObjectId, GitError> {
    if record.len() as u64 > session.budgets.blob.get() {
        return Err(GitError::Budget {
            stream: "ledger record",
            limit: session.budgets.blob.get(),
        });
    }
    let blob = write_objects::hash(session, "blob", record)?;
    let tree = write_objects::single_blob_tree(session, RECORD, &blob)?;
    write_objects::hash(session, "commit", commit_text(&tree, parent).as_bytes())
}

fn commit_text(tree: &GitObjectId, parent: Option<&GitObjectId>) -> String {
    let parent = parent
        .map(|id| format!("parent {}\n", id.as_str()))
        .unwrap_or_default();
    format!(
        "tree {}\n{parent}author Writfold <writfold@invalid> 0 +0000\ncommitter Writfold <writfold@invalid> 0 +0000\n\n{MESSAGE}",
        tree.as_str()
    )
}

/// Parse only the exact bytes `write_commit` produces; anything else refuses.
fn read_commit(session: &Session<'_, '_>, commit: &GitObjectId) -> Result<LedgerCommit, GitError> {
    let bytes = session.object(commit, "commit", session.budgets.stdout)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| GitError::Malformed("ledger commit"))?;
    let rest = text
        .strip_prefix("tree ")
        .ok_or(GitError::Malformed("ledger commit tree"))?;
    let (tree, rest) = rest
        .split_once('\n')
        .ok_or(GitError::Malformed("ledger commit tree"))?;
    let tree = GitObjectId::new(session.repository.format(), tree.to_owned())
        .map_err(|_| GitError::Malformed("ledger commit tree"))?;
    let parent = match rest.strip_prefix("parent ") {
        Some(rest) => {
            let (id, _) = rest
                .split_once('\n')
                .ok_or(GitError::Malformed("ledger commit parent"))?;
            Some(
                GitObjectId::new(session.repository.format(), id.to_owned())
                    .map_err(|_| GitError::Malformed("ledger commit parent"))?,
            )
        }
        None => None,
    };
    if commit_text(&tree, parent.as_ref()) != text {
        return Err(GitError::Malformed("ledger commit profile"));
    }
    let blob = write_objects::single_blob_tree_entry(session, &tree, RECORD)?;
    let record = session.object(&blob, "blob", session.budgets.blob)?;
    Ok(LedgerCommit {
        commit: commit.clone(),
        parent,
        record,
    })
}
