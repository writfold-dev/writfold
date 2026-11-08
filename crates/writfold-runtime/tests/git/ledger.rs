use super::publication::Repository;
use super::support::{ACTIVE, Result, budgets, executable, executable_file};
use writfold_domain::StreamId;
use writfold_runtime::git::{GitWriteError, ledger_ref};

fn stream(name: &str) -> Result<StreamId> {
    Ok(name.parse()?)
}

#[test]
fn selects_records_by_expected_head_and_reads_them_back() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    let s = stream("run.1")?;
    assert_eq!(
        lease.ledger_head(&executable, &s, budgets()?, &ACTIVE)?,
        None
    );
    let first =
        lease.ledger_select(&executable, &s, None, b"first\0record", budgets()?, &ACTIVE)?;
    let second = lease.ledger_select(
        &executable,
        &s,
        Some(&first),
        b"second",
        budgets()?,
        &ACTIVE,
    )?;
    assert_eq!(
        lease.ledger_head(&executable, &s, budgets()?, &ACTIVE)?,
        Some(second.clone())
    );
    let read = lease.ledger_commit(&executable, &second, budgets()?, &ACTIVE)?;
    assert_eq!(
        (read.parent, read.record),
        (Some(first.clone()), b"second".to_vec())
    );
    let read = lease.ledger_commit(&executable, &first, budgets()?, &ACTIVE)?;
    assert_eq!(
        (read.parent, read.record),
        (None, b"first\0record".to_vec())
    );
    // Streams are independent heads.
    let other = stream("run.2")?;
    assert_eq!(
        lease.ledger_head(&executable, &other, budgets()?, &ACTIVE)?,
        None
    );
    drop(lease);
    // The selection survives reopening the writer.
    let reopened = fixture.reopen()?;
    let lease = reopened.try_lease()?;
    assert_eq!(
        lease.ledger_head(&executable, &s, budgets()?, &ACTIVE)?,
        Some(second)
    );
    drop(lease);
    drop(reopened);
    fixture.cleanup()
}

#[test]
fn stale_or_missing_expected_head_conflicts_without_moving_the_head() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    let s = stream("run.1")?;
    let first = lease.ledger_select(&executable, &s, None, b"one", budgets()?, &ACTIVE)?;
    let second = lease.ledger_select(&executable, &s, Some(&first), b"two", budgets()?, &ACTIVE)?;
    for expected in [None, Some(&first)] {
        match lease.ledger_select(&executable, &s, expected, b"three", budgets()?, &ACTIVE) {
            Err(GitWriteError::Conflict { actual }) => assert_eq!(actual, Some(second.clone())),
            other => return Err(format!("expected conflict, got {other:?}").into()),
        }
    }
    assert_eq!(
        lease.ledger_head(&executable, &s, budgets()?, &ACTIVE)?,
        Some(second)
    );
    drop(lease);
    fixture.cleanup()
}

#[test]
fn refs_hex_encode_streams_and_foreign_commits_refuse() -> Result {
    assert_eq!(
        ledger_ref(&stream("a..b")?),
        "refs/writfold/ledger/612e2e62"
    );
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    // A blob is not a record commit.
    let blob = lease.write_blob(&executable, b"not a commit", budgets()?, &ACTIVE)?;
    assert!(
        lease
            .ledger_commit(&executable, &blob, budgets()?, &ACTIVE)
            .is_err()
    );
    drop(lease);
    fixture.cleanup()
}

#[test]
fn commits_outside_the_ledger_profile_refuse() -> Result {
    let file = executable_file()?;
    let executable = executable(&file)?;
    let fixture = Repository::new(&executable)?;
    let lease = fixture.writer.try_lease()?;
    let s = stream("run.1")?;
    let good = lease.ledger_select(&executable, &s, None, b"record", budgets()?, &ACTIVE)?;
    let read = lease.ledger_commit(&executable, &good, budgets()?, &ACTIVE)?;
    assert_eq!(read.record, b"record".to_vec());
    // Same tree, different message: not this profile's commit.
    let git = |args: &[&str]| -> Result<String> {
        let out = std::process::Command::new("git")
            .env("GIT_DIR", &fixture.path)
            .args(args)
            .output()?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).into_owned().into());
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_owned())
    };
    let tree = git(&["rev-parse", &format!("{}^{{tree}}", good.as_str())])?;
    let foreign = git(&["commit-tree", &tree, "-m", "something else"])?;
    let foreign =
        writfold_domain::GitObjectId::new(writfold_domain::GitObjectFormat::Sha256, foreign)?;
    assert!(
        lease
            .ledger_commit(&executable, &foreign, budgets()?, &ACTIVE)
            .is_err()
    );
    drop(lease);
    fixture.cleanup()
}
