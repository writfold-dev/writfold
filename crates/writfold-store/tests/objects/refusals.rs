use super::support::{Fixture, TestResult, budget, object_path, private_write};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use writfold_domain::engineering::objects::BlobReference;
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};
use writfold_runtime::fs::{ContainedRoot, FsError, LockKey, ManagedLocks, RelativePath};
use writfold_store::objects::ObjectError;

#[test]
fn wrong_realm_length_and_digest_references_cannot_substitute_objects() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let retained = store.put(b"original", budget(8)?)?;
    let other = BlobReference::new(
        "other-realm".parse()?,
        *retained.digest(),
        *retained.bytes(),
    );
    assert!(matches!(
        store.get(&other, budget(8)?),
        Err(ObjectError::WrongRealm)
    ));
    for size in [0, 7, 9] {
        let wrong = BlobReference::new(retained.realm().clone(), *retained.digest(), budget(size)?);
        assert!(matches!(
            store.get(&wrong, budget(32)?),
            Err(ObjectError::ObjectLength { .. })
        ));
    }
    let absent = BlobReference::new(
        retained.realm().clone(),
        writfold_wire::engineering::blob_fingerprint(b"absent"),
        budget(6)?,
    );
    assert!(store.get(&absent, budget(6)?).is_err());
    assert_eq!(store.get(&retained, budget(8)?)?, b"original");
    Ok(())
}

#[test]
fn raw_sha256_named_bytes_do_not_qualify_as_engineering_blob_bytes() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let original = store.put(b"original", budget(8)?)?;
    let raw = BlobReference::new(
        original.realm().clone(),
        BlobDigest::from_digest(Sha256::digest(b"original").into()),
        budget(8)?,
    );
    let path = object_path(&fixture.realm, &raw);
    let shard = path.parent().ok_or("missing shard")?;
    if !shard.exists() {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(0o700).create(shard)?;
    }
    private_write(&path, b"original")?;
    assert!(matches!(
        store.get(&raw, budget(8)?),
        Err(ObjectError::ObjectDigest)
    ));
    assert_eq!(fs::read(&path)?, b"original");
    assert_eq!(store.get(&original, budget(8)?)?, b"original");
    Ok(())
}

#[test]
fn corruption_and_truncation_are_never_repaired_by_duplicate_put() -> TestResult {
    for corrupt in [
        b"corrupt!".as_slice(),
        b"short",
        b"longer than original",
        b"",
    ] {
        let fixture = Fixture::new()?;
        let store = fixture.initialize()?;
        let retained = store.put(b"original", budget(8)?)?;
        let untouched = store.put(b"another original", budget(32)?)?;
        let path = object_path(&fixture.realm, &retained);
        fs::write(&path, corrupt)?;
        let inode = fs::metadata(&path)?.ino();
        assert!(store.get(&retained, budget(32)?).is_err());
        assert!(store.put(b"original", budget(8)?).is_err());
        assert_eq!(fs::metadata(&path)?.ino(), inode);
        assert_eq!(fs::read(&path)?, corrupt);
        assert_eq!(store.get(&untouched, budget(32)?)?, b"another original");
    }
    Ok(())
}

#[test]
fn observed_private_access_changes_and_hardlinks_refuse_without_replacement() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let retained = store.put(b"original", budget(8)?)?;
    let path = object_path(&fixture.realm, &retained);
    let inode = fs::metadata(&path)?.ino();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644))?;
    assert!(matches!(
        store.get(&retained, budget(8)?),
        Err(ObjectError::Filesystem(FsError::Permission))
    ));
    assert!(matches!(
        store.put(b"original", budget(8)?),
        Err(ObjectError::Filesystem(FsError::Permission))
    ));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    fs::hard_link(&path, fixture.parent.join("alias"))?;
    assert!(matches!(
        store.get(&retained, budget(8)?),
        Err(ObjectError::Filesystem(FsError::HardLinked))
    ));
    assert!(matches!(
        store.put(b"original", budget(8)?),
        Err(ObjectError::Filesystem(FsError::HardLinked))
    ));
    assert_eq!(fs::metadata(&path)?.ino(), inode);
    assert_eq!(fs::read(&path)?, b"original");
    assert_eq!(fs::read(fixture.parent.join("alias"))?, b"original");
    Ok(())
}

#[test]
fn object_writer_contention_refuses_immediately_without_touching_original() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let retained = store.put(b"original", budget(8)?)?;
    let path = object_path(&fixture.realm, &retained);
    let root = ContainedRoot::open(&fixture.realm)?;
    let locks = ManagedLocks::from_root(root.open_child(&RelativePath::new("coordination")?)?);
    let target = root.pin(&RelativePath::new(path.strip_prefix(&fixture.realm)?)?)?;
    let hex = format!(
        "{}{}",
        path.parent()
            .and_then(|p| p.file_name())
            .ok_or("missing shard")?
            .to_string_lossy(),
        path.file_name().ok_or("missing leaf")?.to_string_lossy()
    );
    let _guard = target.try_lock_private(&locks, &LockKey::new(&format!("blob-{hex}"))?)?;
    assert!(matches!(
        store.put(b"original", budget(8)?),
        Err(ObjectError::Filesystem(FsError::LockBusy))
    ));
    assert_eq!(store.get(&retained, budget(8)?)?, b"original");
    assert_eq!(fs::read(path)?, b"original");
    Ok(())
}

#[test]
fn retention_waits_for_the_actual_identity_publication_owner() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let original = store.put(b"original", budget(8)?)?;
    let root = ContainedRoot::open(&fixture.realm)?;
    let identity = root.pin(&RelativePath::new("identity.json")?)?;
    let locks = ManagedLocks::open(fixture.realm.join("coordination"))?;
    let held = identity.try_lock_private(&locks, &LockKey::new("realm-identity")?)?;
    let reopened = writfold_store::objects::ObjectStore::open(
        &fixture.realm,
        original.realm(),
        writfold_wire::Limits::new(4096, 16, 32, 128)?,
    )?;
    assert_eq!(reopened.get(&original, budget(8)?)?, b"original");
    assert!(matches!(
        reopened.put(b"pending", budget(7)?),
        Err(ObjectError::Filesystem(FsError::LockBusy))
    ));
    drop(held);
    let retained = reopened.put(b"pending", budget(7)?)?;
    assert_eq!(reopened.get(&retained, budget(7)?)?, b"pending");
    assert_eq!(reopened.get(&original, budget(8)?)?, b"original");
    Ok(())
}
