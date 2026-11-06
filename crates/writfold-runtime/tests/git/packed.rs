use super::support::*;
use std::fs;
use std::os::unix::fs::symlink;
use writfold_domain::GitObjectFormat;
use writfold_runtime::fs::RelativePath;
use writfold_runtime::git::GitError;

#[test]
fn repacked_and_garbage_collected_histories_preserve_exact_pinned_bytes() -> Result {
    for format in [GitObjectFormat::Sha1, GitObjectFormat::Sha256] {
        let fixture = Fixture::new(format)?;
        let original = b"duplicate\r\nduplicate\r\n\0\xffpacked historical bytes";
        fs::write(fixture.path.join("file"), original)?;
        let commit = fixture.commit()?;
        let blob = fixture.blob(original)?;
        fs::write(fixture.path.join("file"), b"successor\r\n\0\xff")?;
        fixture.commit()?;
        add_harmless_config(&fixture)?;
        fixture.git(&["repack", "-ad", "--write-bitmap-index"], &[])?;
        fixture.git(&["prune-packed"], &[])?;
        fixture.git(&["gc", "--prune=now"], &[])?;
        assert!(
            !fixture
                .path
                .join(".git/objects")
                .join(&blob.as_str()[..2])
                .join(&blob.as_str()[2..])
                .exists(),
            "fixture must actually read packed bytes"
        );
        let mut packed = false;
        for entry in fs::read_dir(fixture.path.join(".git/objects/pack"))? {
            if entry?
                .path()
                .extension()
                .is_some_and(|extension| extension == "pack")
            {
                packed = true;
            }
        }
        assert!(packed, "real repack/gc must produce a packfile");
        fs::write(fixture.path.join("file"), b"dirty worktree")?;
        let repository = fixture.repository(format)?;
        let file = executable_file()?;
        let executable = executable(&file)?;
        let path = RelativePath::new("file")?;
        let entry = repository.lookup_blob(&executable, &commit, &path, budgets()?, &ACTIVE)?;
        assert_eq!(entry.blob(), &blob);
        let observed =
            repository.read_blob(&executable, &commit, &path, &blob, budgets()?, &ACTIVE)?;
        assert_eq!(observed.bytes(), original);
        assert_eq!(observed.commit(), &commit);
        let missing = "0".repeat(commit.as_str().len()).parse()?;
        assert!(matches!(
            repository.lookup_blob(&executable, &missing, &path, budgets()?, &ACTIVE),
            Err(GitError::Process(ProcessError::Command { .. }))
        ));
        assert!(!fixture.path.join("transport-helper-executed").exists());
    }
    Ok(())
}

fn add_harmless_config(fixture: &Fixture) -> Result {
    for (key, value) in [
        ("remote.origin.url", "ext::touch transport-helper-executed"),
        ("remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"),
        ("remote.origin.pushurl", "ssh://example.invalid/unused"),
        ("branch.main.remote", "origin"),
        ("branch.main.merge", "refs/heads/main"),
        ("user.name", "Packed \"Fixture\""),
        ("user.email", "packed@example.invalid"),
        ("user.signingkey", "unused-by-read-only-commands"),
    ] {
        fixture.git(&["config", key, value], &[])?;
    }
    Ok(())
}

#[test]
fn pack_symlinks_nonregular_entries_and_promisors_are_refused() -> Result {
    for entry_kind in ["symlink", "directory", "promisor", "unknown"] {
        let fixture = Fixture::new(GitObjectFormat::Sha1)?;
        let name = format!(
            "pack-{}.{}",
            "0".repeat(40),
            if entry_kind == "promisor" {
                "promisor"
            } else {
                "idx"
            }
        );
        let path = fixture.path.join(".git/objects/pack").join(name);
        match entry_kind {
            "symlink" => {
                fs::write(fixture.path.join("external-index"), b"not admitted")?;
                symlink(fixture.path.join("external-index"), &path)?;
            }
            "directory" => fs::create_dir(&path)?,
            "promisor" => fs::write(&path, b"")?,
            _ => fs::write(fixture.path.join(".git/objects/pack/unrecognized"), b"")?,
        }
        assert!(
            fixture.repository(GitObjectFormat::Sha1).is_err(),
            "admitted {entry_kind}"
        );
    }
    Ok(())
}

#[test]
fn harmless_sections_do_not_admit_helpers_includes_or_partial_clones() -> Result {
    for (key, value) in [
        ("remote.origin.promisor", "true"),
        ("remote.origin.partialclonefilter", "blob:none"),
        ("include.path", "../external-config"),
        ("core.sshcommand", "touch transport-helper-executed"),
        ("extensions.partialclone", "origin"),
    ] {
        let fixture = Fixture::new(GitObjectFormat::Sha1)?;
        add_harmless_config(&fixture)?;
        fixture.git(&["config", key, value], &[])?;
        assert!(
            fixture.repository(GitObjectFormat::Sha1).is_err(),
            "admitted {key}"
        );
        assert!(!fixture.path.join("transport-helper-executed").exists());
    }
    Ok(())
}
