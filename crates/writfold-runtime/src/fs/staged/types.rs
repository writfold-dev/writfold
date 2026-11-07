use super::super::FsError;
use thiserror::Error;
use writfold_domain::ContentDigest;

/// Plain data for the owner's durable encoding; never an authorization token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StageIdentity {
    pub device: u64,
    pub inode: u64,
}

/// Exact preimage metadata. Access time is intentionally excluded: reads change it.
/// Only ordinary permission bits are admitted; ACLs/xattrs and special bits are refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StageMetadata {
    pub identity: StageIdentity,
    /// Full native regular-file mode, including the file type bits.
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub links: u64,
    pub length: u64,
    pub modified_seconds: i64,
    pub modified_nanoseconds: u64,
    pub changed_seconds: i64,
    pub changed_nanoseconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StageImage {
    pub metadata: StageMetadata,
    pub bytes: Vec<u8>,
    pub digest: ContentDigest,
}

/// Persist with the original effect/intent and dispatch claim, before installation.
/// Fields are public to permit lossless owner-side encoding without runtime serde.
/// Rehydration is data, not a grant. Only trusted, intact owner records are admitted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StageWitness {
    pub parent: StageIdentity,
    pub lock: StageIdentity,
    pub target_name: Vec<u8>,
    pub stage_name: String,
    pub preimage: StageImage,
    pub candidate: StageImage,
}

/// An exclusive stage can remain even when filling or synchronization fails.
/// No destructor or later operation scans/removes these attempts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedStage {
    pub parent: StageIdentity,
    pub name: String,
    pub identity: Option<StageIdentity>,
}

#[derive(Debug, Error)]
pub enum StageError {
    #[error(transparent)]
    Filesystem(#[from] FsError),
    #[error("ACLs, extended attributes, or special mode/ownership configurations are unsupported")]
    UnsupportedMetadata,
    #[error("the exact captured preimage, stage, or stable binding changed")]
    Conflict,
    #[error("the persisted stage witness is inconsistent")]
    InvalidWitness,
}

#[derive(Debug, Error)]
#[error("replacement staging failed: {source}")]
pub struct StageFailure {
    #[source]
    pub source: StageError,
    pub retained: Option<RetainedStage>,
}

impl From<StageError> for StageFailure {
    fn from(source: StageError) -> Self {
        Self {
            source,
            retained: None,
        }
    }
}

/// A failed install never implies that the stage was removed.
#[derive(Debug, Error)]
pub enum InstallError {
    #[error("replacement was not issued: {0}")]
    NotInstalled(#[source] StageError),
    #[error("replacement was issued; visibility or durability is unknown: {0}")]
    OutcomeUnknown(#[source] StageError),
}

/// Native observations only, not an effect receipt or hardware power-loss guarantee.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstalledStage {
    pub identity: StageIdentity,
}

#[derive(Debug)]
pub enum StageReconciliation {
    InstalledObservedSync(InstalledStage),
    /// Both exact preimage and exact stage were observed; only that stage was
    /// unlinked, then its parent synchronized. Caller must retain this result.
    NotInstalledCleaned,
    Unknown(StageError),
}
