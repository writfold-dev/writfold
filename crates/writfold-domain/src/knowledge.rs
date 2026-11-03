//! Candidate knowledge and independent assessment dimensions.
use crate::{Integer, evidence::*, ids::*, language::*, records::record};
mod traits;
use vstd::prelude::*;

verus! {
#[derive(Debug)]
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

#[cfg(verus_keep_ghost)]
pub ghost enum DependencyView {
    Observation(ObservationView),
    Claim(ClaimView),
    Attestation(AttestationView),
    Enumeration(EnumerationView),
    Corrections { target: ClaimRefView, events: Seq<CorrectionView> },
    Context(ContextView),
}
#[cfg(verus_keep_ghost)]
impl View for Dependency {
    type V = DependencyView;
    open spec fn view(&self) -> DependencyView {
        match self {
            Self::Observation(value) => DependencyView::Observation(value@),
            Self::Claim(value) => DependencyView::Claim(value@),
            Self::Attestation(value) => DependencyView::Attestation(value@),
            Self::Enumeration(value) => DependencyView::Enumeration(value@),
            Self::Corrections { target, events } => DependencyView::Corrections {
                target: target@, events: events.deep_view(),
            },
            Self::Context(value) => DependencyView::Context(value@),
        }
    }
}
#[cfg(verus_keep_ghost)]
impl DeepView for Dependency {
    type V = DependencyView;
    open spec fn deep_view(&self) -> DependencyView { self@ }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptResult {
    Pass,
    Fail,
}
}

record!(Receipt, ReceiptView {
    id: ReceiptId, claim: ClaimRef, context: Context, witness: Witness,
    dependencies: Vec<Dependency>, result: ReceiptResult, expires_at: Integer,
});

record!(KB, KBView {
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

record!(Checked, CheckedView { statement: Statement, dependencies: Vec<Dependency> });

::vstd::prelude::verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Support {
    None,
    Positive,
    Negative,
    Both,
}
}

record!(Assessment, AssessmentView {
    support: Support,
    applicable: bool,
    freshness: Freshness,
    validated: bool,
    contested: bool,
    available: bool,
    usable: bool,
});

verus! {
pub open spec fn first_claim_match(
    claims: Seq<ClaimView>, reference: ClaimRefView, found: Option<ClaimView>,
) -> bool {
    match found {
        None => forall|i: int| 0 <= i < claims.len() ==> !claim_matches_spec(claims[i], reference),
        Some(claim) => exists|i: int| 0 <= i < claims.len()
            && claim == claims[i] && claim_matches_spec(claims[i], reference)
            && forall|j: int| 0 <= j < i ==> !claim_matches_spec(claims[j], reference),
    }
}

pub fn find_claim<'a>(kb: &'a KB, reference: &ClaimRef) -> (found: Option<&'a Claim>)
    ensures first_claim_match(kb@.claims, reference@, found.deep_view()),
{
    let claims = kb.claims();
    proof { assert(claims.deep_view().len() == claims.len()); }
    let mut index = 0;
    while index < claims.len()
        invariant
            claims.deep_view() == kb@.claims,
            claims.len() == kb@.claims.len(),
            index <= claims.len(),
            forall|i: int| 0 <= i < index ==> !claim_matches_spec(kb@.claims[i], reference@),
        decreases claims.len() - index,
    {
        let claim = &claims[index];
        proof { assert(claim@ == kb@.claims[index as int]); }
        if claim.matches_ref(reference) {
            return Some(claim);
        }
        index += 1;
    }
    None
}

pub open spec fn first_observation_match(
    observations: Seq<ObservationView>, ident: Seq<char>, found: Option<ObservationView>,
) -> bool {
    match found {
        None => forall|i: int| 0 <= i < observations.len() ==> observations[i].id != ident,
        Some(observation) => exists|i: int| 0 <= i < observations.len()
            && observation == observations[i] && observations[i].id == ident
            && forall|j: int| 0 <= j < i ==> observations[j].id != ident,
    }
}

pub fn find_observation<'a>(kb: &'a KB, ident: &ObservationId) -> (found: Option<&'a Observation>)
    ensures first_observation_match(kb@.observations, ident@, found.deep_view()),
{
    let observations = kb.observations();
    proof { assert(observations.deep_view().len() == observations.len()); }
    let mut index = 0;
    while index < observations.len()
        invariant
            observations.deep_view() == kb@.observations,
            observations.len() == kb@.observations.len(),
            index <= observations.len(),
            forall|i: int| 0 <= i < index ==> kb@.observations[i].id != ident@,
        decreases observations.len() - index,
    {
        let observation = &observations[index];
        proof { assert(observation@ == kb@.observations[index as int]); }
        if observation.id() == ident {
            return Some(observation);
        }
        index += 1;
    }
    None
}
}

crate::records::scalar_views!(ReceiptResult, Support);
