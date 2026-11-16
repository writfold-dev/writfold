//! Provenance at the evidence acceptance boundary, independent of source syntax.
use writfold_core::assess::Grounding;
use writfold_core::semantic_evidence::{self, Origin};

/// A retrieval pointer is not a request to independently check its source.
/// Checked references use the existing citation/receipt grammar unchanged.
#[derive(Clone, Copy, Debug)]
pub enum EvidenceInput<'a> {
    CheckedReference(&'a str),
    #[cfg(feature = "semantic")]
    LexicalPointer(&'a str),
    #[cfg(feature = "semantic")]
    SemanticPointer(&'a str),
}

impl EvidenceInput<'_> {
    /// The proved decision consumed by both evidence acceptance and retrieval
    /// output. Scores and labels cannot upgrade a raw pointer's grounding.
    pub fn grounding(self, checked: Grounding) -> Grounding {
        let origin = match self {
            Self::CheckedReference(_) => Origin::CheckedSource,
            #[cfg(feature = "semantic")]
            Self::LexicalPointer(_) => Origin::LexicalRetrieval,
            #[cfg(feature = "semantic")]
            Self::SemanticPointer(_) => Origin::SemanticRetrieval,
        };
        semantic_evidence::grounding(origin, checked)
    }
}
