use super::support::{Fixture, TestResult, budget, identity, limits, names, object_path, realm};
use std::fs;
use std::os::unix::fs::MetadataExt;
use writfold_store::objects::{ObjectError, ObjectStore};
use writfold_wire::{Limits, engineering::blob_fingerprint};

#[test]
fn fresh_and_reopened_stores_retain_exact_empty_binary_and_utf8_bytes() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    assert!(names(&fixture.realm.join("objects/sha256"))?.is_empty());
    for original in [
        b"".as_slice(),
        b"\0\xff\x80\r\n\0",
        "naïve\r\n雪\r\n".as_bytes(),
    ] {
        let reference = store.put(original, budget(original.len() as u64)?)?;
        assert_eq!(reference.digest(), &blob_fingerprint(original));
        assert_eq!(reference.bytes().get(), original.len() as u64);
        assert_eq!(fs::read(object_path(&fixture.realm, &reference))?, original);
        assert_eq!(
            store.get(&reference, budget(original.len() as u64)?)?,
            original
        );
        let reopened = ObjectStore::open(&fixture.realm, &realm()?, limits()?)?;
        assert_eq!(
            reopened.get(&reference, budget(original.len() as u64)?)?,
            original
        );
    }
    for name in names(&fixture.realm.join("objects/sha256"))? {
        assert_eq!(name.as_encoded_bytes().len(), 2);
    }
    assert_eq!(fs::metadata(&fixture.realm)?.mode() & 0o7777, 0o700);
    assert_eq!(
        fs::metadata(fixture.realm.join("identity.json"))?.mode() & 0o7777,
        0o600
    );
    Ok(())
}

#[test]
fn duplicate_put_verifies_original_and_reuses_inode_after_reopen() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let original = b"exact original\0\xff\r\n";
    let first = store.put(original, budget(128)?)?;
    let path = object_path(&fixture.realm, &first);
    let before = fs::metadata(&path)?;
    drop(store);
    let reopened = ObjectStore::open(&fixture.realm, &realm()?, limits()?)?;
    let duplicate = reopened.put(original, budget(128)?)?;
    let after = fs::metadata(&path)?;
    assert_eq!(first.realm(), duplicate.realm());
    assert_eq!(first.digest(), duplicate.digest());
    assert_eq!(first.bytes(), duplicate.bytes());
    assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
    assert_eq!(after.nlink(), 1);
    assert_eq!(fs::read(&path)?, original);
    Ok(())
}

#[test]
fn initialization_budgets_refuse_before_creating_any_root() -> TestResult {
    let fixture = Fixture::new()?;
    for limits in [
        Limits::new(0, 4, 16, 32)?,
        Limits::new(4096, 0, 16, 32)?,
        Limits::new(4096, 4, 0, 32)?,
        Limits::new(4096, 4, 16, 0)?,
    ] {
        assert!(ObjectStore::initialize(&fixture.realm, identity("instance-a")?, limits).is_err());
        assert!(!fixture.realm.exists());
        assert!(names(&fixture.parent)?.is_empty());
    }
    let _store = fixture.initialize()?;
    let before = fs::read(fixture.realm.join("identity.json"))?;
    assert!(ObjectStore::initialize(&fixture.realm, identity("different")?, limits()?).is_err());
    assert_eq!(fs::read(fixture.realm.join("identity.json"))?, before);
    Ok(())
}

#[test]
fn ordinary_get_and_open_never_add_locks_and_put_budget_is_preflight() -> TestResult {
    let fixture = Fixture::new()?;
    assert!(ObjectStore::open(&fixture.realm, &realm()?, limits()?).is_err());
    assert!(!fixture.realm.exists());
    let store = fixture.initialize()?;
    let reference = store.put(b"original", budget(8)?)?;
    let before = names(&fixture.realm.join("coordination"))?;
    let shards = names(&fixture.realm.join("objects/sha256"))?;
    let reopened = ObjectStore::open(&fixture.realm, &realm()?, limits()?)?;
    assert_eq!(reopened.get(&reference, budget(8)?)?, b"original");
    assert!(matches!(
        store.put(b"too large", budget(0)?),
        Err(ObjectError::Budget { .. })
    ));
    assert!(matches!(
        store.get(&reference, budget(7)?),
        Err(ObjectError::Budget { .. })
    ));
    assert_eq!(names(&fixture.realm.join("coordination"))?, before);
    assert_eq!(names(&fixture.realm.join("objects/sha256"))?, shards);
    assert_eq!(
        fs::read(object_path(&fixture.realm, &reference))?,
        b"original"
    );
    Ok(())
}
