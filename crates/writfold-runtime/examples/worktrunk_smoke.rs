//! Static-musl consumer: worktrunk_smoke GIT_PATH GIT_SHA256 WT_PATH FIXTURE_ROOT.
//! Parent prepares disposable SHA1/SHA256 repositories and armed positive hook
//! controls under its recorded container timeout. This consumer invokes only
//! the public adapter, retaining every attempt and its successful raw stdout.
//! No owner/grant/full-gate claim is made.
#[path = "worktrunk_smoke/fixture.rs"]
mod fixture;
use fixture::{Result, count};
use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use writfold_domain::{ContentDigest, GitObjectFormat, GitObjectId};
use writfold_runtime::fs::{ContainedRoot, RelativePath};
use writfold_runtime::git::{GitError, GitExecutable};
use writfold_runtime::process::ProcessSupervisor;
use writfold_runtime::worktrunk::{
    Branch, CreateError, CreateRequest, Destinations, Limits, Source, Worktrunk,
};

fn main() -> Result {
    let args: Vec<_> = std::env::args().collect();
    let corrupt = args.len() == 6 && args[5] == "--expect-corrupt-refusal";
    if args.len() != 5 && !corrupt {
        return Err("usage: worktrunk_smoke GIT_PATH GIT_SHA256 WT_PATH FIXTURE_ROOT [--expect-corrupt-refusal]".into());
    }
    let root = Path::new(&args[4]);
    eprintln!("retained Worktrunk native smoke: {}", root.display());
    let supervisor = ProcessSupervisor::enable()?;
    let git_file = fixture::pin(Path::new(&args[1]))?;
    let digest: ContentDigest = args[2].parse()?;
    let git = GitExecutable::admit(&git_file, &digest, count(64 << 20)?, &supervisor)?;
    let wt_file = fixture::pin(Path::new(&args[3]))?;
    let adapter = Worktrunk::admit(&wt_file, &git, &root.join("controls"))?;
    let wrong: ContentDigest = "0".repeat(64).parse()?;
    assert!(matches!(
        GitExecutable::admit(&git_file, &wrong, count(64 << 20)?, &supervisor),
        Err(GitError::ExecutableMismatch)
    ));
    assert!(Worktrunk::admit(&git_file, &git, &root.join("wrong-wt")).is_err());
    assert!(!root.join("wrong-wt").exists());
    for (name, format) in [
        ("sha1", GitObjectFormat::Sha1),
        ("sha256", GitObjectFormat::Sha256),
    ] {
        if corrupt {
            corrupt_refusal(&adapter, &root.join(name), format)?;
        } else {
            exercise(&adapter, &root.join(name), format)?;
        }
    }
    if corrupt {
        println!(
            "SHA1/SHA256 corrupt base closure refused before creation; refs and original bytes preserved"
        );
    } else {
        println!(
            "SHA1/SHA256 native bindings, preservation, conflicts, pins and armed-control nonexecution observed; retained {}, not full alpha qualification",
            root.display()
        );
    }
    Ok(())
}

fn limits() -> Result<Limits> {
    Ok(Limits {
        stdout: count(1 << 20)?,
        stderr: count(65536)?,
        metadata: count(4 << 20)?,
        checkout: count(4 << 20)?,
        timeout: Duration::from_secs(30),
    })
}

fn exercise(adapter: &Worktrunk<'_, '_>, directory: &Path, format: GitObjectFormat) -> Result {
    let repo = directory.join("repo");
    let destination = directory.join("destinations");
    let source = Source::open(&repo, format)?;
    let destinations = Destinations::open(&destination)?;
    let base = GitObjectId::new(
        format,
        fs::read_to_string(directory.join("base"))?
            .trim()
            .to_owned(),
    )?;
    let cancelled = AtomicBool::new(false);
    let original = fs::read(repo.join("file"))?;
    let created = adapter.create(
        &source,
        &destinations,
        CreateRequest {
            leaf: &RelativePath::new("created")?,
            branch: &Branch::new("owned-run")?,
            base: &base,
        },
        limits()?,
        &cancelled,
    )?;
    assert_eq!(created.base, base);
    assert_eq!(created.branch.as_str(), "owned-run");
    assert_eq!(created.common_directory, repo.join(".git"));
    assert_eq!(created.path, destination.join("created"));
    assert_eq!(fs::read(created.path.join("file"))?, b"exact base\0\n");
    assert_eq!(
        fs::read(created.path.join("nested/copy"))?,
        b"exact base\0\n"
    );
    assert_eq!(fs::read(created.path.join("nested/empty"))?, b"");
    assert_eq!(fs::read(repo.join("file"))?, original);
    fs::write(
        directory.join("adapter.stdout.json"),
        &created.worktrunk_stdout,
    )?;
    fs::write(created.path.join("file"), b"candidate edit, not original\n")?;
    assert_eq!(fs::read(repo.join("file"))?, original);
    for marker in [
        "git-marker",
        "user-marker",
        "background-marker",
        "system-marker",
        "ambient-git-marker",
    ] {
        assert!(
            !directory.join(marker).exists(),
            "armed marker ran: {marker}"
        );
    }
    refusals(adapter, &source, &destinations, &base, directory)?;
    Ok(())
}

