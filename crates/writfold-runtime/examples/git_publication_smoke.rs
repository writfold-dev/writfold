//! Run with the externally maintained Git ELF SHA-256 profile digest as the
//! sole argument. Uses a fresh disposable repository and retains failed attempts.
use std::error::Error;
use std::fs;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use writfold_domain::{ByteCount, ContentDigest};
use writfold_runtime::fs::{ContainedRoot, RelativePath};
use writfold_runtime::git::{
    GitBudgets, GitExecutable, GitManifestEntry, GitPrepareInput, GitWriteError, GitWriter,
};
use writfold_runtime::process::ProcessSupervisor;

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn count(value: u64) -> Result<ByteCount> {
    Ok(ByteCount::new(value)?)
}
fn budgets() -> Result<GitBudgets> {
    Ok(GitBudgets::new(
        count(1 << 20)?,
        count(65536)?,
        count(1 << 20)?,
        Duration::from_secs(20),
    ))
}

fn main() -> Result {
    let digest: ContentDigest = std::env::args()
        .nth(1)
        .ok_or("provide the externally maintained /usr/bin/git ELF SHA-256 profile digest")?
        .parse()?;
    let supervisor = ProcessSupervisor::enable()?;
    let file = ContainedRoot::open("/usr/bin")?.pin(&RelativePath::new("git")?)?;
    let executable = GitExecutable::admit(&file, &digest, count(16 << 20)?, &supervisor)?;
    let path = std::env::temp_dir().join(format!(
        "writfold-publication-smoke-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    eprintln!(
        "disposable native repository (retained on failure): {}",
        path.display()
    );
    let active = AtomicBool::new(false);
    let writer = GitWriter::initialize(&path, &executable, count(1 << 20)?, budgets()?, &active)?;
    exercise(&writer, &executable, &active)?;
    drop(writer);
    fs::remove_dir_all(path)?;
    println!(
        "observed native preparation, exact retained reads, expected-old CAS and stale refusal; not alpha/power-loss qualification"
    );
    Ok(())
}

fn exercise(writer: &GitWriter, executable: &GitExecutable<'_>, active: &AtomicBool) -> Result {
    let lease = writer.try_lease()?;
    let blob = lease.write_blob(executable, b"retained payload\0", budgets()?, active)?;
    let entries = [GitManifestEntry {
        path: RelativePath::new("retained/payload")?,
        blob: blob.clone(),
    }];
    let first = lease.prepare(
        executable,
        GitPrepareInput::new(&"1".repeat(64).parse()?, None, b"manifest one", &entries),
        budgets()?,
        active,
    )?;
    assert!(lease.completed(executable, budgets()?, active)?.is_none());
    let manifest = writer.repository().read_blob(
        executable,
        first.commit(),
        &RelativePath::new("manifest")?,
        first.manifest(),
        budgets()?,
        active,
    )?;
    assert_eq!(manifest.bytes(), b"manifest one");
    let payload = writer.repository().read_blob(
        executable,
        first.commit(),
        &entries[0].path,
        &blob,
        budgets()?,
        active,
    )?;
    assert_eq!(payload.bytes(), b"retained payload\0");
    lease.publish(executable, &first, budgets()?, active)?;
    let next = lease.prepare(
        executable,
        GitPrepareInput::new(&"2".repeat(64).parse()?, Some(first.commit()), b"next", &[]),
        budgets()?,
        active,
    )?;
    let rival = lease.prepare(
        executable,
        GitPrepareInput::new(
            &"3".repeat(64).parse()?,
            Some(first.commit()),
            b"rival",
            &[],
        ),
        budgets()?,
        active,
    )?;
    lease.publish(executable, &next, budgets()?, active)?;
    assert!(matches!(
        lease.publish(executable, &rival, budgets()?, active),
        Err(GitWriteError::Conflict { .. })
    ));
    assert_eq!(
        lease.prepared(executable, rival.operation(), budgets()?, active)?,
        Some(rival)
    );
    assert_eq!(
        lease.history(executable, next.commit(), budgets()?, active)?,
        next
    );
    let zero = GitBudgets::new(
        count(0)?,
        count(65536)?,
        count(1024)?,
        Duration::from_secs(20),
    );
    assert!(matches!(
        lease.write_blob(executable, b"", zero, active),
        Err(GitWriteError::Unknown(_))
    ));
    Ok(())
}
