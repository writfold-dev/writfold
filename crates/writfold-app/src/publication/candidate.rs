//! A prepared candidate: the ordered record changes one approval publishes,
//! with canonical bytes (retained as a Git blob) and their SHA-256 digest.
use super::PublicationError;
use sha2::{Digest, Sha256};
use writfold_domain::ContentDigest;
use writfold_store::backend::{HeadName, RecordChange, WriteRequest};

const DOMAIN: &[u8] = b"writfold-dolt-candidate-v1\0";
/// The one conditional head the publication advances.
pub(super) const HEAD: &str = "main";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    changes: Vec<RecordChange>,
    bytes: Vec<u8>,
    digest: ContentDigest,
}

impl Candidate {
    /// Canonical bytes: the domain, then each change as its length-prefixed
    /// key, kind and either `1` plus length-prefixed content or `0` (delete).
    pub fn new(changes: Vec<RecordChange>) -> Result<Self, PublicationError> {
        let mut bytes = DOMAIN.to_vec();
        for change in &changes {
            framed(&mut bytes, change.key.as_str().as_bytes());
            framed(&mut bytes, change.kind.as_str().as_bytes());
            match &change.content {
                Some(content) => {
                    bytes.push(1);
                    framed(&mut bytes, content);
                }
                None => bytes.push(0),
            }
        }
        let digest = digest(&bytes)?;
        Ok(Self {
            changes,
            bytes,
            digest,
        })
    }

    pub fn changes(&self) -> &[RecordChange] {
        &self.changes
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn digest(&self) -> &ContentDigest {
        &self.digest
    }

    /// The storage request this candidate makes from `base`. Its content is
    /// the write's identity, so `observe` finds exactly this request.
    pub fn request<P: Clone>(&self, base: &P) -> Result<WriteRequest<P>, PublicationError> {
        Ok(WriteRequest {
            head: HeadName::new(HEAD.to_owned())?,
            expected_old: base.clone(),
            changes: self.changes.clone(),
        })
    }
}

/// SHA-256 of retained candidate bytes, as lowercase hex.
pub(super) fn digest(bytes: &[u8]) -> Result<ContentDigest, PublicationError> {
    let mut text = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 15)]));
    }
    text.parse()
        .map_err(|_| PublicationError::Manifest("candidate digest"))
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn framed(bytes: &mut Vec<u8>, field: &[u8]) {
    bytes.extend_from_slice(&(field.len() as u64).to_be_bytes());
    bytes.extend_from_slice(field);
}
