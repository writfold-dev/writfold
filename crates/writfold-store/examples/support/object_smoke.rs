#[path = "object_smoke_adversaries.rs"]
mod adversaries;

use std::error::Error;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use writfold_domain::ByteCount;
use writfold_domain::engineering::EngineeringDigest;
use writfold_domain::engineering::objects::{BlobReference, RealmIdentity};
use writfold_runtime::fs::ContainedRoot;
use writfold_store::objects::ObjectStore;
use writfold_wire::Limits;

type SmokeResult<T = ()> = Result<T, Box<dyn Error>>;

pub fn run(directory: &Path) -> SmokeResult {
    let workspace = ContainedRoot::create(directory)?;
    workspace.check_private()?;
    let path = directory.join("realm");
    let store = ObjectStore::initialize(&path, identity("instance-a")?, limits()?)?;
    let originals = [
        b"".as_slice(),
        b"\0\xff\x80\r\n\0",
        "naïve\r\n雪\r\n".as_bytes(),
    ];
    let mut references = Vec::with_capacity(originals.len());
    for original in originals {
        let reference = store.put(original, budget(original.len() as u64)?)?;
        require(
            store.get(&reference, budget(original.len() as u64)?)? == original,
            "fresh byte fidelity",
        )?;
        require(
            fs::read(object_path(&path, &reference))? == original,
            "actual retained bytes",
        )?;
        references.push(reference);
    }
    drop(store);
    let store = ObjectStore::open(&path, &"realm-smoke".parse()?, limits()?)?;
    for (reference, original) in references.iter().zip(originals) {
        let object = object_path(&path, reference);
        let before = fs::metadata(&object)?;
        let duplicate = store.put(original, budget(original.len() as u64)?)?;
        let after = fs::metadata(&object)?;
        require(
            (before.dev(), before.ino()) == (after.dev(), after.ino()),
            "duplicate inode reuse",
        )?;
        require(
            reference.realm() == duplicate.realm()
                && reference.digest() == duplicate.digest()
                && reference.bytes() == duplicate.bytes(),
            "duplicate reference",
        )?;
        require(
            store.get(reference, budget(original.len() as u64)?)? == original,
            "reopened byte fidelity",
        )?;
    }
    let lock_count = fs::read_dir(path.join("coordination"))?.count();
    for reference in &references {
        let _bytes = store.get(reference, *reference.bytes())?;
    }
    require(
        fs::read_dir(path.join("coordination"))?.count() == lock_count,
        "read created a lock",
    )?;
    adversaries::run(directory, &path, &store, &references[1])?;
    for (reference, original) in references.iter().zip(originals) {
        require(
            store.get(reference, budget(original.len() as u64)?)? == original,
            "refusal changed an original",
        )?;
    }
    println!(
        "retained_objects_smoke=ok fresh=reopened exact_bytes=verified duplicates=inode_reused refusals=verified authority=none evidence={}",
        directory.display()
    );
    Ok(())
}

fn identity(instance: &str) -> SmokeResult<RealmIdentity> {
    Ok(RealmIdentity::new(
        "realm-smoke".parse()?,
        "kb-smoke".parse()?,
        instance.parse()?,
    ))
}

fn limits() -> SmokeResult<Limits> {
    Ok(Limits::new(4096, 4, 16, 32)?)
}

fn budget(bytes: u64) -> SmokeResult<ByteCount> {
    Ok(ByteCount::new(bytes)?)
}

fn require(accepted: bool, failure: &'static str) -> SmokeResult {
    if accepted {
        Ok(())
    } else {
        Err(failure.into())
    }
}

fn object_path(root: &Path, reference: &BlobReference) -> PathBuf {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hex = String::with_capacity(64);
    for byte in reference.digest().as_digest() {
        hex.push(char::from(HEX[usize::from(byte >> 4)]));
        hex.push(char::from(HEX[usize::from(byte & 15)]));
    }
    root.join("objects/sha256").join(&hex[..2]).join(&hex[2..])
}
