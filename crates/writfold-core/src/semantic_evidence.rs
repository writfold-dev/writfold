//! The evidence boundary between retrieval pointers and independently checked sources.
//!
//! A lexical or semantic match locates something to inspect; it is not a citation
//! verdict, gate receipt, or checklist support. Only an independently checked
//! source preserves the grounding supplied by its existing checker. In particular,
//! finding a source semantically does not prevent a later, independently checked
//! citation to that source from grounding evidence.
//!
//! Scores, ranks and intended use are deliberately not inputs: none can promote a
//! retrieval pointer. `checked` is a caller-supplied prerequisite, not a claim that
//! this pure function verifies source bytes, freshness, receipts, or provenance.
//! Extract only `grounding`, not the derived implementations. The Lean statements
//! are in `proofs/Writfold/SemanticEvidence.lean`.
use crate::assess::Grounding;

/// How the candidate reached an evidence acceptance boundary, not how a human
/// originally discovered the underlying source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Origin {
    CheckedSource,
    LexicalRetrieval,
    SemanticRetrieval,
}

/// Keep the actual source check, or refuse an unchecked retrieval pointer.
/// Attested evidence remains attested; stale/failed checks remain ungrounded.
pub fn grounding(origin: Origin, checked: Grounding) -> Grounding {
    match origin {
        Origin::CheckedSource => checked,
        Origin::LexicalRetrieval => Grounding::Ungrounded,
        Origin::SemanticRetrieval => Grounding::Ungrounded,
    }
}

#[cfg(test)]
mod tests {
    use super::{Origin, grounding};
    use crate::assess::Grounding;

    #[test]
    fn checked_source_preserves_the_real_prerequisite() {
        for checked in [
            Grounding::Grounded,
            Grounding::Attested,
            Grounding::Ungrounded,
        ] {
            assert_eq!(grounding(Origin::CheckedSource, checked), checked);
        }
    }

    #[test]
    fn retrieval_cannot_promote_even_a_grounded_claim() {
        for origin in [Origin::LexicalRetrieval, Origin::SemanticRetrieval] {
            for checked in [
                Grounding::Grounded,
                Grounding::Attested,
                Grounding::Ungrounded,
            ] {
                assert_eq!(grounding(origin, checked), Grounding::Ungrounded);
            }
        }
    }
}
