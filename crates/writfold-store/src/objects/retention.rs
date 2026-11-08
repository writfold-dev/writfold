use super::identity::{byte_count, observed_length, private_read, relative};
use super::{ObjectError, ObjectStore};
use writfold_domain::ByteCount;
use writfold_domain::engineering::objects::BlobReference;
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};
use writfold_runtime::fs::{ContainedRoot, FsError, LockKey, ReadExtent, ReadResult};
use writfold_wire::engineering::blob_fingerprint;

impl ObjectStore {
    /// Retain the exact original bytes or verify and synchronize an identical
    /// existing object in place. There are no retries, repairs or replacements.
    /// A returned reference records actual realm/digest/length, not effect rights.
    pub fn put(&self, content: &[u8], budget: ByteCount) -> Result<BlobReference, ObjectError> {
        let bytes = byte_count(content.len())?;
        check_budget(bytes, budget)?;
        self.check_context()?;
        let digest = blob_fingerprint(content);
        let hex = digest_hex(&digest);
        let shard = self.ensure_shard(&hex[..2])?;
        let target = shard.pin(&relative(&hex[2..])?)?;
        let key = LockKey::new(&format!("blob-{hex}"))?;
        let mut guard = target.try_lock_private(&self.locks, &key)?;
        self.check_context()?;
        shard.check_private()?;
        let receipt = match target.check_private() {
            Ok(identity) => {
                let retained = guard.read(bytes)?;
                if retained.identity() != identity || target.check_private()? != identity {
                    return Err(FsError::ChangedSource.into());
                }
                verify_length(&retained, bytes)?;
                // Exact equality binds these bytes to the fingerprint of content;
                // do not rehash or replace this comparison with hash equality.
                if retained.bytes() != content {
                    return Err(ObjectError::ObjectBytes);
                }
                // A previous failed publication may have left visible bytes. A
                // duplicate put must establish current durability, not assume it.
                guard.sync_existing(&retained)?
            }
            Err(FsError::Missing) => guard.create(content, budget)?,
            Err(error) => return Err(error.into()),
        };
        if receipt.bytes() != content.len() || target.check_private()? != receipt.identity() {
            return Err(FsError::ChangedSource.into());
        }
        guard.check_current()?;
        shard.check_private()?;
        self.check_context()?;
        Ok(BlobReference::new(
            self.identity.realm().clone(),
            digest,
            bytes,
        ))
    }

    /// Read an entire qualified object, never a truncated success or substituted
    /// HEAD. This path never creates or synchronizes lock entries or directories.
    pub fn get(
        &self,
        reference: &BlobReference,
        budget: ByteCount,
    ) -> Result<Vec<u8>, ObjectError> {
        if reference.realm() != self.identity.realm() {
            return Err(ObjectError::WrongRealm);
        }
        check_budget(*reference.bytes(), budget)?;
        self.check_context()?;
        let hex = digest_hex(reference.digest());
        let shard = self.hashes.open_child(&relative(&hex[..2])?)?;
        shard.check_private()?;
        let target = shard.pin(&relative(&hex[2..])?)?;
        let retained = private_read(&target, *reference.bytes())?;
        verify(&retained, reference.digest(), *reference.bytes())?;
        shard.check_private()?;
        self.check_context()?;
        Ok(retained.into_bytes())
    }

    fn ensure_shard(&self, prefix: &str) -> Result<ContainedRoot, ObjectError> {
        let guard = self
            .identity_file
            .try_lock_private(&self.locks, &LockKey::new("realm-identity")?)?;
        let identity = self.check_context()?;
        // Reopen can observe identity bytes whose first publication returned
        // durability-unknown. Explicit retention settles that binding under the
        // same key as initialization; open/get remain read-only.
        guard.sync_existing(&identity)?;
        let path = relative(prefix)?;
        let shard = match self.hashes.open_child(&path) {
            Ok(shard) => {
                shard.check_private()?;
                // Explicit put also settles a directory left by failed creation.
                shard.sync()?;
                shard
            }
            Err(FsError::Missing) => self.hashes.create_child(&path)?,
            Err(error) => return Err(error.into()),
        };
        shard.check_private()?;
        guard.check_current()?;
        self.check_context()?;
        Ok(shard)
    }
}

fn check_budget(required: ByteCount, limit: ByteCount) -> Result<(), ObjectError> {
    if required > limit {
        return Err(ObjectError::Budget {
            limit,
            required: required.get(),
        });
    }
    Ok(())
}

fn verify(retained: &ReadResult, digest: &BlobDigest, bytes: ByteCount) -> Result<(), ObjectError> {
    verify_length(retained, bytes)?;
    // The runtime digest is raw SHA-256. Only the engineering domain-separated
    // fingerprint qualifies this object path and reference.
    if blob_fingerprint(retained.bytes()) != *digest {
        return Err(ObjectError::ObjectDigest);
    }
    Ok(())
}

fn verify_length(retained: &ReadResult, bytes: ByteCount) -> Result<(), ObjectError> {
    let observed = observed_length(retained);
    if retained.extent() != ReadExtent::Complete || observed != bytes.get() {
        return Err(ObjectError::ObjectLength {
            expected: bytes,
            observed,
        });
    }
    Ok(())
}

fn digest_hex(digest: &BlobDigest) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hex = String::with_capacity(64);
    for byte in digest.as_digest() {
        hex.push(char::from(HEX[usize::from(byte >> 4)]));
        hex.push(char::from(HEX[usize::from(byte & 15)]));
    }
    hex
}
