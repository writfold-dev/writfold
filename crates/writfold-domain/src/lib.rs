//! Checked values the runtime and the KB pass between them: Git object
//! IDs, content digests, byte counts and the revision identities of
//! authored material. Parsing never trims, folds case or normalizes
//! Unicode. Plain Rust, tested; the Verus formalization this crate once
//! carried was retired on 21 November 2026 (#54) and is archived with the
//! pre-cleanup tree (`writfold-archive`, `public-pre-cleanup-2026-11-21`).
mod ids;
pub mod text;
mod units;

pub use ids::{
    ContentDigest, DesignRevision, GitObjectFormat, GitObjectId, IdentityError,
    ProposalMaterialRevision,
};
pub use units::{ByteCount, QuantityError};
