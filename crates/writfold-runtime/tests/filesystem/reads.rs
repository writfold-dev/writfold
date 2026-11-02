use super::support::{Fixture, TestResult, limit};
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::symlink;
use std::path::Path;
use writfold_runtime::fs::{ContainedRoot, FsError, ManagedLocks, ReadExtent, RelativePath};

#[test]
fn relative_paths_reject_escape_without_utf8_coercion() -> TestResult {
    for path in [
        "", "/abs", "..", "a/..", "../b", "a//b", "a/", ".", "a/./b", "nul\0x",
    ] {
        assert!(RelativePath::new(path).is_err(), "accepted {path:?}");
    }
    let fixture = Fixture::new()?;
    let name = OsString::from_vec(b"source-\xff".to_vec());
    fs::write(fixture.path.join(&name), b"abc")?;
    let path = RelativePath::new(Path::new(&name))?;
    assert_eq!(path.as_path().as_os_str(), name);
    let read = ContainedRoot::open(&fixture.path)?
        .pin(&path)?
        .read(limit(3)?)?;
    assert_eq!(read.bytes(), b"abc");
    assert_eq!(
        read.digest().as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    Ok(())
}

#[test]
fn temporary_spelling_remains_regular_user_data() -> TestResult {
    let fixture = Fixture::new()?;
    let name = ".writfold-tmp-user-notes.md";
    fs::write(fixture.path.join(name), b"retained user document")?;
    let root = ContainedRoot::open(&fixture.path)?;
    let read = root.pin(&RelativePath::new(name)?)?.read(limit(64)?)?;
    assert_eq!(read.bytes(), b"retained user document");
    assert_eq!(read.extent(), ReadExtent::Complete);
    Ok(())
}

#[test]
fn ordinary_open_and_reads_never_initialize() -> TestResult {
    let fixture = Fixture::new()?;
    let absent = fixture.path.join("absent");
    assert!(matches!(
        ContainedRoot::open(&absent),
        Err(FsError::Missing)
    ));
    assert!(matches!(ManagedLocks::open(&absent), Err(FsError::Missing)));
    assert!(!absent.exists());
    let root = ContainedRoot::open(&fixture.path)?;
    assert!(matches!(
        root.pin(&RelativePath::new("parent/file")?),
        Err(FsError::Missing)
    ));
    assert!(matches!(
        root.pin(&RelativePath::new("leaf")?)?.read(limit(8)?),
        Err(FsError::Missing)
    ));
    assert_eq!(fs::read_dir(&fixture.path)?.count(), 0);
    let initialized = ContainedRoot::create(&absent)?;
    assert!(absent.is_dir());
    assert!(matches!(
        ContainedRoot::create(&absent),
        Err(FsError::AlreadyExists)
    ));
    assert!(matches!(
        initialized
            .pin(&RelativePath::new("missing")?)?
            .read(limit(8)?),
        Err(FsError::Missing)
    ));
    assert_eq!(fs::read_dir(&absent)?.count(), 0);
    Ok(())
}

#[test]
fn root_admission_refuses_symlink_leaf_and_ancestors() -> TestResult {
    let fixture = Fixture::new()?;
    fs::create_dir_all(fixture.path.join("real/nested"))?;
    symlink(fixture.path.join("real"), fixture.path.join("alias"))?;
    for path in [
        fixture.path.join("alias"),
        fixture.path.join("alias/nested"),
    ] {
        assert!(matches!(ContainedRoot::open(path), Err(FsError::Symlink)));
    }
    assert!(matches!(
        ContainedRoot::create(fixture.path.join("alias/new")),
        Err(FsError::Symlink)
    ));
    assert!(!fixture.path.join("real/new").exists());
    assert!(matches!(
        ContainedRoot::open(fixture.path.join("real/..")),
        Err(FsError::InvalidPath(_))
    ));
    Ok(())
}

#[test]
fn relative_root_admission_opens_each_component() -> TestResult {
    const CHILD_ROOT: &str = "WRITFOLD_FS_RELATIVE_ROOT";
    if let Some(relative) = std::env::var_os(CHILD_ROOT) {
        let relative = Path::new(&relative);
        let root = ContainedRoot::open(relative.join("real/nested"))?;
        assert_eq!(
            root.pin(&RelativePath::new("content")?)?
                .read(limit(16)?)?
                .bytes(),
            b"contained"
        );
        assert!(matches!(
            ContainedRoot::open(relative.join("alias/nested")),
            Err(FsError::Symlink)
        ));
        return Ok(());
    }
    let fixture = Fixture::new()?;
    fs::create_dir_all(fixture.path.join("real/nested"))?;
    fs::write(fixture.path.join("real/nested/content"), b"contained")?;
    symlink("real", fixture.path.join("alias"))?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "reads::relative_root_admission_opens_each_component",
        ])
        .current_dir(fixture.path.parent().ok_or("fixture parent missing")?)
        .env(
            CHILD_ROOT,
            fixture.path.file_name().ok_or("fixture leaf missing")?,
        )
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn contained_paths_refuse_ancestor_and_leaf_links() -> TestResult {
    let fixture = Fixture::new()?;
    let outside = Fixture::new()?;
    fs::write(outside.path.join("secret"), b"outside")?;
    symlink(&outside.path, fixture.path.join("ancestor"))?;
    symlink(outside.path.join("secret"), fixture.path.join("leaf"))?;
    let root = ContainedRoot::open(&fixture.path)?;
    assert!(matches!(
        root.pin(&RelativePath::new("ancestor/secret")?),
        Err(FsError::Symlink)
    ));
    assert!(matches!(
        root.pin(&RelativePath::new("leaf")?)?.read(limit(32)?),
        Err(FsError::Symlink)
    ));
    assert_eq!(fs::read(outside.path.join("secret"))?, b"outside");
    Ok(())
}

