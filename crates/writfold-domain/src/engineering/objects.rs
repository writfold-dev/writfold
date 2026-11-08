//! Structurally typed retained-object metadata, not truth or live authority.
use super::BlobDigest;
use crate::{ByteCount, InstanceId, KbId, RealmId, records::record};
use vstd::prelude::*;

record!(
    /// A supplied local realm identity. The wire profile and private filesystem
    /// binding must be validated separately; this does not make a realm ready.
    RealmIdentity, RealmIdentityView {
        realm: RealmId,
        kb: KbId,
        instance: InstanceId,
    }
);

verus! {
/// A realm-qualified content assertion. Only a verified store observation can
/// establish availability of the exact bytes; this value conveys no permission.
#[derive(Debug)]
pub struct BlobReference {
    realm: RealmId,
    digest: BlobDigest,
    bytes: ByteCount,
}

#[cfg(verus_keep_ghost)]
pub ghost struct BlobReferenceView {
    pub realm: Seq<char>,
    pub digest: Seq<u8>,
    pub bytes: nat,
}

#[cfg(verus_keep_ghost)]
impl View for BlobReference {
    type V = BlobReferenceView;
    closed spec fn view(&self) -> Self::V {
        BlobReferenceView { realm: self.realm@, digest: self.digest@, bytes: self.bytes@ }
    }
}

impl BlobReference {
    pub fn new(realm: RealmId, digest: BlobDigest, bytes: ByteCount) -> (reference: Self)
        ensures reference@ == (BlobReferenceView { realm: realm@, digest: digest@, bytes: bytes@ }),
    {
        Self { realm, digest, bytes }
    }

    pub fn realm(&self) -> (realm: &RealmId)
        ensures realm@ == self@.realm,
    {
        &self.realm
    }

    pub fn digest(&self) -> (digest: &BlobDigest)
        ensures digest@ == self@.digest,
    {
        &self.digest
    }

    pub fn bytes(&self) -> (bytes: &ByteCount)
        ensures bytes@ == self@.bytes,
    {
        &self.bytes
    }
}
}
