use std::error::Error;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use writfold_domain::engineering::EngineeringDigest;
use writfold_domain::engineering::objects::{BlobReference, RealmIdentity};
use writfold_domain::{ByteCount, RealmId};
use writfold_store::objects::ObjectStore;
use writfold_wire::Limits;

pub type TestResult = Result<(), Box<dyn Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Fixture {
    pub parent: PathBuf,
    pub realm: PathBuf,
}

impl Fixture {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let parent = std::env::temp_dir().join(format!(
            ".writfold-objects-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::DirBuilder::new().mode(0o700).create(&parent)?;
        Ok(Self {
            realm: parent.join("realm"),
            parent,
        })
    }

    pub fn initialize(&self) -> Result<ObjectStore, Box<dyn Error>> {
        Ok(ObjectStore::initialize(
            &self.realm,
            identity("instance-a")?,
            limits()?,
        )?)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.parent);
    }
}

pub fn identity(instance: &str) -> Result<RealmIdentity, Box<dyn Error>> {
    Ok(RealmIdentity::new(
        "realm-a".parse()?,
        "kb-a".parse()?,
        instance.parse()?,
    ))
}

pub fn realm() -> Result<RealmId, Box<dyn Error>> {
    Ok("realm-a".parse()?)
}

pub fn limits() -> Result<Limits, Box<dyn Error>> {
    Ok(Limits::new(4096, 4, 16, 32)?)
}

pub fn budget(bytes: u64) -> Result<ByteCount, Box<dyn Error>> {
    Ok(ByteCount::new(bytes)?)
}

pub fn object_path(root: &Path, reference: &BlobReference) -> PathBuf {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hex = String::with_capacity(64);
    for byte in reference.digest().as_digest() {
        hex.push(char::from(HEX[usize::from(byte >> 4)]));
        hex.push(char::from(HEX[usize::from(byte & 15)]));
    }
    root.join("objects/sha256").join(&hex[..2]).join(&hex[2..])
}

pub fn private_write(path: &Path, bytes: &[u8]) -> TestResult {
    fs::write(path, bytes)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

pub fn names(path: &Path) -> Result<Vec<std::ffi::OsString>, Box<dyn Error>> {
    let mut names = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    names.sort();
    Ok(names)
}
