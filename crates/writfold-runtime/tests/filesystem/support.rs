use std::error::Error;
use std::fs;
use std::io;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use writfold_domain::ByteCount;
use writfold_runtime::fs::{ContainedRoot, ManagedLocks, PinnedFile, RelativePath};

pub type TestResult = Result<(), Box<dyn Error>>;
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

pub struct Fixture {
    pub path: PathBuf,
}

impl Fixture {
    pub fn new() -> io::Result<Self> {
        Self::inside(&std::env::temp_dir())
    }

    pub fn inside(parent: &Path) -> io::Result<Self> {
        for _ in 0..128 {
            let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                ".writfold-fs-test-{}-{sequence}",
                std::process::id()
            ));
            match fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "fixture names exhausted",
        ))
    }

    pub fn target(&self, path: impl AsRef<Path>) -> Result<PinnedFile, Box<dyn Error>> {
        Ok(ContainedRoot::open(&self.path)?.pin(&RelativePath::new(path)?)?)
    }

    pub fn locks(&self) -> Result<ManagedLocks, Box<dyn Error>> {
        Ok(ManagedLocks::create(self.path.join("managed-locks"))?)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn limit(bytes: u64) -> Result<ByteCount, Box<dyn Error>> {
    Ok(ByteCount::new(bytes)?)
}

pub fn temporary_names(path: &Path) -> io::Result<Vec<std::ffi::OsString>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(path)? {
        let name = entry?.file_name();
        if name.to_string_lossy().starts_with(".writfold-tmp-") {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}
