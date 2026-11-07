use super::support::{self, ACTIVE, Result, budgets, count, executable, executable_file};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use writfold_domain::{GitObjectFormat, PlanDigest};
use writfold_runtime::fs::{ContainedRoot, RelativePath};
use writfold_runtime::git::{
    GitBudgets, GitError, GitExecutable, GitManifestEntry, GitPrepareInput, GitWriteError,
    GitWriter,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) struct Repository {
    pub path: PathBuf,
    pub writer: GitWriter,
}

impl Repository {
    pub fn new(executable: &GitExecutable<'_>) -> Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "writfold-publication-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let writer =
            GitWriter::initialize(&path, executable, count(1 << 20)?, budgets()?, &ACTIVE)?;
        Ok(Self { path, writer })
    }

    pub fn reopen(&self) -> Result<GitWriter> {
        Ok(GitWriter::open(
            ContainedRoot::open(&self.path)?,
            GitObjectFormat::Sha256,
            count(1 << 20)?,
        )?)
    }

    // Explicit successful cleanup only. Failed attempts remain for diagnosis.
    pub fn cleanup(self) -> Result {
        drop(self.writer);
        fs::remove_dir_all(self.path)?;
        Ok(())
    }
}

pub(super) fn operation(byte: char) -> Result<PlanDigest> {
    Ok(byte.to_string().repeat(64).parse()?)
}

#[test]
fn publishes_exact_retained_manifest_and_blobs() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    let blob = lease.write_blob(&executable, b"payload\0bytes", budgets()?, &ACTIVE)?;
    let entries = [GitManifestEntry {
        path: RelativePath::new("retained/data")?,
        blob: blob.clone(),
    }];
    let first = lease.prepare(
        &executable,
        GitPrepareInput::new(&operation('1')?, None, b"exact manifest\0", &entries),
        budgets()?,
        &ACTIVE,
    )?;
    assert!(lease.completed(&executable, budgets()?, &ACTIVE)?.is_none());
    assert_eq!(
        lease
            .prepared(&executable, first.operation(), budgets()?, &ACTIVE)?
            .as_ref(),
        Some(&first)
    );
    let manifest = fixture.writer.repository().read_blob(
        &executable,
        first.commit(),
        &RelativePath::new("manifest")?,
        first.manifest(),
        budgets()?,
        &ACTIVE,
    )?;
    assert_eq!(manifest.bytes(), b"exact manifest\0");
    let payload = fixture.writer.repository().read_blob(
        &executable,
        first.commit(),
        &entries[0].path,
        &blob,
        budgets()?,
        &ACTIVE,
    )?;
    assert_eq!(payload.bytes(), b"payload\0bytes");
    lease.publish(&executable, &first, budgets()?, &ACTIVE)?;
    assert_eq!(
        lease.completed(&executable, budgets()?, &ACTIVE)?.as_ref(),
        Some(first.commit())
    );
    drop(lease);
    fixture.cleanup()
}

#[test]
fn prepared_rivals_cannot_overwrite_completed_successor() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    let first = lease.prepare(
        &executable,
        GitPrepareInput::new(&operation('1')?, None, b"first", &[]),
        budgets()?,
        &ACTIVE,
    )?;
    lease.publish(&executable, &first, budgets()?, &ACTIVE)?;
    let second = lease.prepare(
        &executable,
        GitPrepareInput::new(&operation('2')?, Some(first.commit()), b"second", &[]),
        budgets()?,
        &ACTIVE,
    )?;
    let rival = lease.prepare(
        &executable,
        GitPrepareInput::new(&operation('3')?, Some(first.commit()), b"rival", &[]),
        budgets()?,
        &ACTIVE,
    )?;
    lease.publish(&executable, &second, budgets()?, &ACTIVE)?;
    assert!(
        matches!(lease.publish(&executable, &rival, budgets()?, &ACTIVE),
        Err(GitWriteError::Conflict { actual: Some(head) }) if &head == second.commit())
    );
    assert_eq!(
        lease.history(&executable, second.commit(), budgets()?, &ACTIVE)?,
        second
    );
    assert_eq!(
        lease.prepared(&executable, rival.operation(), budgets()?, &ACTIVE)?,
        Some(rival)
    );
    assert_eq!(
        lease.completed(&executable, budgets()?, &ACTIVE)?.as_ref(),
        Some(second.commit())
    );
    drop(lease);
    fixture.cleanup()
}

