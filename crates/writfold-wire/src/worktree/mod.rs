//! Closed bounded worktree/1 descriptors; no RPC or trusted native capabilities.
mod decode;
mod encode;
mod json;
use crate::{Limits, engineering::cbor};
use sha2::{Digest, Sha256};
use writfold_domain::worktree::*;
pub const PROFILE: &str = "writfold-worktree/1";
pub const ALGORITHM: &str = "sha256";
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Json(#[from] crate::Error),
    #[error(transparent)]
    Cbor(#[from] cbor::Error),
    #[error(transparent)]
    Domain(#[from] InvalidWorktree),
    #[error("invalid worktree schema or cross-reference")]
    Invalid,
}
pub type Result<T, E = Error> = std::result::Result<T, E>;
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Record {
    Recipe(Recipe),
    Capture(Capture),
    Candidate(Candidate),
    PreparedEdit(PreparedEdit),
    RequestKey(RequestKey),
    Observation(EffectObservation),
}
impl Record {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Recipe(_) => "recipe",
            Self::Capture(_) => "capture",
            Self::Candidate(_) => "candidate",
            Self::PreparedEdit(_) => "prepared-edit",
            Self::RequestKey(_) => "request-key",
            Self::Observation(_) => "observation",
        }
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self> {
        decode::record(bytes, limits)
    }
    pub fn encode_json(&self, limits: Limits) -> Result<Vec<u8>> {
        json::record(self, limits)
    }
    pub fn canonical_bytes(&self, limits: Limits) -> Result<Vec<u8>> {
        encode::record(self, limits)
    }
    pub fn identity(&self, limits: Limits) -> Result<String> {
        let bytes = self.canonical_bytes(limits)?;
        Ok(format!(
            "wfwt1:{}:sha256:{}",
            self.kind(),
            hex(&hash(self.kind(), &bytes))
        ))
    }
}
pub fn raw_sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn hash(kind: &str, bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"writfold-worktree/1/");
    hash.update(kind.as_bytes());
    hash.update(b"\0");
    hash.update(bytes);
    hash.finalize().into()
}
pub(super) fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 15) as usize] as char);
    }
    text
}
macro_rules! digests {
    ($($fun:ident, $ty:ty, $variant:ident, $digest:ty;)+) => {$(
        pub fn $fun(value: &$ty, limits: Limits) -> Result<$digest> {
            let bytes = encode::$fun(value, limits)?;
            Ok(<$digest>::from_digest(hash(<$digest>::KIND, &bytes)))
        }
    )+};
}
digests! {
    recipe_digest, Recipe, Recipe, RecipeDigest;
    capture_digest, Capture, Capture, CaptureDigest;
    candidate_digest, Candidate, Candidate, CandidateDigest;
    prepared_edit_digest, PreparedEdit, PreparedEdit, PreparedEditDigest;
    request_key_digest, RequestKey, RequestKey, RequestKeyDigest;
    observation_digest, EffectObservation, Observation, ObservationDigest;
}
/// Validate exact identities, realm/run scope and byte derivation, not compiler correctness.
pub fn validate_prepared(
    prepared: &PreparedEdit,
    recipe: &Recipe,
    capture: &Capture,
    candidate: &Candidate,
    expected_scope: &Scope,
    limits: Limits,
) -> Result<()> {
    let capture_id = capture_digest(capture, limits)?;
    if capture.scope() != expected_scope
        || prepared.recipe() != recipe_digest(recipe, limits)?
        || prepared.capture() != capture_id
        || candidate.capture() != capture_id
        || prepared.candidate() != candidate_digest(candidate, limits)?
    {
        return Err(Error::Invalid);
    }
    prepared.validate_origins(recipe.source())?;
    prepared.validate_bytes(capture.content(), candidate.content())?;
    Ok(())
}
/// Data consistency only. A host must separately establish native evidence and authority.
pub fn validate_observation(
    observation: &EffectObservation,
    intent: writfold_domain::engineering::EffectIntentDigest,
    stamp: writfold_domain::engineering::DispatchStampDigest,
    prepared: &PreparedEdit,
    capture: &Capture,
    candidate: &Candidate,
    limits: Limits,
) -> Result<()> {
    if observation.intent() != intent
        || observation.stamp() != stamp
        || observation.prepared() != prepared_edit_digest(prepared, limits)?
        || prepared.capture() != capture_digest(capture, limits)?
        || candidate.capture() != prepared.capture()
        || prepared.candidate() != candidate_digest(candidate, limits)?
    {
        return Err(Error::Invalid);
    }
    prepared.validate_bytes(capture.content(), candidate.content())?;
    let consistent = match observation.content() {
        ContentObservation::Original(hash) => *hash == raw_sha256(capture.content()),
        ContentObservation::Candidate(hash) => *hash == raw_sha256(candidate.content()),
        ContentObservation::Other(hash) => {
            *hash != raw_sha256(capture.content()) && *hash != raw_sha256(candidate.content())
        }
        ContentObservation::Unobserved => true,
    };
    if consistent {
        Ok(())
    } else {
        Err(Error::Invalid)
    }
}
