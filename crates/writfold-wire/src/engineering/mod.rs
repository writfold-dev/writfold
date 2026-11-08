//! Engineering/1 byte boundaries, separate from historical signed JSON.
//!
//! Canonical encoding and a typed digest establish neither schema validity nor
//! authenticity, authority, complete input capture, or permission to reuse evidence.

pub mod cbor;
pub mod dependency;
pub mod edit_approval;
pub mod grant;
pub mod ledger;
pub mod objects;

use sha2::{Digest, Sha256};
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};

/// Hash already-validated canonical payload bytes in the selected nominal domain.
/// The preimage is streamed; neither the payload nor its prefix is concatenated.
pub fn fingerprint<D: EngineeringDigest>(payload: &cbor::RawCbor<'_>) -> D {
    let mut hash = domain_hash::<D>();
    hash.update(payload.as_bytes());
    D::from_digest(hash.finalize().into())
}

/// Hash raw retained bytes as the canonical engineering byte-string payload.
/// This streams the fixed domain, shortest CBOR header and original bytes;
/// it never allocates or copies an artifact-sized encoded value.
pub fn blob_fingerprint(bytes: &[u8]) -> BlobDigest {
    let mut hash = domain_hash::<BlobDigest>();
    let mut header = [0_u8; 9];
    let length = cbor::argument_header(2, bytes.len() as u64, &mut header);
    hash.update(&header[..length]);
    hash.update(bytes);
    BlobDigest::from_digest(hash.finalize().into())
}

fn domain_hash<D: EngineeringDigest>() -> Sha256 {
    let mut hash = Sha256::new();
    hash.update(b"writfold-engineering/1/");
    hash.update(D::KIND.as_str().as_bytes());
    hash.update(b"\0");
    hash
}
