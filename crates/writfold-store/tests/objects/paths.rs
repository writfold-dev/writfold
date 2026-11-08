use super::support::{
    Fixture, TestResult, budget, identity, limits, object_path, private_write, realm,
};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use writfold_runtime::fs::FsError;
use writfold_store::objects::{ObjectError, ObjectStore};

#[test]
fn initialization_refuses_missing_ancestors_symlinks_and_traversal() -> TestResult {
    let fixture = Fixture::new()?;
    let alias = fixture.parent.join("alias");
    symlink(&fixture.parent, &alias)?;
    for path in [
        fixture.parent.join("missing/realm"),
        alias.join("realm"),
        fixture.parent.join("../escaped-realm"),
    ] {
        assert!(ObjectStore::initialize(path, identity("instance-a")?, limits()?).is_err());
    }
    assert!(!fixture.realm.exists());
    assert!(!fixture.parent.join("missing").exists());
    Ok(())
}

#[test]
fn root_and_contained_namespace_permissions_are_required_on_every_operation() -> TestResult {
    for relative in [
        "",
        "objects",
        "objects/sha256",
        "coordination",
        "identity.json",
    ] {
        let fixture = Fixture::new()?;
        let store = fixture.initialize()?;
        let reference = store.put(b"original", budget(8)?)?;
        let path = fixture.realm.join(relative);
        let mode = if relative == "identity.json" {
            0o644
        } else {
            0o755
        };
        fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
        assert!(ObjectStore::open(&fixture.realm, &realm()?, limits()?).is_err());
        assert!(store.get(&reference, budget(8)?).is_err());
        assert!(store.put(b"original", budget(8)?).is_err());
        assert_eq!(
            fs::read(object_path(&fixture.realm, &reference))?,
            b"original"
        );
    }
    Ok(())
}

#[test]
fn observed_namespace_replacements_never_redirect_retention() -> TestResult {
    for relative in ["objects", "objects/sha256", "coordination"] {
        let fixture = Fixture::new()?;
        let store = fixture.initialize()?;
        let reference = store.put(b"original", budget(8)?)?;
        let object = object_path(&fixture.realm, &reference);
        let path = fixture.realm.join(relative);
        let saved = fixture.parent.join("saved");
        fs::rename(&path, &saved)?;
        symlink(&saved, &path)?;
        assert!(store.get(&reference, budget(8)?).is_err());
        assert!(store.put(b"new", budget(8)?).is_err());
        assert!(ObjectStore::open(&fixture.realm, &realm()?, limits()?).is_err());
        assert_eq!(fs::read(object)?, b"original");
    }
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let reference = store.put(b"original", budget(8)?)?;
    let saved = fixture.parent.join("saved-root");
    fs::rename(&fixture.realm, &saved)?;
    symlink(&saved, &fixture.realm)?;
    assert!(store.get(&reference, budget(8)?).is_err());
    assert!(store.put(b"new", budget(8)?).is_err());
    assert_eq!(fs::read(object_path(&saved, &reference))?, b"original");
    Ok(())
}

#[test]
fn object_symlinks_nonregulars_and_shard_links_preserve_saved_originals() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let reference = store.put(b"original", budget(8)?)?;
    let path = object_path(&fixture.realm, &reference);
    let saved = fixture.parent.join("saved-object");
    fs::rename(&path, &saved)?;
    symlink(&saved, &path)?;
    assert!(store.get(&reference, budget(8)?).is_err());
    assert!(store.put(b"original", budget(8)?).is_err());
    fs::remove_file(&path)?;
    fs::create_dir(&path)?;
    assert!(matches!(
        store.get(&reference, budget(8)?),
        Err(ObjectError::Filesystem(FsError::NonRegular))
    ));
    assert!(store.put(b"original", budget(8)?).is_err());
    assert_eq!(fs::read(&saved)?, b"original");
    let shard = path.parent().ok_or("missing shard")?;
    let saved_shard = fixture.parent.join("saved-shard");
    fs::rename(shard, &saved_shard)?;
    symlink(&saved_shard, shard)?;
    assert!(store.get(&reference, budget(8)?).is_err());
    assert!(store.put(b"original", budget(8)?).is_err());
    assert_eq!(fs::read(saved)?, b"original");
    Ok(())
}

#[test]
fn created_coordination_entries_cannot_redirect_or_relax_retention() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let reference = store.put(b"original", budget(8)?)?;
    let entries = fs::read_dir(fixture.realm.join("coordination"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    for (index, path) in entries.iter().enumerate() {
        let original_inode = fs::metadata(path)?.ino();
        fs::set_permissions(path, fs::Permissions::from_mode(0o644))?;
        assert!(matches!(
            store.put(b"original", budget(8)?),
            Err(ObjectError::Filesystem(FsError::Permission))
        ));
        assert_eq!(store.get(&reference, budget(8)?)?, b"original");
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        let alias = fixture.parent.join(format!("lock-alias-{index}"));
        fs::hard_link(path, &alias)?;
        assert!(matches!(
            store.put(b"original", budget(8)?),
            Err(ObjectError::Filesystem(FsError::HardLinked))
        ));
        fs::remove_file(alias)?;
        let saved = fixture.parent.join(format!("saved-lock-{index}"));
        fs::rename(path, &saved)?;
        let foreign = fixture.parent.join(format!("foreign-{index}"));
        private_write(&foreign, b"untouched")?;
        let foreign_inode = fs::metadata(&foreign)?.ino();
        symlink(&foreign, path)?;
        assert!(store.put(b"original", budget(8)?).is_err());
        assert_eq!(fs::metadata(&foreign)?.ino(), foreign_inode);
        assert_eq!(fs::read(&foreign)?, b"untouched");
        fs::remove_file(path)?;
        fs::rename(saved, path)?;
        assert_eq!(fs::metadata(path)?.ino(), original_inode);
        assert_eq!(
            fs::read(object_path(&fixture.realm, &reference))?,
            b"original"
        );
    }
    Ok(())
}

#[test]
fn insecure_shard_and_missing_retained_file_do_not_change_saved_bytes() -> TestResult {
    let fixture = Fixture::new()?;
    let store = fixture.initialize()?;
    let reference = store.put(b"original", budget(8)?)?;
    let path = object_path(&fixture.realm, &reference);
    let shard = path.parent().ok_or("missing shard")?;
    fs::set_permissions(shard, fs::Permissions::from_mode(0o755))?;
    assert!(store.get(&reference, budget(8)?).is_err());
    assert!(store.put(b"original", budget(8)?).is_err());
    assert_eq!(fs::read(&path)?, b"original");
    fs::set_permissions(shard, fs::Permissions::from_mode(0o700))?;
    let saved = fixture.parent.join("saved-original");
    fs::rename(&path, &saved)?;
    assert!(matches!(
        store.get(&reference, budget(8)?),
        Err(ObjectError::Filesystem(FsError::Missing))
    ));
    assert!(!path.exists());
    assert_eq!(fs::read(saved)?, b"original");
    Ok(())
}