#[test]
fn nonregular_files_are_refused_without_fifo_blocking() -> TestResult {
    let fixture = Fixture::new()?;
    fs::create_dir(fixture.path.join("directory"))?;
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        fixture.path.join("fifo"),
        rustix::fs::Mode::RWXU,
    )?;
    for name in ["directory", "fifo"] {
        assert!(matches!(
            fixture.target(name)?.read(limit(16)?),
            Err(FsError::NonRegular)
        ));
    }
    Ok(())
}

#[test]
fn admitted_root_survives_path_rename_without_redirection() -> TestResult {
    let fixture = Fixture::new()?;
    fs::create_dir(fixture.path.join("root"))?;
    fs::create_dir(fixture.path.join("outside"))?;
    fs::write(fixture.path.join("root/file"), b"inside")?;
    fs::write(fixture.path.join("outside/file"), b"outside")?;
    let root = ContainedRoot::open(fixture.path.join("root"))?;
    fs::rename(fixture.path.join("root"), fixture.path.join("moved"))?;
    symlink("outside", fixture.path.join("root"))?;
    let read = root.pin(&RelativePath::new("file")?)?.read(limit(64)?)?;
    assert_eq!(read.bytes(), b"inside");
    Ok(())
}

#[test]
fn opened_leaf_replacement_and_in_place_change_are_observed() -> TestResult {
    let fixture = Fixture::new()?;
    fs::write(fixture.path.join("file"), b"original")?;
    let target = fixture.target("file")?;
    let opened = target.open_read()?;
    fs::rename(fixture.path.join("file"), fixture.path.join("old"))?;
    fs::write(fixture.path.join("file"), b"replacement")?;
    assert!(matches!(
        opened.read(limit(64)?),
        Err(FsError::ChangedSource)
    ));
    let current = target.open_read()?;
    fs::write(fixture.path.join("file"), b"different-size")?;
    assert!(matches!(
        current.read(limit(64)?),
        Err(FsError::ChangedSource)
    ));
    Ok(())
}

#[test]
fn bounded_reads_report_prefix_digest_and_zero_budget_explicitly() -> TestResult {
    let fixture = Fixture::new()?;
    fs::write(fixture.path.join("file"), b"abcdef")?;
    let target = fixture.target("file")?;
    let prefix = target.read(limit(3)?)?;
    assert_eq!(prefix.bytes(), b"abc");
    assert_eq!(
        prefix.digest().as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        prefix.extent(),
        ReadExtent::Truncated {
            limit: limit(3)?,
            observed_size: 6
        }
    );
    let full = target.read(limit(6)?)?;
    assert_eq!(full.bytes(), b"abcdef");
    assert_eq!(full.extent(), ReadExtent::Complete);
    let none = target.read(limit(0)?)?;
    assert!(none.bytes().is_empty());
    assert_eq!(
        none.extent(),
        ReadExtent::Truncated {
            limit: limit(0)?,
            observed_size: 6
        }
    );
    assert_eq!(
        none.digest().as_str(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    fs::write(fixture.path.join("empty"), [])?;
    assert_eq!(
        fixture.target("empty")?.read(limit(0)?)?.extent(),
        ReadExtent::Complete
    );
    Ok(())
}

#[test]
fn binary_reads_cross_buffer_boundaries_without_loss() -> TestResult {
    let fixture = Fixture::new()?;
    let bytes: Vec<u8> = (0..65_539).map(|index| (index % 256) as u8).collect();
    fs::write(fixture.path.join("file"), &bytes)?;
    let target = fixture.target("file")?;
    assert_eq!(target.read(limit(bytes.len() as u64)?)?.into_bytes(), bytes);
    assert_eq!(target.read(limit(32_769)?)?.bytes(), &bytes[..32_769]);
    Ok(())
}
