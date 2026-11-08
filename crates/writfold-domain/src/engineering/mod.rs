//! Domain-separated engineering fingerprints, not payload or authority validation.
//!
//! Raw digest construction does not validate a payload, schema, signature, or receipt.
//! Historical identifier namespaces are unchanged.
//!
//! Nominal types cannot substitute for one another:
//! ```compile_fail
//! use writfold_domain::engineering::{BlobDigest, EngineeringDigest, ReceiptDigest};
//! fn accept_receipt(_: ReceiptDigest) {}
//! accept_receipt(BlobDigest::from_digest([0; 32]));
//! ```
//! The bytes are private, including for consumers implementing their own types:
//! ```compile_fail
//! use writfold_domain::engineering::BlobDigest;
//! let digest = BlobDigest([0; 32]);
//! ```
//! The trait is sealed; callers cannot invent a new fingerprint domain:
//! ```compile_fail
//! use writfold_domain::engineering::{EngineeringDigest, FingerprintKind};
//! struct Forged([u8; 32]);
//! impl EngineeringDigest for Forged {
//!     const KIND: FingerprintKind = FingerprintKind::Blob;
//!     fn from_digest(bytes: [u8; 32]) -> Self { Self(bytes) }
//!     fn as_digest(&self) -> &[u8; 32] { &self.0 }
//! }
//! ```

mod codec;
pub mod dependency;
mod digests;
pub mod execution;
mod kinds;
pub mod ledger;
pub mod objects;
pub mod procedure;

pub use digests::*;
pub use kinds::FingerprintKind;

use std::{error::Error, fmt};

vstd::prelude::verus! {
/// Invalid exact fingerprint spelling or unknown closed fingerprint kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FingerprintError;
}

impl fmt::Display for FingerprintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid engineering fingerprint")
    }
}
impl Error for FingerprintError {}

mod sealed {
    vstd::prelude::verus! {
    pub trait Sealed {}
    }
}

vstd::prelude::verus! {
/// A supplied SHA-256 digest in one closed engineering domain.
///
/// Construction wraps bytes only: it does not hash or validate a payload.
pub trait EngineeringDigest: sealed::Sealed + Sized {
    const KIND: FingerprintKind;
    fn from_digest(bytes: [u8; 32]) -> Self;
    fn as_digest(&self) -> &[u8; 32];
}
}
