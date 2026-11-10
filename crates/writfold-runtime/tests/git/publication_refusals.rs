use super::publication::{Repository, operation};
use super::support::{ACTIVE, Result, budgets, count, executable, executable_file};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use writfold_domain::GitObjectFormat;
use writfold_runtime::fs::ContainedRoot;
use writfold_runtime::git::{GitError, GitPrepareInput, GitWriteError, GitWriter};

#[test]
fn malformed_symbolic_and_wrong_algorithm_completed_refs_refuse() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    let prepared = lease.prepare(
        &executable,
        GitPrepareInput::new(&operation('9')?, None, b"m", &[]),
        budgets()?,
        &ACTIVE,
    )?;
    let completed = fixture.path.join("refs/kctx/knowledge");
    for malformed in [
        b"ref: refs/heads/other\n".as_slice(),
        b"not-a-hash\n",
        b"0000000000000000000000000000000000000000\n",
        b"abc",
    ] {
        fs::write(&completed, malformed)?;
        assert!(matches!(
            lease.completed(&executable, budgets()?, &ACTIVE),
            Err(GitError::Malformed(_))
        ));
        assert!(matches!(
            lease.publish(&executable, &prepared, budgets()?, &ACTIVE),
            Err(GitWriteError::Unissued(GitError::Malformed(_)))
        ));
    }
    let blob = lease.write_blob(&executable, b"not a commit", budgets()?, &ACTIVE)?;
    fs::write(&completed, format!("{}\n", blob.as_str()))?;
    assert!(matches!(
        lease.completed(&executable, budgets()?, &ACTIVE),
        Err(GitError::ObjectType)
    ));
    assert!(matches!(
        lease.publish(&executable, &prepared, budgets()?, &ACTIVE),
        Err(GitWriteError::Unissued(GitError::ObjectType))
    ));
    drop(lease);
    fixture.cleanup()
}

#[test]
fn symlinked_ref_or_object_directory_never_reaches_native_writer() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    let prepared = lease.prepare(
        &executable,
        GitPrepareInput::new(&operation('a')?, None, b"m", &[]),
        budgets()?,
        &ACTIVE,
    )?;
    let completed = fixture.path.join("refs/kctx/knowledge");
    symlink("../../HEAD", &completed)?;
    assert!(matches!(
        lease.publish(&executable, &prepared, budgets()?, &ACTIVE),
        Err(GitWriteError::Unissued(_))
    ));
    assert!(fixture.reopen().is_err());
    fs::remove_file(completed)?;
    drop(lease);
    fs::rename(
        fixture.path.join("objects"),
        fixture.path.join("objects-retained"),
    )?;
    symlink("objects-retained", fixture.path.join("objects"))?;
    assert!(fixture.reopen().is_err());
    fixture.cleanup()
}

#[test]
fn private_root_config_and_stable_coordination_are_revalidated() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    fs::set_permissions(&fixture.path, fs::Permissions::from_mode(0o755))?;
    assert!(fixture.reopen().is_err());
    assert!(matches!(
        lease.write_blob(&executable, b"no", budgets()?, &ACTIVE),
        Err(GitWriteError::Unissued(_))
    ));
    fs::set_permissions(&fixture.path, fs::Permissions::from_mode(0o700))?;
    let config = fixture.path.join("config");
    let original = fs::read(&config)?;
    let mut unsupported = original.clone();
    unsupported.extend_from_slice(b"\n[core]\n hooksPath = /tmp/foreign\n");
    fs::write(&config, unsupported)?;
    assert!(fixture.reopen().is_err());
    fs::write(config, original)?;
    fs::rename(
        fixture.path.join("writfold-locks"),
        fixture.path.join("old-locks"),
    )?;
    fs::create_dir(fixture.path.join("writfold-locks"))?;
    fs::set_permissions(
        fixture.path.join("writfold-locks"),
        fs::Permissions::from_mode(0o700),
    )?;
    assert!(matches!(
        lease.write_blob(&executable, b"no", budgets()?, &ACTIVE),
        Err(GitWriteError::Unissued(_))
    ));
    drop(lease);
    fixture.cleanup()
}

#[test]
fn initialization_is_explicit_and_existing_repository_is_not_reinitialized() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let config = fs::read(fixture.path.join("config"))?;
    assert!(
        GitWriter::initialize(
            &fixture.path,
            &executable,
            count(1 << 20)?,
            budgets()?,
            &ACTIVE
        )
        .is_err()
    );
    assert_eq!(fs::read(fixture.path.join("config"))?, config);
    fs::remove_dir_all(fixture.path.join("writfold-locks"))?;
    assert!(
        GitWriter::open(
            ContainedRoot::open(&fixture.path)?,
            GitObjectFormat::Sha256,
            count(1 << 20)?
        )
        .is_err()
    );
    assert!(!fixture.path.join("writfold-locks").exists());
    fixture.cleanup()
}

/// A killed `update-ref` leaves `<ref>.lock`; publication then fails until the
/// lease holder clears the abandoned lock, which changes no reference.
#[test]
fn abandoned_reference_locks_block_publication_until_cleared() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    let prepared = lease.prepare(
        &executable,
        GitPrepareInput::new(&operation('8')?, None, b"m", &[]),
        budgets()?,
        &ACTIVE,
    )?;
    fs::create_dir_all(fixture.path.join("refs/kctx"))?;
    fs::write(fixture.path.join("refs/kctx/knowledge.lock"), b"partial")?;
    assert!(matches!(
        lease.publish(&executable, &prepared, budgets()?, &ACTIVE),
        Err(GitWriteError::Unknown(_))
    ));
    assert_eq!(lease.completed(&executable, budgets()?, &ACTIVE)?, None);
    assert_eq!(lease.clear_stale_ref_locks()?, 1);
    assert_eq!(lease.clear_stale_ref_locks()?, 0);
    assert_eq!(lease.completed(&executable, budgets()?, &ACTIVE)?, None);
    lease.publish(&executable, &prepared, budgets()?, &ACTIVE)?;
    assert_eq!(
        lease.completed(&executable, budgets()?, &ACTIVE)?.as_ref(),
        Some(prepared.commit())
    );
    drop(lease);
    fixture.cleanup()
}
