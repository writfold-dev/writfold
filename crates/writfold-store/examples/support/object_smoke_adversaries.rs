use super::{SmokeResult, budget, identity, limits, object_path, require};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::Path;
use writfold_domain::engineering::objects::BlobReference;
use writfold_store::objects::{ObjectError, ObjectStore};
use writfold_wire::engineering::objects::encode_identity;

pub(super) fn run(
    workspace: &Path,
    root: &Path,
    store: &ObjectStore,
    original: &BlobReference,
) -> SmokeResult {
    let wrong_realm = BlobReference::new(
        "wrong-realm".parse()?,
        *original.digest(),
        *original.bytes(),
    );
    require(
        matches!(
            store.get(&wrong_realm, budget(64)?),
            Err(ObjectError::WrongRealm)
        ),
        "realm refusal",
    )?;
    for length in [0, original.bytes().get() + 1] {
        let wrong_length = BlobReference::new(
            original.realm().clone(),
            *original.digest(),
            budget(length)?,
        );
        require(
            matches!(
                store.get(&wrong_length, budget(64)?),
                Err(ObjectError::ObjectLength { .. })
            ),
            "length refusal",
        )?;
    }
    require(
        store.get(original, budget(0)?).is_err(),
        "get budget refusal",
    )?;
    require(
        store.put(b"oversized", budget(0)?).is_err(),
        "put budget refusal",
    )?;
    require(
        ObjectStore::initialize(root, identity("other")?, limits()?).is_err(),
        "existing root refusal",
    )?;
    corrupted(root, store)?;
    paths(workspace, root, store)?;
    drift(workspace)?;
    Ok(())
}

fn corrupted(root: &Path, store: &ObjectStore) -> SmokeResult {
    let reference = store.put(b"original", budget(8)?)?;
    let path = object_path(root, &reference);
    let before = fs::metadata(&path)?;
    fs::write(&path, b"corrupt!")?;
    require(
        matches!(
            store.get(&reference, budget(8)?),
            Err(ObjectError::ObjectDigest)
        ),
        "corrupt get refusal",
    )?;
    require(
        store.put(b"original", budget(8)?).is_err(),
        "corrupt duplicate refusal",
    )?;
    require(
        fs::read(&path)? == b"corrupt!",
        "corrupt object overwritten",
    )?;
    require(
        fs::metadata(&path)?.ino() == before.ino(),
        "corrupt inode replaced",
    )?;
    Ok(())
}

fn paths(workspace: &Path, root: &Path, store: &ObjectStore) -> SmokeResult {
    let linked = store.put(b"symlink-original", budget(32)?)?;
    let path = object_path(root, &linked);
    let saved = fs::canonicalize(workspace)?.join("symlink-original");
    fs::rename(&path, &saved)?;
    symlink(&saved, &path)?;
    require(
        store.get(&linked, budget(32)?).is_err(),
        "symlink get refusal",
    )?;
    require(
        store.put(b"symlink-original", budget(32)?).is_err(),
        "symlink put refusal",
    )?;
    require(
        fs::read(&saved)? == b"symlink-original",
        "symlink target changed",
    )?;
    let linked = store.put(b"hardlink-original", budget(32)?)?;
    let path = object_path(root, &linked);
    let saved = workspace.join("hardlink-original");
    fs::hard_link(&path, &saved)?;
    require(
        store.get(&linked, budget(32)?).is_err(),
        "hardlink get refusal",
    )?;
    require(
        store.put(b"hardlink-original", budget(32)?).is_err(),
        "hardlink put refusal",
    )?;
    require(
        fs::read(&saved)? == b"hardlink-original",
        "hardlink target changed",
    )?;
    let private = store.put(b"private-original", budget(32)?)?;
    let path = object_path(root, &private);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644))?;
    require(
        store.get(&private, budget(32)?).is_err(),
        "privacy get refusal",
    )?;
    require(
        store.put(b"private-original", budget(32)?).is_err(),
        "privacy put refusal",
    )?;
    require(
        fs::read(&path)? == b"private-original",
        "private original changed",
    )?;
    Ok(())
}

fn drift(workspace: &Path) -> SmokeResult {
    let path = workspace.join("identity-drift");
    let store = ObjectStore::initialize(&path, identity("instance-a")?, limits()?)?;
    let reference = store.put(b"identity-original", budget(32)?)?;
    let changed = encode_identity(&identity("instance-b")?, 4096)?;
    fs::write(path.join("identity.json"), &changed)?;
    require(
        matches!(
            store.get(&reference, budget(32)?),
            Err(ObjectError::IdentityChanged)
        ),
        "identity get refusal",
    )?;
    require(
        matches!(
            store.put(b"new", budget(32)?),
            Err(ObjectError::IdentityChanged)
        ),
        "identity put refusal",
    )?;
    require(
        fs::read(path.join("identity.json"))? == changed,
        "identity silently rebound",
    )?;
    require(
        fs::read(object_path(&path, &reference))? == b"identity-original",
        "identity refusal changed bytes",
    )?;
    Ok(())
}