fn refusals(
    adapter: &Worktrunk<'_, '_>,
    source: &Source,
    destinations: &Destinations,
    base: &GitObjectId,
    directory: &Path,
) -> Result {
    let active = AtomicBool::new(false);
    let repo = directory.join("repo");
    let destination = directory.join("destinations");
    let refs = fixture::reference_snapshot(&repo)?;
    let occupied = destination.join("occupied");
    ContainedRoot::create(&occupied)?;
    fs::write(occupied.join("keep"), b"never clobber\0")?;
    for (leaf, branch) in [
        ("occupied", "blocked-path"),
        ("branch-conflict", "owned-run"),
    ] {
        assert!(matches!(
            adapter.create(
                source,
                destinations,
                CreateRequest {
                    leaf: &RelativePath::new(leaf)?,
                    branch: &Branch::new(branch)?,
                    base,
                },
                limits()?,
                &active
            ),
            Err(CreateError::Unissued(_))
        ));
        assert_eq!(fixture::reference_snapshot(&repo)?, refs);
    }
    assert_eq!(fs::read(occupied.join("keep"))?, b"never clobber\0");
    assert!(!destination.join("branch-conflict").exists());
    let stopped = AtomicBool::new(true);
    assert!(matches!(
        adapter.create(
            source,
            destinations,
            CreateRequest {
                leaf: &RelativePath::new("cancelled")?,
                branch: &Branch::new("cancelled-run")?,
                base,
            },
            limits()?,
            &stopped
        ),
        Err(CreateError::Unissued(_))
    ));
    assert!(!destination.join("cancelled").exists());
    ContainedRoot::create(repo.join(".config"))?;
    let hook = format!(
        "pre-start = \"printf armed > {}\"\npost-start = \"printf armed > {}\"\n",
        directory.join("project-marker").display(),
        directory.join("project-background-marker").display()
    );
    fs::write(repo.join(".config/wt.toml"), hook.as_bytes())?;
    assert!(matches!(
        adapter.create(
            source,
            destinations,
            CreateRequest {
                leaf: &RelativePath::new("project-refused")?,
                branch: &Branch::new("project-refused")?,
                base,
            },
            limits()?,
            &active
        ),
        Err(CreateError::Unissued(_))
    ));
    assert!(!destination.join("project-refused").exists());
    assert!(!directory.join("project-marker").exists());
    assert!(!directory.join("project-background-marker").exists());
    assert_eq!(fs::read(repo.join(".config/wt.toml"))?, hook.as_bytes());
    assert_eq!(fixture::reference_snapshot(&repo)?, refs);
    Ok(())
}

fn corrupt_refusal(
    adapter: &Worktrunk<'_, '_>,
    directory: &Path,
    format: GitObjectFormat,
) -> Result {
    let repo = directory.join("repo");
    let destination = directory.join("destinations");
    let source = Source::open(&repo, format)?;
    let destinations = Destinations::open(&destination)?;
    let base = GitObjectId::new(
        format,
        fs::read_to_string(directory.join("base"))?
            .trim()
            .to_owned(),
    )?;
    let original = fs::read(repo.join("file"))?;
    let refs = fixture::reference_snapshot(&repo)?;
    let cancelled = AtomicBool::new(false);
    let result = adapter.create(
        &source,
        &destinations,
        CreateRequest {
            leaf: &RelativePath::new("corrupt-base")?,
            branch: &Branch::new("corrupt-base")?,
            base: &base,
        },
        limits()?,
        &cancelled,
    );
    assert!(
        matches!(result, Err(CreateError::Unissued(_))),
        "{result:?}"
    );
    assert_eq!(fixture::reference_snapshot(&repo)?, refs);
    assert_eq!(fs::read(repo.join("file"))?, original);
    assert!(!destination.join("corrupt-base").exists());
    Ok(())
}
