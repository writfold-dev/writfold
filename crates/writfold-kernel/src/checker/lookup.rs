use vstd::prelude::*;
use writfold_domain::*;

verus! {

pub open spec fn first_matching<A>(values: Seq<A>, predicate: spec_fn(A) -> bool) -> Option<A>
    decreases values.len(),
{
    if values.len() == 0 { None }
    else if predicate(values[0]) { Some(values[0]) }
    else { first_matching(values.drop_first(), predicate) }
}

pub proof fn first_matching_absent<A>(values: Seq<A>, predicate: spec_fn(A) -> bool)
    requires forall|i: int| 0 <= i < values.len() ==> !predicate(values[i]),
    ensures first_matching(values, predicate) == None,
    decreases values.len(),
{
    if values.len() > 0 {
        assert forall|i: int| 0 <= i < values.drop_first().len() implies
            !predicate(values.drop_first()[i]) by { assert(values.drop_first()[i] == values[i + 1]); }
        first_matching_absent(values.drop_first(), predicate);
    }
}

pub proof fn first_matching_at<A>(values: Seq<A>, predicate: spec_fn(A) -> bool, index: int)
    requires 0 <= index < values.len(), predicate(values[index]),
        forall|i: int| 0 <= i < index ==> !predicate(values[i]),
    ensures first_matching(values, predicate) == Some(values[index]),
    decreases index,
{
    if index > 0 {
        assert(values.drop_first()[index - 1] == values[index]);
        assert forall|i: int| 0 <= i < index - 1 implies !predicate(values.drop_first()[i]) by {
            assert(values.drop_first()[i] == values[i + 1]);
        }
        first_matching_at(values.drop_first(), predicate, index - 1);
    }
}

pub open spec fn claim_value(kb: KBView, reference: ClaimRefView) -> Option<ClaimView> {
    first_matching(kb.claims, |claim: ClaimView| claim_matches_spec(claim, reference))
}
pub open spec fn observation_value(kb: KBView, ident: Seq<char>) -> Option<ObservationView> {
    first_matching(kb.observations, |observation: ObservationView| observation.id == ident)
}
pub open spec fn attestation_value(kb: KBView, ident: Seq<char>) -> Option<AttestationView> {
    first_matching(kb.attestations, |attestation: AttestationView| attestation.id == ident)
}
pub open spec fn enumeration_value(kb: KBView, ident: Seq<char>) -> Option<EnumerationView> {
    first_matching(kb.enumerations, |enumeration: EnumerationView| enumeration.id == ident)
}

pub broadcast proof fn claim_lookup_value(kb: KBView, reference: ClaimRefView, found: Option<ClaimView>)
    requires #[trigger] first_claim_match(kb.claims, reference, found),
    ensures found == claim_value(kb, reference),
{
    let predicate = |claim: ClaimView| claim_matches_spec(claim, reference);
    match found {
        None => first_matching_absent(kb.claims, predicate),
        Some(claim) => {
            let index = choose|i: int| 0 <= i < kb.claims.len() && claim == kb.claims[i]
                && claim_matches_spec(kb.claims[i], reference)
                && forall|j: int| 0 <= j < i ==> !claim_matches_spec(kb.claims[j], reference);
            first_matching_at(kb.claims, predicate, index);
        },
    }
}

pub broadcast proof fn observation_lookup_value(kb: KBView, ident: Seq<char>, found: Option<ObservationView>)
    requires #[trigger] first_observation_match(kb.observations, ident, found),
    ensures found == observation_value(kb, ident),
{
    let predicate = |observation: ObservationView| observation.id == ident;
    match found {
        None => first_matching_absent(kb.observations, predicate),
        Some(observation) => {
            let index = choose|i: int| 0 <= i < kb.observations.len() && observation == kb.observations[i]
                && kb.observations[i].id == ident
                && forall|j: int| 0 <= j < i ==> kb.observations[j].id != ident;
            first_matching_at(kb.observations, predicate, index);
        },
    }
}

pub(super) fn find_attestation<'a>(kb: &'a KB, ident: &AttestationId) -> (found: Option<&'a Attestation>)
    ensures found.deep_view() == attestation_value(kb@, ident@),
{
    let rows = kb.attestations();
    let mut index = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while index < rows.len()
        invariant
            rows.deep_view() == kb@.attestations, rows.len() == kb@.attestations.len(),
            index <= rows.len(),
            forall|i: int| 0 <= i < index ==> kb@.attestations[i].id != ident@,
        decreases rows.len() - index,
    {
        let row = &rows[index];
        proof { assert(row@ == kb@.attestations[index as int]); }
        if row.id() == ident {
            proof { first_matching_at(kb@.attestations, |a: AttestationView| a.id == ident@, index as int); }
            return Some(row);
        }
        index += 1;
    }
    proof { first_matching_absent(kb@.attestations, |a: AttestationView| a.id == ident@); }
    None
}

pub(super) fn find_enumeration<'a>(kb: &'a KB, ident: &EnumerationId) -> (found: Option<&'a Enumeration>)
    ensures found.deep_view() == enumeration_value(kb@, ident@),
{
    let rows = kb.enumerations();
    let mut index = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while index < rows.len()
        invariant
            rows.deep_view() == kb@.enumerations, rows.len() == kb@.enumerations.len(),
            index <= rows.len(),
            forall|i: int| 0 <= i < index ==> kb@.enumerations[i].id != ident@,
        decreases rows.len() - index,
    {
        let row = &rows[index];
        proof { assert(row@ == kb@.enumerations[index as int]); }
        if row.id() == ident {
            proof { first_matching_at(kb@.enumerations, |e: EnumerationView| e.id == ident@, index as int); }
            return Some(row);
        }
        index += 1;
    }
    proof { first_matching_absent(kb@.enumerations, |e: EnumerationView| e.id == ident@); }
    None
}

} // verus!
