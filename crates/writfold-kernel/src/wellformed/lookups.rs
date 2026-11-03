//! First-match determinism holds even before namespace uniqueness is known.
use vstd::prelude::*;
use writfold_domain::*;

verus! {
pub broadcast proof fn first_claim_unique(claims: Seq<ClaimView>, reference: ClaimRefView, a: ClaimView, b: ClaimView)
    requires #[trigger] first_claim_match(claims, reference, Some(a)),
        #[trigger] first_claim_match(claims, reference, Some(b)),
    ensures a == b,
{
    let i = choose|i: int| 0 <= i < claims.len() && a == claims[i] && claim_matches_spec(claims[i], reference)
        && forall|j: int| 0 <= j < i ==> !claim_matches_spec(claims[j], reference);
    let j = choose|j: int| 0 <= j < claims.len() && b == claims[j] && claim_matches_spec(claims[j], reference)
        && forall|i: int| 0 <= i < j ==> !claim_matches_spec(claims[i], reference);
    assert(i == j);
}
pub broadcast proof fn first_observation_unique(values: Seq<ObservationView>, id: Seq<char>, a: ObservationView, b: ObservationView)
    requires #[trigger] first_observation_match(values, id, Some(a)),
        #[trigger] first_observation_match(values, id, Some(b)),
    ensures a == b,
{
    let i = choose|i: int| 0 <= i < values.len() && a == values[i] && values[i].id == id
        && forall|j: int| 0 <= j < i ==> values[j].id != id;
    let j = choose|j: int| 0 <= j < values.len() && b == values[j] && values[j].id == id
        && forall|i: int| 0 <= i < j ==> values[i].id != id;
    assert(i == j);
}
}
