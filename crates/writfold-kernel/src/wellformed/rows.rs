use super::{collections, context_wf, graph, membership::*, natural, nonempty, scope_wf,
    statement_wf, unique, witness_wf};
#[cfg(verus_keep_ghost)]
use super::spec::*;
use vstd::prelude::*;
use writfold_domain::{Integer, *};

verus! {
broadcast use {super::lookups::first_claim_unique, super::lookups::first_observation_unique};
pub fn dependency_wf(dependency: &Dependency, kb: &KB) -> (valid: bool)
    ensures valid == dependency_wf_spec(dependency@, kb@),
{
    match dependency {
        Dependency::Observation(value) => observation_member(kb.observations(), value),
        Dependency::Claim(value) => claim_member(kb.claims(), value),
        Dependency::Attestation(value) => attestation_member(kb.attestations(), value),
        Dependency::Enumeration(value) => enumeration_member(kb.enumerations(), value),
        Dependency::Corrections { target, events } => {
            if find_claim(kb, target).is_none() { return false; }
            let mut i = 0;
            while i < events.len()
                invariant i <= events.len(), claim_present(kb@, target@),
                    forall|j: int| 0 <= j < i ==> kb@.corrections.contains(events.deep_view()[j])
                        && events.deep_view()[j].target == target@,
                decreases events.len() - i,
            {
                let event = &events[i];
                proof { assert(event@ == events.deep_view()[i as int]); }
                if !correction_member(kb.corrections(), event) || event.target() != target {
                    proof {
                        assert(!kb@.corrections.contains(events.deep_view()[i as int])
                            || events.deep_view()[i as int].target != target@);
                    }
                    return false;
                }
                i += 1;
            }
            true
        }
        Dependency::Context(value) => context_wf(value) && value.now() == &Integer::from(0),
    }
}
fn inventory_bound(attestation: &Attestation, kb: &KB, id: &EnumerationId) -> (valid: bool)
    ensures valid == (exists|i: int| 0 <= i < kb@.enumerations.len()
        && kb@.enumerations[i].id == id@
        && attestation@.statement.polarity == Polarity::Positive
        && attestation@.observation == kb@.enumerations[i].observation
        && attestation@.statement.scope == kb@.enumerations[i].scope),
{
    let values = kb.enumerations();
    let mut i = 0;
    while i < values.len()
        invariant i <= values.len(), values.deep_view() == kb@.enumerations,
            forall|j: int| 0 <= j < i ==> !(kb@.enumerations[j].id == id@
                && attestation@.statement.polarity == Polarity::Positive
                && attestation@.observation == kb@.enumerations[j].observation
                && attestation@.statement.scope == kb@.enumerations[j].scope),
        decreases values.len() - i,
    {
        let value = &values[i];
        proof { assert(value@ == kb@.enumerations[i as int]); }
        if value.id() == id && *attestation.statement().polarity() == Polarity::Positive
            && attestation.observation() == value.observation()
            && attestation.statement().scope() == value.scope() {
            proof {
                assert(kb@.enumerations[i as int].id == id@
                    && attestation@.statement.polarity == Polarity::Positive
                    && attestation@.observation == kb@.enumerations[i as int].observation
                    && attestation@.statement.scope == kb@.enumerations[i as int].scope);
            }
            return true;
        }
        i += 1;
    }
    false
}
fn attestation_wf(attestation: &Attestation, kb: &KB) -> (valid: bool)
    ensures valid == attestation_wf_spec(attestation@, kb@),
{
    let Some(claim) = find_claim(kb, attestation.claim()) else { return false; };
    if claim.statement() != attestation.statement()
        || !matches!(attestation.statement().formula(), Formula::Atomic(_) | Formula::Inventory { .. })
        || !nonempty(attestation.authority().as_str())
        || find_observation(kb, attestation.observation()).is_none() { return false; }
    if let Formula::Inventory { enumeration } = attestation.statement().formula() {
        return inventory_bound(attestation, kb, enumeration);
    }
    true
}
fn enumeration_wf(enumeration: &Enumeration, kb: &KB) -> (valid: bool)
    ensures valid == enumeration_wf_spec(enumeration@, kb@),
{
    let Some(observation) = find_observation(kb, enumeration.observation()) else { return false; };
    if observation.source() != enumeration.source() || observation.revision() != enumeration.revision()
        || !nonempty(enumeration.query()) || !nonempty(enumeration.method())
        || !scope_wf(enumeration.scope())
        || !writfold_domain::equality::strings_equal(enumeration.members(), enumeration.universe())
        || !unique(enumeration.members()) { return false; }
    let members = enumeration.members();
    let mut i = 0;
    while i < members.len()
        invariant i <= members.len(), members.deep_view() == enumeration@.members,
            enumeration@.members == enumeration@.universe, enumeration@.members.no_duplicates(),
            observation@.source == enumeration@.source, observation@.revision == enumeration@.revision,
            first_observation_match(kb@.observations, enumeration@.observation, Some(observation@)),
            nonempty_spec(enumeration@.query), nonempty_spec(enumeration@.method), scope_wf_spec(enumeration@.scope),
            forall|j: int| 0 <= j < i ==> nonempty_spec(enumeration@.members[j]),
        decreases members.len() - i,
    {
        let member = &members[i];
        proof { assert(member@ == enumeration@.members[i as int]); }
        if !nonempty(member) {
            proof { assert(!nonempty_spec(enumeration@.members[i as int])); }
            return false;
        }
        i += 1;
    }
    true
}
pub(super) fn correction_wf(correction: &Correction, kb: &KB) -> (valid: bool)
    ensures valid == correction_wf_spec(correction@, kb@),
{
    if find_claim(kb, correction.target()).is_none() || !natural(correction.recorded_at()) { return false; }
    match (correction.kind(), correction.replacement()) {
        (CorrectionKind::Supersede, Some(replacement)) => find_claim(kb, replacement).is_some(),
        (CorrectionKind::Supersede, None) => false,
        (_, Some(_)) => false,
        (_, None) => true,
    }
}
fn observation_wf(observation: &Observation, _kb: &KB) -> (valid: bool)
    ensures valid == observation_wf_spec(observation@),
{
    nonempty(observation.source().as_str()) && nonempty(observation.revision().as_str())
        && nonempty(observation.content().as_str()) && natural(observation.observed_at())
}
fn claim_wf(claim: &Claim, kb: &KB) -> (valid: bool)
    ensures valid == statement_wf_spec(claim@.statement, kb@),
{
    statement_wf(claim.statement(), kb)
}
fn derivation_wf(derivation: &Derivation, kb: &KB) -> (valid: bool)
    ensures valid == derivation_wf_spec(derivation@, kb@),
{
    find_claim(kb, derivation.claim()).is_some() && witness_wf(derivation.witness(), kb)
}
fn receipt_wf(receipt: &Receipt, kb: &KB) -> (valid: bool)
    ensures valid == receipt_wf_spec(receipt@, kb@),
{
    if find_claim(kb, receipt.claim()).is_none() || !context_wf(receipt.context())
        || !witness_wf(receipt.witness(), kb) || !natural(receipt.expires_at()) { return false; }
    let dependencies = receipt.dependencies();
    let mut i = 0;
    while i < dependencies.len()
        invariant i <= dependencies.len(), dependencies.deep_view() == receipt@.dependencies,
            claim_present(kb@, receipt@.claim), context_wf_spec(receipt@.context),
            witness_wf_spec(receipt@.witness, kb@), natural_spec(receipt@.expires_at),
            forall|j: int| 0 <= j < i ==> dependency_wf_spec(receipt@.dependencies[j], kb@),
        decreases dependencies.len() - i,
    {
        let dependency = &dependencies[i];
        proof { assert(dependency@ == receipt@.dependencies[i as int]); }
        if !dependency_wf(dependency, kb) {
            proof { assert(!dependency_wf_spec(receipt@.dependencies[i as int], kb@)); }
            return false;
        }
        i += 1;
    }
    true
}
}

