//! Candidate knowledge and independent assessment dimensions.
use crate::{Integer, evidence::*, ids::*, language::*, records::record};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Dependency {
    Observation(Observation),
    Claim(Claim),
    Attestation(Attestation),
    Enumeration(Enumeration),
    Corrections {
        target: ClaimRef,
        events: Vec<Correction>,
    },
    Context(Context),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptResult {
    Pass,
    Fail,
}

record!(Receipt {
    id: ReceiptId, claim: ClaimRef, context: Context, witness: Witness,
    dependencies: Vec<Dependency>, result: ReceiptResult, expires_at: Integer,
});

record!(KB {
    snapshot: SnapshotId, observations: Vec<Observation>, claims: Vec<Claim>,
    attestations: Vec<Attestation>, enumerations: Vec<Enumeration>,
    derivations: Vec<Derivation>, corrections: Vec<Correction>, receipts: Vec<Receipt>,
});

impl KB {
    pub fn empty(snapshot: SnapshotId) -> Self {
        Self::new(
            snapshot,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }
}

record!(Checked { statement: Statement, dependencies: Vec<Dependency> });

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Support {
    None,
    Positive,
    Negative,
    Both,
}

record!(Assessment {
    support: Support,
    applicable: bool,
    freshness: Freshness,
    validated: bool,
    contested: bool,
    available: bool,
    usable: bool,
});

pub fn find_claim<'a>(kb: &'a KB, reference: &ClaimRef) -> Option<&'a Claim> {
    kb.claims()
        .iter()
        .find(|claim| claim.matches_ref(reference))
}

pub fn find_observation<'a>(kb: &'a KB, ident: &ObservationId) -> Option<&'a Observation> {
    kb.observations()
        .iter()
        .find(|observation| observation.id() == ident)
}
