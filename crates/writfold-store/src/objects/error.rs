use thiserror::Error;
use writfold_domain::{ByteCount, QuantityError};
use writfold_runtime::fs::{FsError, WriteError};

#[derive(Debug, Error)]
pub enum ObjectError {
    #[error(transparent)]
    Filesystem(#[from] FsError),
    /// In particular, `PublishedButDurabilityUnknown` is never converted into a
    /// successful reference or silently retried. Its digest is a raw FS digest,
    /// not an engineering BlobDigest.
    #[error(transparent)]
    Write(#[from] WriteError),
    #[error(transparent)]
    IdentityEncoding(#[from] writfold_wire::engineering::objects::ObjectError),
    #[error(transparent)]
    Quantity(#[from] QuantityError),
    #[error("the retained identity or reference belongs to a different realm")]
    WrongRealm,
    #[error("the bound realm identity bytes or file incarnation changed")]
    IdentityChanged,
    #[error("the object length is {observed}, expected {expected:?}")]
    ObjectLength { expected: ByteCount, observed: u64 },
    #[error("the retained bytes do not have the qualified engineering blob digest")]
    ObjectDigest,
    #[error("the existing object differs from the exact original put bytes")]
    ObjectBytes,
    #[error("byte budget {limit:?} cannot retain {required} bytes")]
    Budget { limit: ByteCount, required: u64 },
    #[error("the representation budget cannot be expressed on this host")]
    RepresentationBudget,
}
