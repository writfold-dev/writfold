//! What a cited reference stands on now: grounded, attested or not. The
//! proved kernels decide with it (`ground`, `checklist`); its names are the
//! words receipts and rows use. Moved out of the retired `assess` kernel
//! (21 November 2026, #54); not itself translated.

/// What Writfold found for one cited piece of evidence, or for one
/// assumption once all its citations are read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Grounding {
    /// Fresh (a KB citation) or satisfied (a receipt).
    Grounded,
    /// Met by an attestation: grounded, and named as such.
    Attested,
    /// Stale, failed, missing, unreadable, or cited nothing.
    Ungrounded,
}

impl Grounding {
    pub fn name(self) -> &'static str {
        match self {
            Grounding::Grounded => "grounded",
            Grounding::Attested => "attested",
            Grounding::Ungrounded => "ungrounded",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "grounded" => Some(Grounding::Grounded),
            "attested" => Some(Grounding::Attested),
            "ungrounded" => Some(Grounding::Ungrounded),
            _ => None,
        }
    }
}
