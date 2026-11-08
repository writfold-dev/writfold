use super::{Branch, WorktrunkError, command, profile, tree};
use writfold_domain::GitObjectFormat;

const SHA1: &[u8] = b"[core]\nrepositoryformatversion = 0\nbare = false\nfilemode = true\nlogallrefupdates = true\n";

#[test]
fn branch_profile_refuses_revision_and_path_syntax() {
    for name in [
        "", "-option", "a/b", "a..b", "a.lock", "HEAD", "HEAD~1", "main\n", "@{x}",
    ] {
        assert!(Branch::new(name).is_err(), "{name:?}");
    }
    assert!(Branch::new("run-A_42").is_ok());
    assert!(Branch::new(&"a".repeat(129)).is_err());
}

#[test]
fn config_is_closed_without_weakening_immutable_reader() {
    assert!(profile::config(SHA1, GitObjectFormat::Sha1).is_ok());
    assert!(profile::config(SHA1, GitObjectFormat::Sha256).is_err());
    for extra in [
        "hooksPath = /tmp/hooks\n",
        "fsmonitor = command\n",
        "worktree = /tmp/other\n",
        "[include]\npath = /tmp/config\n",
        "[filter \"evil\"]\nsmudge = command\n",
        "[remote \"origin\"]\nurl = https://example.invalid\n",
        "[alias]\nx = !command\n",
        "[extensions]\nworktreeConfig = true\n",
        "[worktrunk]\nunknown = x\n",
    ] {
        let mut bytes = SHA1.to_vec();
        bytes.extend_from_slice(extra.as_bytes());
        assert!(
            profile::config(&bytes, GitObjectFormat::Sha1).is_err(),
            "{extra}"
        );
    }
    let mut state = SHA1.to_vec();
    state.extend_from_slice(b"[worktrunk]\nhistory = main\nhistory-path = /private/source\n[worktrunk \"hints\"]\nworktree-path = 1\n");
    assert!(profile::config(&state, GitObjectFormat::Sha1).is_ok());
    assert!(profile::config(b"[core]\nrepositoryformatversion = 1\nbare = false\n[extensions]\nobjectformat = sha256\n", GitObjectFormat::Sha256).is_ok());
}

fn tree(mode: &str, kind: &str, size: &str, path: &str) -> Vec<u8> {
    format!("{mode} {kind} {} {size}\t{path}\0", "a".repeat(40)).into_bytes()
}

fn check_tree(bytes: &[u8], limit: u64) -> Result<(), WorktrunkError> {
    tree::Entries::new(bytes, GitObjectFormat::Sha1, limit)?.try_for_each(|entry| entry.map(|_| ()))
}

#[test]
fn checkout_profile_bounds_bytes_and_refuses_executable_configuration() {
    assert!(check_tree(&tree("100644", "blob", "4", "file"), 4).is_ok());
    assert!(check_tree(&tree("100755", "blob", "4", "bin/tool"), 4).is_ok());
    assert!(check_tree(&tree("040000", "tree", "-", "bin"), 0).is_ok());
    assert!(check_tree(&tree("100644", "blob", "5", "file"), 4).is_err());
    for (mode, kind, size, path) in [
        ("120000", "blob", "1", "link"),
        ("160000", "commit", "-", "submodule"),
        ("100644", "blob", "1", ".config/wt.toml"),
        ("100644", "blob", "1", "nested/.gitattributes"),
        ("100644", "blob", "1", ".gitmodules"),
        ("100644", "blob", "1", "../escape"),
    ] {
        assert!(
            check_tree(&tree(mode, kind, size, path), 1024).is_err(),
            "{path}"
        );
    }
    assert!(check_tree(b"malformed", 1024).is_err());
    assert!(check_tree(b"\0", 1024).is_err());
    assert!(check_tree(&[], 0).is_ok());
}

#[test]
fn observation_lines_are_exact_not_trimmed() {
    assert_eq!(command::line(b"main\n").ok(), Some("main"));
    for bytes in [
        b"main".as_slice(),
        b"main\r\n",
        b"main\nother\n",
        b"\n",
        b"\xff\n",
    ] {
        assert!(matches!(
            command::line(bytes),
            Err(WorktrunkError::Observation(_))
        ));
    }
}

#[test]
fn source_fingerprint_preserves_bytes_and_refuses_links() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::fs::ContainedRoot;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use writfold_domain::ByteCount;
    let path = std::env::temp_dir().join(format!(
        "writfold-worktrunk-profile-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    let root = ContainedRoot::create(&path)?;
    let limits = super::Limits {
        stdout: ByteCount::new(1024)?,
        stderr: ByteCount::new(1024)?,
        metadata: ByteCount::new(1024)?,
        checkout: ByteCount::new(1024)?,
        timeout: Duration::from_secs(1),
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    let cancelled = std::sync::atomic::AtomicBool::new(false);
    fs::write(path.join("file"), b"original\0")?;
    let original = profile::fingerprint(&root, limits, deadline, &cancelled)?;
    ContainedRoot::create(path.join(".git"))?;
    fs::write(path.join(".git/config"), b"native metadata may change")?;
    assert_eq!(
        profile::fingerprint(&root, limits, deadline, &cancelled)?,
        original
    );
    fs::write(path.join("file"), b"changed\0")?;
    assert_ne!(
        profile::fingerprint(&root, limits, deadline, &cancelled)?,
        original
    );
    symlink("file", path.join("link"))?;
    assert!(profile::fingerprint(&root, limits, deadline, &cancelled).is_err());
    fs::remove_file(path.join("link"))?;
    fs::hard_link(path.join("file"), path.join("hardlink"))?;
    assert!(profile::fingerprint(&root, limits, deadline, &cancelled).is_err());
    fs::remove_dir_all(path)?;
    Ok(())
}
