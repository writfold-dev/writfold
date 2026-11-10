//! Pure replay of a literal-path recipe against host-retained bytes, never effect authority.
//!
//! The host must compare `source_sha256` with its retained capture and revalidate
//! that capture before any separately authorized effect. Replay cannot establish
//! filesystem freshness and does not resolve interactive handles or frozen sets.

use sha2::{Digest, Sha256};
use splicery_domain::{OriginalImage, Plan, Profile};
use splicery_kernel::{Candidate, EditReport, PreparedBatch};
use splicery_syntax::ast::Recipe;

/// Identity of the embedded product-source export, not of a CLI executable.
#[derive(Debug, PartialEq, Eq)]
pub struct ReplayIdentity {
    commit: &'static str,
    source_artifact_sha256: &'static str,
    profile: &'static str,
}

impl ReplayIdentity {
    pub fn commit(&self) -> &str {
        self.commit
    }

    pub fn source_artifact_sha256(&self) -> &str {
        self.source_artifact_sha256
    }

    pub fn profile(&self) -> &str {
        self.profile
    }
}

static IDENTITY: ReplayIdentity = ReplayIdentity {
    commit: "b14c67a7aca9f416c4facdf189ef340d5666351f",
    source_artifact_sha256: "b4c3574a1874f8441b871b1b901eb5ef51f8829a760237adfd1b2df11a1827e0",
    profile: "writfold-splicery-literal-edit/1",
};

/// Parser, lowering and kernel refusals remain distinguishable and attributed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReplayError {
    #[error("path must be a nonempty canonical relative path")]
    InvalidPath,
    #[error("recipe literal path does not equal the host path")]
    WrongPath,
    #[error("create, rename and remove recipes are not edit replay")]
    UnsupportedFileOperation,
    #[error("source exceeds the pinned input limit or is invalid for its text profile")]
    InvalidSource,
    #[error("recipe parsing refused: {0:?}")]
    Parse(splicery_syntax::Diagnostic),
    #[error("recipe lowering refused: {0:?}")]
    Lower(splicery_compile::CompileDiagnostic),
    #[error("kernel preparation or rendering refused: {0:?}")]
    Kernel(splicery_kernel::KernelError),
}

/// A preparation result only. Construction is restricted to pinned replay.
#[derive(Debug)]
pub struct ReplayedEdit {
    candidate: Candidate,
    relative_path: String,
    source_sha256: [u8; 32],
    candidate_sha256: [u8; 32],
    recipe_sha256: [u8; 32],
    source_profile: Profile,
}

impl ReplayedEdit {
    pub fn candidate(&self) -> &[u8] {
        self.candidate.bytes()
    }

    /// Sorted, original-byte-coordinate edits with exact recipe-origin spans.
    pub fn edits(&self) -> &[EditReport] {
        self.candidate.edits()
    }

    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn source_sha256(&self) -> &[u8; 32] {
        &self.source_sha256
    }

    pub fn candidate_sha256(&self) -> &[u8; 32] {
        &self.candidate_sha256
    }

    pub fn recipe_sha256(&self) -> &[u8; 32] {
        &self.recipe_sha256
    }

    pub fn source_profile(&self) -> &Profile {
        &self.source_profile
    }

    pub fn identity(&self) -> &ReplayIdentity {
        &IDENTITY
    }

    pub fn no_change(&self) -> bool {
        self.candidate.no_change()
    }

    pub fn into_candidate(self) -> Vec<u8> {
        self.candidate.into_bytes()
    }
}

/// Replay exact recipe bytes without IO, aliases, capture discovery or authority.
pub fn replay(
    source: Vec<u8>,
    expected_relative_path: &str,
    recipe: &[u8],
) -> Result<ReplayedEdit, ReplayError> {
    validate_path(expected_relative_path)?;
    // No ambient path: only an explicit literal-path recipe is accepted.
    let parsed = splicery_syntax::parse_recipe(recipe, None).map_err(ReplayError::Parse)?;
    let Recipe::Edit { path, raw, .. } = &parsed else {
        return Err(ReplayError::UnsupportedFileOperation);
    };
    validate_path(path)?;
    if path != expected_relative_path {
        return Err(ReplayError::WrongPath);
    }
    let source_profile = if *raw { Profile::Raw } else { Profile::Text };
    let image = OriginalImage::new(source, source_profile).ok_or(ReplayError::InvalidSource)?;
    let source_sha256 = image.capture_sha256();
    let plan = splicery_compile::lower(&parsed, &image, &[], &[]).map_err(ReplayError::Lower)?;
    let Plan::Edit(splices) = plan else {
        return Err(ReplayError::UnsupportedFileOperation);
    };
    let batch = PreparedBatch::new(image, splices).map_err(ReplayError::Kernel)?;
    let candidate = splicery_kernel::render(&batch).map_err(ReplayError::Kernel)?;
    Ok(ReplayedEdit {
        candidate_sha256: Sha256::digest(candidate.bytes()).into(),
        candidate,
        relative_path: expected_relative_path.to_owned(),
        source_sha256,
        recipe_sha256: Sha256::digest(recipe).into(),
        source_profile,
    })
}

fn validate_path(path: &str) -> Result<(), ReplayError> {
    if path.contains(['\\', ':', '\0'])
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(ReplayError::InvalidPath);
    }
    Ok(())
}
