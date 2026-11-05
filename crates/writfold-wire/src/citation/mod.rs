//! Versioned durable citation material, not live authorization or an admitted claim.
//! The public descriptor retains exact native algorithms and metadata; compact
//! aliases and request generations are deliberately absent from its identity.
mod codec;
mod fingerprint;
mod json;

pub use codec::{decode_descriptor, encode_descriptor};
pub use fingerprint::{
    binding_fingerprint, correspondence_payload_fingerprint, representation_fingerprint,
    selection_digest,
};
use sha2::{Digest, Sha256};
use writfold_domain::{
    CitationId, ContentDigest, ContentId, ContextId, EvidenceRoleId, GitObjectFormat, GitObjectId,
    RealmId, SnapshotId, SourceId, SourceRevision,
    citation::{Binding, CoordinateUnit, RepresentationProfile, RepresentationRef, Selector},
};

pub const NAMESPACE: &str = "writfold-citation";
pub const SCHEMA: u64 = 1;
pub const GIT_BLOB_PROFILE: &str = "git-blob/1";

#[derive(Debug, thiserror::Error)]
pub enum CitationWireError {
    #[error(transparent)]
    Wire(#[from] crate::Error),
    #[error(transparent)]
    Cbor(#[from] crate::engineering::cbor::Error),
    #[error("citation record has an invalid field set or root shape")]
    Shape,
    #[error("invalid citation field {0}")]
    Field(&'static str),
    #[error("invalid citation identity in {0}")]
    Identity(&'static str),
    #[error("unsupported citation namespace")]
    UnsupportedNamespace,
    #[error("unsupported citation schema")]
    UnsupportedSchema,
    #[error("unsupported citation record kind")]
    UnsupportedKind,
    #[error("unsupported citation representation or coordinate profile")]
    UnsupportedProfile,
    #[error("Git commit and blob formats differ")]
    ObjectFormat,
    #[error("citation path is not an exact repository-relative UTF-8 path")]
    Path,
    #[error("expected selection preimage has the wrong byte length")]
    SelectionLength,
    #[error("expected selection preimage differs from its retained digest")]
    SelectionDigest,
    #[error("citation output allocation failed")]
    Allocation,
    #[error("citation JSON encoding failed")]
    Encoding,
}

type Result<T, E = CitationWireError> = std::result::Result<T, E>;

/// Checked native identities and literal path syntax, not proof that Git contains
/// this source or that the caller may read it. Native admission remains separate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitSource {
    reference: RepresentationRef,
    blob: GitObjectId,
}

impl GitSource {
    pub fn new(
        realm: RealmId,
        repository: SourceId,
        revision: GitObjectId,
        path: String,
        blob: GitObjectId,
        capture_digest: ContentDigest,
    ) -> Result<Self> {
        if revision.format() != blob.format() {
            return Err(CitationWireError::ObjectFormat);
        }
        if !valid_path(&path) {
            return Err(CitationWireError::Path);
        }
        let revision = SourceRevision::try_from(revision.into_string())
            .map_err(|_| CitationWireError::Identity("revision"))?;
        // Preserve SourceCapture::content_identity: semantic content names the
        // retained raw SHA-256 preimage, never the native Git blob identifier.
        let content = ContentId::try_from(capture_digest.into_string())
            .map_err(|_| CitationWireError::Identity("capture_digest"))?;
        Ok(Self {
            reference: RepresentationRef::new(
                realm,
                repository,
                revision,
                path,
                content,
                RepresentationProfile::GitBlob,
            ),
            blob,
        })
    }

    pub fn reference(&self) -> &RepresentationRef {
        &self.reference
    }
    pub fn format(&self) -> GitObjectFormat {
        self.blob.format()
    }
    pub fn blob(&self) -> &GitObjectId {
        &self.blob
    }
}

/// Owned immutable wire material. A digest is an input binding, not an approval.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CitationDescriptor {
    id: CitationId,
    release: SnapshotId,
    context: ContextId,
    role: EvidenceRoleId,
    source: GitSource,
    selector: Selector,
    selection_digest: ContentDigest,
}

impl CitationDescriptor {
    pub fn new(
        id: CitationId,
        release: SnapshotId,
        context: ContextId,
        role: EvidenceRoleId,
        source: GitSource,
        selector: Selector,
        selection_digest: ContentDigest,
    ) -> Result<Self> {
        if selector.unit() != CoordinateUnit::Byte {
            return Err(CitationWireError::UnsupportedProfile);
        }
        Ok(Self {
            id,
            release,
            context,
            role,
            source,
            selector,
            selection_digest,
        })
    }

    pub fn id(&self) -> &CitationId {
        &self.id
    }
    pub fn release(&self) -> &SnapshotId {
        &self.release
    }
    pub fn context(&self) -> &ContextId {
        &self.context
    }
    pub fn role(&self) -> &EvidenceRoleId {
        &self.role
    }
    pub fn source(&self) -> &GitSource {
        &self.source
    }
    pub fn selector(&self) -> Selector {
        self.selector
    }
    pub fn selection_digest(&self) -> &ContentDigest {
        &self.selection_digest
    }

    /// Rehydrate the exact retained preimage without copying it. Availability,
    /// full representation integrity and current access are still checked by the
    /// source host and kernel. This does not refresh an observation or receipt.
    pub fn bind<'a>(&'a self, expected: &'a [u8]) -> Result<Binding<'a>> {
        let width = self.selector.last().get() - self.selector.first().get();
        if expected.len() as u128 != u128::from(width) {
            return Err(CitationWireError::SelectionLength);
        }
        let digest = Sha256::digest(expected);
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let exact = digest
            .iter()
            .zip(self.selection_digest.as_str().as_bytes().as_chunks::<2>().0)
            .all(|(byte, encoded)| {
                *encoded == [HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 15)]]
            });
        if !exact {
            return Err(CitationWireError::SelectionDigest);
        }
        Ok(Binding::new(
            &self.id,
            &self.release,
            &self.context,
            &self.role,
            &self.source.reference,
            self.selector,
            expected,
        ))
    }
}

// Same literal descendant grammar as runtime::fs::RelativePath, applied at this
// UTF-8 wire boundary. No canonicalization or path-based access authorization.
fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && !path.as_bytes().contains(&0)
        && path
            .as_bytes()
            .split(|byte| *byte == b'/')
            .all(|part| !part.is_empty() && part != b"." && part != b"..")
}

fn algorithm(format: GitObjectFormat) -> &'static str {
    match format {
        GitObjectFormat::Sha1 => "sha1",
        GitObjectFormat::Sha256 => "sha256",
    }
}
