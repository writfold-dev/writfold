mod dolt_cases;

use rustix::fs::{FlockOperation, flock};
use std::fs::{self, File};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use writfold_domain::ByteCount;
use writfold_runtime::{
    dolt::{DoltDatabase, DoltError},
    fs::{ContainedRoot, FsError, RelativePath},
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    metadata_limit: ByteCount,
}
fn private(path: &Path) -> std::io::Result<()> {
    fs::DirBuilder::new().mode(0o700).create(path)
}
impl Fixture {
    fn new() -> Result<Self> {
        let metadata_limit = ByteCount::new(4096)?;
        for _ in 0..128 {
            let root = std::env::temp_dir().join(format!(
                ".writfold-dolt-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match private(&root) {
                Ok(()) => {
                    let fixture = Self {
                        root,
                        metadata_limit,
                    };
                    for path in ["db", "home", "db/.dolt", "db/coordination"] {
                        private(&fixture.path(path))?;
                    }
                    fs::write(fixture.path("db/coordination/database.lock"), b"owned lock")?;
                    fs::set_permissions(
                        fixture.path("db/coordination/database.lock"),
                        fs::Permissions::from_mode(0o600),
                    )?;
                    fs::write(fixture.path("unrelated"), b"must survive")?;
                    return Ok(fixture);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err("fixture names exhausted".into())
    }
    fn path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }
    fn open(&self) -> std::result::Result<DoltDatabase, DoltError> {
        DoltDatabase::open(
            ContainedRoot::open(self.path("db"))?,
            ContainedRoot::open(self.path("home"))?,
            self.metadata_limit,
        )
    }
    fn preserved(&self) -> Result {
        assert_eq!(fs::read(self.path("unrelated"))?, b"must survive");
        Ok(())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn missing<T: std::fmt::Debug>(result: std::result::Result<T, DoltError>) {
    assert!(
        matches!(result, Err(DoltError::Filesystem(FsError::Missing))),
        "{result:?}"
    );
}
fn stale<T: std::fmt::Debug>(result: std::result::Result<T, DoltError>) {
    assert!(
        matches!(result, Err(DoltError::Filesystem(FsError::ChangedSource))),
        "{result:?}"
    );
}
fn profile<T: std::fmt::Debug>(result: std::result::Result<T, DoltError>) {
    assert!(
        matches!(result, Err(DoltError::UnsupportedProfile(_))),
        "{result:?}"
    );
}

#[test]
fn missing_prerequisites_are_never_created() -> Result {
    for path in [
        "db/coordination/database.lock",
        "db/coordination",
        "db/.dolt",
        "db",
        "home",
    ] {
        let fixture = Fixture::new()?;
        let absent = fixture.path(path);
        if absent.is_dir() {
            fs::remove_dir_all(&absent)?;
        } else {
            fs::remove_file(&absent)?;
        }
        missing(fixture.open());
        assert!(!absent.try_exists()?);
        fixture.preserved()?;
    }
    Ok(())
}

#[test]
fn real_flock_conflict_release_and_reacquire_preserve_lock() -> Result {
    let fixture = Fixture::new()?;
    let lock_path = fixture.path("db/coordination/database.lock");
    let before = fs::metadata(&lock_path)?;
    let database = fixture.open()?;
    let competing = fixture.open()?;
    let external = File::open(&lock_path)?;
    flock(&external, FlockOperation::NonBlockingLockExclusive)?;
    assert!(matches!(
        database.acquire(),
        Err(DoltError::Filesystem(FsError::LockBusy))
    ));
    drop(external);
    let lease = database.acquire()?;
    lease.check_current()?;
    assert!(matches!(
        competing.acquire(),
        Err(DoltError::Filesystem(FsError::LockBusy))
    ));
    drop(lease);
    let lease = competing.acquire()?;
    lease.check_current()?;
    drop(lease);
    database.acquire()?.check_current()?;
    let after = fs::metadata(&lock_path)?;
    assert_eq!(
        (before.dev(), before.ino(), before.nlink()),
        (after.dev(), after.ino(), after.nlink())
    );
    assert_eq!(fs::read(lock_path)?, b"owned lock");
    assert_eq!(fs::read_dir(fixture.path("home"))?.count(), 0);
    fixture.preserved()
}

#[test]
fn removed_lock_is_not_recreated_on_acquisition_or_lease_check() -> Result {
    let fixture = Fixture::new()?;
    let database = fixture.open()?;
    let lease = database.acquire()?;
    let lock = fixture.path("db/coordination/database.lock");
    let saved = fixture.path("saved.lock");
    fs::rename(&lock, &saved)?;
    stale(lease.check_current());
    drop(lease);
    missing(database.acquire());
    assert!(!lock.try_exists()?);
    assert_eq!(fs::read(&saved)?, b"owned lock");
    fs::rename(saved, &lock)?;
    database.acquire()?.check_current()?;
    fixture.preserved()
}

#[test]
fn replaced_lock_rejects_old_database_even_after_guard_release() -> Result {
    let fixture = Fixture::new()?;
    let database = fixture.open()?;
    let lease = database.acquire()?;
    let lock = fixture.path("db/coordination/database.lock");
    fs::rename(&lock, fixture.path("saved.lock"))?;
    fs::write(&lock, b"replacement")?;
    stale(lease.check_current());
    drop(lease);
    stale(database.acquire());
    assert_eq!(fs::read(&lock)?, b"replacement");
    assert_eq!(fs::read(fixture.path("saved.lock"))?, b"owned lock");
    fixture.preserved()
}

#[test]
fn replaced_directory_bindings_invalidate_leases_and_acquisitions() -> Result {
    for path in ["db/coordination", "db/.dolt", "db", "home"] {
        let fixture = Fixture::new()?;
        let database = fixture.open()?;
        let lease = database.acquire()?;
        let original = fixture.path(path);
        let saved = fixture.path("saved-directory");
        fs::rename(&original, &saved)?;
        private(&original)?;
        fs::write(original.join("replacement-marker"), b"do not touch")?;
        stale(lease.check_current());
        drop(lease);
        stale(database.acquire());
        assert_eq!(
            fs::read(original.join("replacement-marker"))?,
            b"do not touch"
        );
        assert_eq!(fs::read_dir(&original)?.count(), 1);
        fs::remove_dir_all(&original)?;
        fs::rename(saved, original)?;
        database.acquire()?.check_current()?;
        fixture.preserved()?;
    }
    Ok(())
}

#[test]
fn symlink_directory_admission_refuses_without_touching_target() -> Result {
    for path in ["db", "db/.dolt", "db/coordination", "home"] {
        let fixture = Fixture::new()?;
        let original = fixture.path(path);
        let saved = fixture.path("symlink-target");
        fs::rename(&original, &saved)?;
        fs::write(saved.join("sentinel"), b"unchanged")?;
        symlink(&saved, &original)?;
        assert!(matches!(
            fixture.open(),
            Err(DoltError::Filesystem(FsError::Symlink))
        ));
        assert!(fs::symlink_metadata(&original)?.file_type().is_symlink());
        assert_eq!(fs::read(saved.join("sentinel"))?, b"unchanged");
        fixture.preserved()?;
    }
    Ok(())
}

#[test]
fn symlink_nonregular_and_hardlinked_locks_refuse() -> Result {
    let fixture = Fixture::new()?;
    let lock = fixture.path("db/coordination/database.lock");
    fs::remove_file(&lock)?;
    symlink(fixture.path("unrelated"), &lock)?;
    assert!(matches!(
        fixture.open(),
        Err(DoltError::Filesystem(FsError::Symlink))
    ));
    fs::remove_file(&lock)?;
    private(&lock)?;
    assert!(matches!(
        fixture.open(),
        Err(DoltError::Filesystem(FsError::NonRegular))
    ));
    fs::remove_dir(&lock)?;
    fs::hard_link(fixture.path("unrelated"), &lock)?;
    // The actual locking boundary refuses aliases of the preprovisioned entry.
    let database = fixture.open()?;
    assert!(matches!(
        database.acquire(),
        Err(DoltError::Filesystem(FsError::HardLinked))
    ));
    assert_eq!(fs::metadata(&lock)?.nlink(), 2);
    fixture.preserved()
}

#[test]
fn nonprivate_directories_and_shared_home_refuse() -> Result {
    for path in ["db", "home", "db/coordination"] {
        let fixture = Fixture::new()?;
        fs::set_permissions(fixture.path(path), fs::Permissions::from_mode(0o750))?;
        profile(fixture.open());
        assert_eq!(fs::metadata(fixture.path(path))?.mode() & 0o777, 0o750);
        fixture.preserved()?;
    }
    let fixture = Fixture::new()?;
    profile(DoltDatabase::open(
        ContainedRoot::open(fixture.path("db"))?,
        ContainedRoot::open(fixture.path("db"))?,
        ByteCount::new(4096)?,
    ));
    fixture.preserved()
}

#[test]
fn native_server_discovery_and_global_credentials_refuse_before_and_after_open() -> Result {
    for path in [
        "db/sql-server.info",
        "db/sql-server.lock",
        "db/.dolt/sql-server.info",
        "db/.dolt/sql-server.lock",
        "home/sql-server.info",
        "home/sql-server.lock",
        "home/config_global.json",
        "home/config.json",
        "home/creds",
        "home/.dolt/config_global.json",
        "home/.dolt/config.json",
        "home/.dolt/creds",
        "home/.dolt/sql-server.info",
        "home/.dolt/sql-server.lock",
    ] {
        let fixture = Fixture::new()?;
        if path.starts_with("home/.dolt/") {
            private(&fixture.path("home/.dolt"))?;
        }
        let database = fixture.open()?;
        let lease = database.acquire()?;
        let marker = fixture.path(path);
        fs::write(&marker, b"synthetic prohibited fixture")?;
        profile(fixture.open());
        profile(lease.check_current());
        drop(lease);
        profile(database.acquire());
        assert_eq!(fs::read(&marker)?, b"synthetic prohibited fixture");
        fs::remove_file(marker)?;
        database.acquire()?.check_current()?;
        fixture.preserved()?;
    }
    Ok(())
}

#[test]
fn metadata_budgets_are_exact_and_never_truncate_files() -> Result {
    let fixture = Fixture::new()?;
    let open = |limit| -> Result<std::result::Result<DoltDatabase, DoltError>> {
        Ok(DoltDatabase::open(
            ContainedRoot::open(fixture.path("db"))?,
            ContainedRoot::open(fixture.path("home"))?,
            ByteCount::new(limit)?,
        ))
    };
    assert!(matches!(
        open(9)?,
        Err(DoltError::Budget {
            stream: "lock metadata",
            limit: 9
        })
    ));
    open(10)??.acquire()?.check_current()?;
    fs::write(fixture.path("db/.dolt/config.json"), b"01234567890")?;
    assert!(matches!(
        open(10)?,
        Err(DoltError::Budget {
            stream: "configuration",
            limit: 10
        })
    ));
    open(11)??.acquire()?.check_current()?;
    assert_eq!(
        fs::read(fixture.path("db/.dolt/config.json"))?,
        b"01234567890"
    );
    assert_eq!(
        fs::read(fixture.path("db/coordination/database.lock"))?,
        b"owned lock"
    );
    fixture.preserved()
}

#[test]
fn ordinary_pinned_reads_survive_rename_but_dolt_opt_in_identity_does_not() -> Result {
    let fixture = Fixture::new()?;
    let root = ContainedRoot::open(fixture.path("db"))?;
    let pinned = root.pin(&RelativePath::new("coordination/database.lock")?)?;
    let database = fixture.open()?;
    fs::rename(fixture.path("db"), fixture.path("renamed-db"))?;
    stale(database.acquire());
    assert!(matches!(root.check_current(), Err(FsError::ChangedSource)));
    let read = pinned.read(ByteCount::new(10)?)?;
    assert_eq!(read.bytes(), b"owned lock");
    assert!(!fixture.path("db").try_exists()?);
    fixture.preserved()
}
