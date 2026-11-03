//! Explicit O(n) borrowed-key scratch replaces opaque BTreeSet key models.
//! Sorting is O(n log n); no record, identifier, or string data is cloned.
use vstd::prelude::*;
use writfold_domain::*;
use super::{order::Key, sort};

macro_rules! unique_rows {
    ($name:ident, $row:ty, $rowvar:ident => $get:expr, $model:expr) => {
        verus! {
        pub(super) fn $name(values: &Vec<$row>) -> (unique: bool)
            ensures unique == values.deep_view().map_values($model).no_duplicates(),
        {
            if values.len() < 2 { return true; }
            let mut keys: Vec<Key<'_>> = Vec::with_capacity(values.len());
            let mut i = 0;
            while i < values.len()
                invariant i <= values.len(), keys.len() == i,
                    keys.deep_view() == values.deep_view().take(i as int).map_values(|r| (($model)(r), Seq::empty())),
                decreases values.len() - i,
            {
                let $rowvar = &values[i];
                let key = ($get, "");
                proof { assert(key.deep_view() == (($model)(values.deep_view()[i as int]), Seq::<char>::empty())); }
                keys.push(key);
                proof {
                    assert(keys.deep_view() =~= values.deep_view().take(i as int + 1)
                        .map_values(|r| (($model)(r), Seq::<char>::empty())));
                }
                i += 1;
            }
            proof {
                assert(keys.deep_view() =~= values.deep_view().map_values($model)
                    .map_values(|text: Seq<char>| (text, Seq::<char>::empty())));
                values.deep_view().map_values($model).lemma_no_duplicates_injective(|text: Seq<char>| (text, Seq::<char>::empty()));
            }
            sort::unique(keys)
        }
        }
    };
}
unique_rows!(unique_observations, Observation, r => r.id().as_str(), |r: ObservationView| r.id);
unique_rows!(unique_attestations, Attestation, r => r.id().as_str(), |r: AttestationView| r.id);
unique_rows!(unique_enumerations, Enumeration, r => r.id().as_str(), |r: EnumerationView| r.id);
unique_rows!(unique_derivations, Derivation, r => r.id().as_str(), |r: DerivationView| r.id);
unique_rows!(unique_corrections, Correction, r => r.id().as_str(), |r: CorrectionView| r.id);
unique_rows!(unique_receipts, Receipt, r => r.id().as_str(), |r: ReceiptView| r.id);
unique_rows!(unique_sources, SourceSelection, r => r.source().as_str(), |r: SourceSelectionView| r.source);
unique_rows!(unique_actors, ActorId, r => r.as_str(), |r: Seq<char>| r);

verus! {
pub(crate) fn unique(values: &[String]) -> (unique: bool)
    ensures unique == values.deep_view().no_duplicates(),
{
    if values.len() < 2 { return true; }
    let mut keys: Vec<Key<'_>> = Vec::with_capacity(values.len());
    let mut i = 0;
    while i < values.len()
        invariant i <= values.len(), keys.len() == i,
            keys.deep_view() == values.deep_view().take(i as int).map_values(|text: Seq<char>| (text, Seq::<char>::empty())),
        decreases values.len() - i,
    {
        let key = (values[i].as_str(), "");
        proof { assert(key.deep_view() == (values.deep_view()[i as int], Seq::<char>::empty())); }
        keys.push(key);
        proof {
            assert(keys.deep_view() =~= values.deep_view().take(i as int + 1)
                .map_values(|text: Seq<char>| (text, Seq::<char>::empty())));
        }
        i += 1;
    }
    proof {
        assert(keys.deep_view() =~= values.deep_view().map_values(|text: Seq<char>| (text, Seq::<char>::empty())));
        values.deep_view().lemma_no_duplicates_injective(|text: Seq<char>| (text, Seq::<char>::empty()));
    }
    sort::unique(keys)
}
pub(super) fn claim_index(kb: &KB) -> (keys: Vec<Key<'_>>)
    ensures sort::sorted(keys.deep_view()), keys.len() == kb@.claims.len(),
        keys.deep_view().to_multiset() == kb@.claims.map_values(|c: ClaimView| (c.id, c.revision)).to_multiset(),
        keys.deep_view().no_duplicates() == kb@.claims.map_values(|c: ClaimView| (c.id, c.revision)).no_duplicates(),
        keys.deep_view().to_set() == kb@.claims.map_values(|c: ClaimView| (c.id, c.revision)).to_set(),
{
    let claims = kb.claims();
    let mut keys: Vec<Key<'_>> = Vec::with_capacity(claims.len());
    let mut i = 0;
    while i < claims.len()
        invariant i <= claims.len(), keys.len() == i, claims.deep_view() == kb@.claims,
            keys.deep_view() == kb@.claims.take(i as int).map_values(|c: ClaimView| (c.id, c.revision)),
        decreases claims.len() - i,
    {
        let claim = &claims[i];
        let key = (claim.id().as_str(), claim.revision().as_str());
        proof { assert(key.deep_view() == (kb@.claims[i as int].id, kb@.claims[i as int].revision)); }
        keys.push(key);
        proof {
            assert(keys.deep_view() =~= kb@.claims.take(i as int + 1).map_values(|c: ClaimView| (c.id, c.revision)));
        }
        i += 1;
    }
    proof { assert(keys.deep_view() =~= kb@.claims.map_values(|c: ClaimView| (c.id, c.revision))); }
    let ghost original = keys.deep_view();
    sort::sort(&mut keys);
    proof {
        sort::permutation_unique(original, keys.deep_view());
        sort::permutation_membership(original, keys.deep_view());
    }
    keys
}
pub(super) fn other_identities_unique(kb: &KB) -> (unique: bool)
    ensures unique == (
        kb@.attestations.map_values(|r: AttestationView| r.id).no_duplicates()
        && kb@.enumerations.map_values(|r: EnumerationView| r.id).no_duplicates()
        && kb@.derivations.map_values(|r: DerivationView| r.id).no_duplicates()
        && kb@.corrections.map_values(|r: CorrectionView| r.id).no_duplicates()
        && kb@.receipts.map_values(|r: ReceiptView| r.id).no_duplicates()),
{
    unique_attestations(kb.attestations()) && unique_enumerations(kb.enumerations())
        && unique_derivations(kb.derivations()) && unique_corrections(kb.corrections())
        && unique_receipts(kb.receipts())
}
}
