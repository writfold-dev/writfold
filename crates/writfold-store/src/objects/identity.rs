use super::{ObjectError, ObjectStore};
use std::path::Path;
use writfold_domain::engineering::objects::RealmIdentity;
use writfold_domain::{ByteCount, RealmId};
use writfold_runtime::fs::{
    ContainedRoot, FsError, LockKey, ManagedLocks, PinnedFile, ReadExtent, ReadResult, RelativePath,
};
use writfold_wire::Limits;
use writfold_wire::engineering::objects::{decode_identity, encode_identity};

pub(super) fn initialize(
    path: &Path,
    identity: RealmIdentity,
    limits: Limits,
) -> Result<ObjectStore, ObjectError> {
    let bytes = encode_identity(&identity, limits.bytes())?;
    // Decode before any filesystem effect to admit depth, entries and nodes too.
    decode_identity(&bytes, limits)?;
    let limit = byte_count(bytes.len())?;
    let root = ContainedRoot::create(path)?;
    root.check_private()?;
    let objects = root.create_child(&relative("objects")?)?;
    objects.check_private()?;
    let hashes = objects.create_child(&relative("sha256")?)?;
    hashes.check_private()?;
    let coordination = root.create_child(&relative("coordination")?)?;
    coordination.check_private()?;
    let locks = ManagedLocks::from_root(coordination);
    let identity_file = root.pin(&relative("identity.json")?)?;
    let receipt = {
        let mut guard = identity_file.try_lock_private(&locks, &LockKey::new("realm-identity")?)?;
        root.check_private()?;
        guard.compare_and_write(None, &bytes, limit, limit)?
    };
    let store = ObjectStore {
        root,
        objects,
        hashes,
        locks,
        identity_file,
        identity,
        identity_bytes: bytes,
        identity_inode: receipt.identity(),
        identity_limit: limit,
    };
    store.check_context()?;
    Ok(store)
}

pub(super) fn open(
    path: &Path,
    expected: &RealmId,
    limits: Limits,
) -> Result<ObjectStore, ObjectError> {
    let root = ContainedRoot::open(path)?;
    root.check_private()?;
    let objects = root.open_child(&relative("objects")?)?;
    objects.check_private()?;
    let hashes = objects.open_child(&relative("sha256")?)?;
    hashes.check_private()?;
    let coordination = root.open_child(&relative("coordination")?)?;
    coordination.check_private()?;
    let locks = ManagedLocks::from_root(coordination);
    let identity_file = root.pin(&relative("identity.json")?)?;
    let retained = private_read(&identity_file, byte_count(limits.bytes())?)?;
    if retained.extent() != ReadExtent::Complete {
        return Err(ObjectError::Budget {
            limit: byte_count(limits.bytes())?,
            required: observed_length(&retained),
        });
    }
    let identity = decode_identity(retained.bytes(), limits)?;
    if identity.realm() != expected {
        return Err(ObjectError::WrongRealm);
    }
    let store = ObjectStore {
        root,
        objects,
        hashes,
        locks,
        identity_file,
        identity,
        identity_inode: retained.identity(),
        identity_limit: byte_count(retained.bytes().len())?,
        identity_bytes: retained.into_bytes(),
    };
    store.check_context()?;
    Ok(store)
}

impl ObjectStore {
    pub(super) fn check_context(&self) -> Result<ReadResult, ObjectError> {
        // Child roots track their own bindings, not the whole parent chain.
        self.root.check_private()?;
        self.objects.check_private()?;
        self.hashes.check_private()?;
        self.locks.check_private()?;
        let retained = private_read(&self.identity_file, self.identity_limit)?;
        if retained.extent() != ReadExtent::Complete
            || retained.identity() != self.identity_inode
            || retained.bytes() != self.identity_bytes.as_slice()
        {
            return Err(ObjectError::IdentityChanged);
        }
        Ok(retained)
    }
}

pub(super) fn private_read(target: &PinnedFile, limit: ByteCount) -> Result<ReadResult, FsError> {
    let opened = target.open_read()?;
    opened.check_private()?;
    let retained = opened.read(limit)?;
    opened.check_private()?;
    Ok(retained)
}

pub(super) fn relative(path: &str) -> Result<RelativePath, FsError> {
    RelativePath::new(path).map_err(FsError::from)
}

pub(super) fn byte_count(bytes: usize) -> Result<ByteCount, ObjectError> {
    let bytes = u64::try_from(bytes).map_err(|_| ObjectError::RepresentationBudget)?;
    Ok(ByteCount::new(bytes)?)
}

pub(super) fn observed_length(retained: &ReadResult) -> u64 {
    match retained.extent() {
        ReadExtent::Complete => retained.bytes().len() as u64,
        ReadExtent::Truncated { observed_size, .. } => observed_size,
    }
}