macro_rules! all_rows {
    ($name:ident, $field:ident, $check:ident, $model:expr) => {
        verus! {
        fn $name(kb: &KB) -> (valid: bool)
            ensures valid == (forall|i: int| 0 <= i < kb@.$field.len() ==> ($model)(kb@.$field[i], kb@)),
        {
            let values = kb.$field();
            let mut i = 0;
            while i < values.len()
                invariant i <= values.len(), values.deep_view() == kb@.$field,
                    forall|j: int| 0 <= j < i ==> ($model)(kb@.$field[j], kb@),
                decreases values.len() - i,
            {
                let value = &values[i];
                proof { assert(value@ == kb@.$field[i as int]); }
                if !$check(value, kb) {
                    proof { assert(!($model)(kb@.$field[i as int], kb@)); }
                    return false;
                }
                i += 1;
            }
            true
        }
        }
    };
}
all_rows!(observations_wf, observations, observation_wf, |r: ObservationView, _k: KBView| observation_wf_spec(r));
all_rows!(claims_wf, claims, claim_wf, |r: ClaimView, k: KBView| statement_wf_spec(r.statement, k));
all_rows!(attestations_wf, attestations, attestation_wf, |r: AttestationView, k: KBView| attestation_wf_spec(r, k));
all_rows!(enumerations_wf, enumerations, enumeration_wf, |r: EnumerationView, k: KBView| enumeration_wf_spec(r, k));
all_rows!(derivations_wf, derivations, derivation_wf, |r: DerivationView, k: KBView| derivation_wf_spec(r, k));
all_rows!(correction_rows_wf, corrections, correction_wf, |r: CorrectionView, k: KBView| correction_wf_spec(r, k));
all_rows!(receipts_wf, receipts, receipt_wf, |r: ReceiptView, k: KBView| receipt_wf_spec(r, k));

verus! {
pub fn well_formed(kb: &KB) -> (valid: bool)
    ensures valid == well_formed_spec(kb@),
{
    if !collections::unique_observations(kb.observations()) { return false; }
    let claim_keys = collections::claim_index(kb);
    if !super::sort::adjacent_unique(&claim_keys) { return false; }
    collections::other_identities_unique(kb) && observations_wf(kb) && claims_wf(kb)
        && attestations_wf(kb) && enumerations_wf(kb) && derivations_wf(kb)
        && correction_rows_wf(kb) && graph::corrections_acyclic(kb, &claim_keys) && receipts_wf(kb)
}
}