#[test]
fn competing_writer_leases_and_stale_initial_proposal_refuse() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let other = fixture.reopen()?;
    let lease = fixture.writer.try_lease()?;
    assert!(matches!(
        other.try_lease(),
        Err(GitError::Filesystem(
            writfold_runtime::fs::FsError::LockBusy
        ))
    ));
    let first = lease.prepare(
        &executable,
        GitPrepareInput::new(&operation('4')?, None, b"first", &[]),
        budgets()?,
        &ACTIVE,
    )?;
    let rival = lease.prepare(
        &executable,
        GitPrepareInput::new(&operation('5')?, None, b"rival", &[]),
        budgets()?,
        &ACTIVE,
    )?;
    lease.publish(&executable, &first, budgets()?, &ACTIVE)?;
    drop(lease);
    let lease = other.try_lease()?;
    assert!(matches!(
        lease.publish(&executable, &rival, budgets()?, &ACTIVE),
        Err(GitWriteError::Conflict { .. })
    ));
    assert_eq!(
        lease.completed(&executable, budgets()?, &ACTIVE)?.as_ref(),
        Some(first.commit())
    );
    drop(lease);
    drop(other);
    fixture.cleanup()
}

#[test]
fn wrong_native_kinds_algorithms_and_duplicate_preparation_refuse() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    let blob = lease.write_blob(&executable, b"blob", budgets()?, &ACTIVE)?;
    assert!(matches!(
        lease.prepare(
            &executable,
            GitPrepareInput::new(&operation('6')?, Some(&blob), b"m", &[]),
            budgets()?,
            &ACTIVE
        ),
        Err(GitWriteError::Unissued(GitError::ObjectType))
    ));
    let sha1 = "a".repeat(40).parse()?;
    assert!(matches!(
        lease.prepare(
            &executable,
            GitPrepareInput::new(&operation('6')?, Some(&sha1), b"m", &[]),
            budgets()?,
            &ACTIVE
        ),
        Err(GitWriteError::Unissued(GitError::ObjectFormat))
    ));
    let prepared = lease.prepare(
        &executable,
        GitPrepareInput::new(&operation('6')?, None, b"m", &[]),
        budgets()?,
        &ACTIVE,
    )?;
    assert!(matches!(
        lease.prepare(
            &executable,
            GitPrepareInput::new(prepared.operation(), None, b"m", &[]),
            budgets()?,
            &ACTIVE
        ),
        Err(GitWriteError::PreparedExists(_))
    ));
    let entry = [GitManifestEntry {
        path: RelativePath::new("bad")?,
        blob: prepared.commit().clone(),
    }];
    assert!(matches!(
        lease.prepare(
            &executable,
            GitPrepareInput::new(&operation('7')?, None, b"m", &entry),
            budgets()?,
            &ACTIVE
        ),
        Err(GitWriteError::Unissued(GitError::ObjectType))
    ));
    drop(lease);
    fixture.cleanup()
}

#[test]
fn foreign_preparation_cannot_publish_into_another_repository() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let foreign = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    let prepared = lease.prepare(
        &executable,
        GitPrepareInput::new(
            &operation('8')?,
            None,
            b"retained in original repository",
            &[],
        ),
        budgets()?,
        &ACTIVE,
    )?;
    let foreign_lease = foreign.writer.try_lease()?;
    assert!(matches!(
        foreign_lease.publish(&executable, &prepared, budgets()?, &ACTIVE),
        Err(GitWriteError::Unissued(GitError::UnsupportedProfile(_)))
    ));
    assert!(
        foreign_lease
            .completed(&executable, budgets()?, &ACTIVE)?
            .is_none()
    );
    drop(foreign_lease);
    drop(lease);
    foreign.cleanup()?;
    fixture.cleanup()
}

#[test]
fn owned_process_failures_remain_unknown_and_pre_cancel_is_unissued() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    let zero_output = GitBudgets::new(
        count(0)?,
        count(65536)?,
        count(1024)?,
        Duration::from_secs(20),
    );
    assert!(matches!(
        lease.write_blob(&executable, b"", zero_output, &ACTIVE),
        Err(GitWriteError::Unknown(GitError::Process(
            support::ProcessError::Budget { .. }
        )))
    ));
    let cancelled = AtomicBool::new(true);
    assert!(matches!(
        lease.write_blob(&executable, b"never issued", budgets()?, &cancelled),
        Err(GitWriteError::Unissued(GitError::Process(
            support::ProcessError::Cancelled
        )))
    ));
    let prepared = lease.prepare(
        &executable,
        GitPrepareInput::new(&operation('8')?, None, b"m", &[]),
        budgets()?,
        &ACTIVE,
    )?;
    fs::write(
        fixture.path.join("refs/kctx/knowledge.lock"),
        b"retained native lock",
    )?;
    assert!(matches!(
        lease.publish(&executable, &prepared, budgets()?, &ACTIVE),
        Err(GitWriteError::Unknown(GitError::Process(
            support::ProcessError::Command { .. }
        )))
    ));
    assert_eq!(
        lease.prepared(&executable, prepared.operation(), budgets()?, &ACTIVE)?,
        Some(prepared)
    );
    drop(lease);
    fixture.cleanup()
}
