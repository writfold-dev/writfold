use super::support::*;
use std::fs;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use writfold_domain::{ContentDigest, GitObjectFormat};
use writfold_runtime::fs::{ContainedRoot, RelativePath};
use writfold_runtime::git::{GitBudgets, GitError, GitExecutable, GitRepository};

#[test]
fn profile_refusals_never_execute_repository_helpers() -> Result {
    for extra in [
        "\n[core]\n fsmonitor = ./helper\n",
        "\n[include]\n path = ./extra\n",
        "\n[filter \"x\"]\n process = ./helper\n",
        "\n[extensions]\n partialclone = origin\n",
    ] {
        let fixture = Fixture::new(GitObjectFormat::Sha1)?;
        let helper = fixture.path.join("helper");
        fs::write(&helper, b"#!/bin/sh\ntouch helper-executed\n")?;
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o700))?;
        use std::io::Write;
        fs::OpenOptions::new()
            .append(true)
            .open(fixture.path.join(".git/config"))?
            .write_all(extra.as_bytes())?;
        assert!(fixture.repository(GitObjectFormat::Sha1).is_err());
        assert!(!fixture.path.join("helper-executed").exists());
    }
    for name in [
        "objects/info/alternates",
        "objects/info/http-alternates",
        "commondir",
        "gitdir",
        "shallow",
        "config.worktree",
        "objects/pack/x.promisor",
    ] {
        let fixture = Fixture::new(GitObjectFormat::Sha1)?;
        fs::write(fixture.path.join(".git").join(name), b"/untrusted")?;
        assert!(
            fixture.repository(GitObjectFormat::Sha1).is_err(),
            "admitted {name}"
        );
    }
    Ok(())
}

#[test]
fn executable_and_metadata_budgets_are_not_self_approval() -> Result {
    let file = executable_file()?;
    let wrong: ContentDigest = "0".repeat(64).parse()?;
    assert!(matches!(
        GitExecutable::admit(&file, &wrong, count(16 << 20)?, supervisor()?),
        Err(GitError::ExecutableMismatch)
    ));
    assert!(matches!(
        GitExecutable::admit(&file, &wrong, count(0)?, supervisor()?),
        Err(GitError::Budget { .. })
    ));
    let fixture = Fixture::new(GitObjectFormat::Sha1)?;
    assert!(matches!(
        GitRepository::admit(
            ContainedRoot::open(fixture.path.join(".git"))?,
            GitObjectFormat::Sha1,
            count(0)?
        ),
        Err(GitError::Budget { .. })
    ));
    assert!(fixture.repository(GitObjectFormat::Sha256).is_err());
    Ok(())
}

#[test]
fn blob_stdout_stderr_deadline_and_cancellation_budgets_fail_closed() -> Result {
    let fixture = Fixture::new(GitObjectFormat::Sha1)?;
    fs::write(fixture.path.join("file"), b"not empty")?;
    let commit = fixture.commit()?;
    let blob = fixture.blob(b"not empty")?;
    let repository = fixture.repository(GitObjectFormat::Sha1)?;
    let file = executable_file()?;
    let executable = executable(&file)?;
    let path = RelativePath::new("file")?;
    let zero_stdout = GitBudgets::new(
        count(0)?,
        count(4096)?,
        count(4096)?,
        Duration::from_secs(10),
    );
    assert!(matches!(
        repository.read_blob(&executable, &commit, &path, &blob, zero_stdout, &ACTIVE),
        Err(GitError::Process(ProcessError::Budget { .. }))
    ));
    let zero_blob = GitBudgets::new(
        count(4096)?,
        count(4096)?,
        count(0)?,
        Duration::from_secs(10),
    );
    assert!(matches!(
        repository.read_blob(&executable, &commit, &path, &blob, zero_blob, &ACTIVE),
        Err(GitError::Budget { .. })
    ));
    let zero_time = GitBudgets::new(count(4096)?, count(4096)?, count(4096)?, Duration::ZERO);
    assert!(matches!(
        repository.read_blob(&executable, &commit, &path, &blob, zero_time, &ACTIVE),
        Err(GitError::Process(ProcessError::Deadline))
    ));
    assert!(matches!(
        repository.read_blob(
            &executable,
            &commit,
            &path,
            &blob,
            budgets()?,
            &AtomicBool::new(true)
        ),
        Err(GitError::Process(ProcessError::Cancelled))
    ));
    let missing = "0".repeat(40).parse()?;
    let zero_stderr = GitBudgets::new(
        count(4096)?,
        count(0)?,
        count(4096)?,
        Duration::from_secs(10),
    );
    assert!(matches!(
        repository.lookup_blob(&executable, &missing, &path, zero_stderr, &ACTIVE),
        Err(GitError::Process(ProcessError::Budget {
            stream: "stderr",
            ..
        }))
    ));
    Ok(())
}

#[test]
fn accepted_reads_ignore_attributes_and_never_invoke_installed_hooks() -> Result {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new(GitObjectFormat::Sha1)?;
    fs::write(fixture.path.join("file"), b"literal\r\n\0bytes")?;
    fs::write(
        fixture.path.join(".gitattributes"),
        b"* filter=evil diff=evil text eol=lf\n",
    )?;
    // Index the exact blob directly: fixture creation must not normalize bytes.
    let blob = fixture.blob(b"literal\r\n\0bytes")?;
    fixture.git(
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "100644",
            blob.as_str(),
            "file",
        ],
        &[],
    )?;
    fixture.git(&["commit", "--quiet", "-m", "raw bytes"], &[])?;
    let commit = oid(&fixture.git(&["rev-parse", "HEAD"], &[])?)?;
    fs::create_dir(fixture.path.join(".git/hooks"))?;
    for hook in [
        "pre-auto-gc",
        "post-checkout",
        "post-index-change",
        "fsmonitor-watchman",
    ] {
        let path = fixture.path.join(".git/hooks").join(hook);
        fs::write(&path, b"#!/bin/sh\nprintf ran > helper-executed\n")?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    let repository = fixture.repository(GitObjectFormat::Sha1)?;
    let file = executable_file()?;
    let executable = executable(&file)?;
    let observed = repository.read_blob(
        &executable,
        &commit,
        &RelativePath::new("file")?,
        &blob,
        budgets()?,
        &ACTIVE,
    )?;
    assert_eq!(observed.bytes(), b"literal\r\n\0bytes");
    assert!(!fixture.path.join(".git/helper-executed").exists());
    assert!(!fixture.path.join("helper-executed").exists());
    Ok(())
}
