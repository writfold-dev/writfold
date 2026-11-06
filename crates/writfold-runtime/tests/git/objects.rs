use super::support::*;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use writfold_domain::{GitObjectFormat, GitObjectId};
use writfold_runtime::fs::RelativePath;
use writfold_runtime::git::GitError;

#[test]
fn exact_history_binary_native_paths_and_replacements() -> Result {
    for format in [GitObjectFormat::Sha1, GitObjectFormat::Sha256] {
        let fixture = Fixture::new(format)?;
        let name = OsStr::from_bytes(b"-literal\tline\r\n[?]*\xff");
        let original = b"duplicate\r\nduplicate\r\n\0\xffbinary";
        fs::write(fixture.path.join(name), original)?;
        fs::create_dir(fixture.path.join("nested"))?;
        fs::write(fixture.path.join("nested/same"), original)?;
        let commit = fixture.commit()?;
        let blob = fixture.blob(original)?;
        fs::write(fixture.path.join(name), b"new HEAD")?;
        let successor = fixture.commit()?;
        fs::write(fixture.path.join(name), b"dirty worktree")?;
        fixture.git(&["replace", commit.as_str(), successor.as_str()], &[])?;
        let replacement = fixture.blob(b"wrong replacement")?;
        fixture.git(&["replace", blob.as_str(), replacement.as_str()], &[])?;
        let repository = fixture.repository(format)?;
        let file = executable_file()?;
        let executable = executable(&file)?;
        for path in [RelativePath::new(name)?, RelativePath::new("nested/same")?] {
            let entry = repository.lookup_blob(&executable, &commit, &path, budgets()?, &ACTIVE)?;
            assert_eq!(entry.blob(), &blob);
            let observed =
                repository.read_blob(&executable, &commit, &path, &blob, budgets()?, &ACTIVE)?;
            assert_eq!(observed.bytes(), original);
            assert_eq!(observed.commit(), &commit);
            assert_eq!(observed.path(), &path);
        }
    }
    Ok(())
}

#[test]
fn typed_path_type_format_identity_and_missing_failures() -> Result {
    let fixture = Fixture::new(GitObjectFormat::Sha1)?;
    fs::write(fixture.path.join("file"), b"actual")?;
    symlink("file", fixture.path.join("link"))?;
    let first = fixture.commit()?;
    fixture.git(
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "160000",
            first.as_str(),
            "submodule",
        ],
        &[],
    )?;
    fixture.git(&["commit", "--quiet", "-m", "submodule"], &[])?;
    let commit = oid(&fixture.git(&["rev-parse", "HEAD"], &[])?)?;
    let blob = fixture.blob(b"actual")?;
    let wrong = fixture.blob(b"other")?;
    let repository = fixture.repository(GitObjectFormat::Sha1)?;
    let file = executable_file()?;
    let executable = executable(&file)?;
    let path = RelativePath::new("file")?;
    assert!(matches!(
        repository.read_blob(&executable, &commit, &path, &wrong, budgets()?, &ACTIVE),
        Err(GitError::ObjectMismatch)
    ));
    for name in ["link", "submodule", "link/child"] {
        assert!(matches!(
            repository.lookup_blob(
                &executable,
                &commit,
                &RelativePath::new(name)?,
                budgets()?,
                &ACTIVE
            ),
            Err(GitError::ObjectType)
        ));
    }
    assert!(matches!(
        repository.lookup_blob(
            &executable,
            &commit,
            &RelativePath::new("absent")?,
            budgets()?,
            &ACTIVE
        ),
        Err(GitError::MissingPath)
    ));
    assert!(matches!(
        repository.lookup_blob(&executable, &blob, &path, budgets()?, &ACTIVE),
        Err(GitError::ObjectType)
    ));
    let missing: GitObjectId = "0".repeat(40).parse()?;
    assert!(matches!(
        repository.lookup_blob(&executable, &missing, &path, budgets()?, &ACTIVE),
        Err(GitError::Process(ProcessError::Command { .. }))
    ));
    let other_format: GitObjectId = "0".repeat(64).parse()?;
    assert!(matches!(
        repository.lookup_blob(&executable, &other_format, &path, budgets()?, &ACTIVE),
        Err(GitError::ObjectFormat)
    ));
    Ok(())
}

#[test]
fn corrupt_named_commit_tree_and_blob_never_acquire_provenance() -> Result {
    for kind in ["commit", "tree", "blob"] {
        let fixture = Fixture::new(GitObjectFormat::Sha1)?;
        fs::write(fixture.path.join("file"), b"original")?;
        let commit = fixture.commit()?;
        let blob = fixture.blob(b"original")?;
        let tree = oid(&fixture.git(&["rev-parse", "HEAD^{tree}"], &[])?)?;
        fs::write(fixture.path.join("file"), b"substitute")?;
        let alternate_commit = fixture.commit()?;
        let alternate_blob = fixture.blob(b"substitute")?;
        let alternate_tree = oid(&fixture.git(&["rev-parse", "HEAD^{tree}"], &[])?)?;
        let (target, replacement) = match kind {
            "commit" => (&commit, &alternate_commit),
            "tree" => (&tree, &alternate_tree),
            _ => (&blob, &alternate_blob),
        };
        let object_path = |oid: &GitObjectId| {
            fixture
                .path
                .join(".git/objects")
                .join(&oid.as_str()[..2])
                .join(&oid.as_str()[2..])
        };
        fs::copy(object_path(replacement), object_path(target))?;
        let repository = fixture.repository(GitObjectFormat::Sha1)?;
        let file = executable_file()?;
        let executable = executable(&file)?;
        assert!(
            repository
                .read_blob(
                    &executable,
                    &commit,
                    &RelativePath::new("file")?,
                    &blob,
                    budgets()?,
                    &ACTIVE
                )
                .is_err(),
            "wrongly named {kind} was accepted"
        );
    }
    Ok(())
}

#[test]
fn pinned_directory_survives_rename_without_following_replacement() -> Result {
    let fixture = Fixture::new(GitObjectFormat::Sha1)?;
    fs::write(fixture.path.join("file"), b"pinned")?;
    let commit = fixture.commit()?;
    let blob = fixture.blob(b"pinned")?;
    let repository = fixture.repository(GitObjectFormat::Sha1)?;
    fs::rename(fixture.path.join(".git"), fixture.path.join("retained"))?;
    symlink("/nonexistent", fixture.path.join(".git"))?;
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
    assert_eq!(observed.bytes(), b"pinned");
    Ok(())
}
