use super::support::{
    Fixture, TestResult, budget, identity, limits, object_path, private_write, realm,
};
use std::fs;
use std::os::unix::fs::symlink;
use writfold_store::objects::{ObjectError, ObjectStore};
use writfold_wire::engineering::objects::encode_identity;

#[test]
fn expected_realm_is_validated_without_altering_identity_or_objects() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let retained = store.put(b"original", budget(8)?)?;
    let identity = fs::read(fixture.realm.join("identity.json"))?;
    assert!(matches!(
        ObjectStore::open(&fixture.realm, &"other-realm".parse()?, limits()?),
        Err(ObjectError::WrongRealm)
    ));
    assert_eq!(fs::read(fixture.realm.join("identity.json"))?, identity);
    assert_eq!(
        fs::read(object_path(&fixture.realm, &retained))?,
        b"original"
    );
    Ok(())
}

#[test]
fn bound_identity_replacement_refuses_even_when_new_bytes_are_identical() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let retained = store.put(b"original", budget(8)?)?;
    let path = fixture.realm.join("identity.json");
    let bytes = fs::read(&path)?;
    fs::rename(&path, fixture.realm.join("identity-original"))?;
    private_write(&path, &bytes)?;
    assert!(matches!(
        store.get(&retained, budget(8)?),
        Err(ObjectError::IdentityChanged)
    ));
    assert!(matches!(
        store.put(b"new", budget(8)?),
        Err(ObjectError::IdentityChanged)
    ));
    assert_eq!(fs::read(&path)?, bytes);
    assert_eq!(fs::read(fixture.realm.join("identity-original"))?, bytes);
    assert_eq!(
        fs::read(object_path(&fixture.realm, &retained))?,
        b"original"
    );
    Ok(())
}

#[test]
fn in_place_incarnation_drift_refuses_for_fresh_and_reopened_handles() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let retained = store.put(b"original", budget(8)?)?;
    let reopened = ObjectStore::open(&fixture.realm, &realm()?, limits()?)?;
    let changed = encode_identity(&identity("instance-b")?, 4096)?;
    fs::write(fixture.realm.join("identity.json"), &changed)?;
    for handle in [&store, &reopened] {
        assert!(matches!(
            handle.get(&retained, budget(8)?),
            Err(ObjectError::IdentityChanged)
        ));
        assert!(matches!(
            handle.put(b"new", budget(8)?),
            Err(ObjectError::IdentityChanged)
        ));
    }
    assert_eq!(fs::read(fixture.realm.join("identity.json"))?, changed);
    assert_eq!(
        fs::read(object_path(&fixture.realm, &retained))?,
        b"original"
    );
    Ok(())
}

#[test]
fn malformed_or_linked_identity_is_not_admitted_by_open() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let retained = store.put(b"original", budget(8)?)?;
    let path = fixture.realm.join("identity.json");
    let original = fs::read(&path)?;
    for invalid in [
        br#"{"namespace":"writfold-realm","schema":1,"profile":"writfold-experimental-alpha/1","realm":"realm-a","realm":"realm-a","kb":"kb-a","instance":"instance-a"}"#.as_slice(),
        br#"{"namespace":"writfold-realm","schema":1,"profile":"writfold-experimental-alpha/1","realm":"realm-a","kb":"kb-a","instance":"instance-a","secret":"extra"}"#,
    ] {
        fs::write(&path, invalid)?;
        assert!(ObjectStore::open(&fixture.realm, &realm()?, limits()?).is_err());
        assert_eq!(fs::read(&path)?, invalid);
    }
    private_write(&fixture.parent.join("original-identity"), &original)?;
    fs::remove_file(&path)?;
    symlink(fixture.parent.join("original-identity"), &path)?;
    assert!(ObjectStore::open(&fixture.realm, &realm()?, limits()?).is_err());
    fs::remove_file(&path)?;
    fs::hard_link(fixture.parent.join("original-identity"), &path)?;
    assert!(ObjectStore::open(&fixture.realm, &realm()?, limits()?).is_err());
    assert_eq!(
        fs::read(fixture.parent.join("original-identity"))?,
        original
    );
    assert_eq!(
        fs::read(object_path(&fixture.realm, &retained))?,
        b"original"
    );
    Ok(())
}
