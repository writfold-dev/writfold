//! Engineering/1 byte boundaries, separate from historical signed JSON.
//!
//! Canonical encoding and a typed digest establish neither schema validity nor
//! authenticity, authority, complete input capture, or permission to reuse evidence.

pub mod cbor;

use sha2::{Digest, Sha256};
use writfold_domain::engineering::EngineeringDigest;

/// Hash already-validated canonical payload bytes in the selected nominal domain.
/// The preimage is streamed; neither the payload nor its prefix is concatenated.
pub fn fingerprint<D: EngineeringDigest>(payload: &cbor::RawCbor<'_>) -> D {
    let mut hash = Sha256::new();
    hash.update(b"writfold-engineering/1/");
    hash.update(D::KIND.as_str().as_bytes());
    hash.update(b"\0");
    hash.update(payload.as_bytes());
    D::from_digest(hash.finalize().into())
}
