//! Equality is of complete records, never only their identifiers.
use vstd::prelude::*;
use writfold_domain::*;

macro_rules! record_membership {
    ($name:ident, $row:ty) => {
        verus! {
        pub(super) fn $name(values: &Vec<$row>, value: &$row) -> (present: bool)
            ensures present == values.deep_view().contains(value@),
        {
            let mut i = 0;
            while i < values.len()
                invariant i <= values.len(),
                    forall|j: int| 0 <= j < i ==> values.deep_view()[j] != value@,
                decreases values.len() - i,
            {
                let current = &values[i];
                proof { assert(current@ == values.deep_view()[i as int]); }
                if current == value {
                    proof { assert(values.deep_view()[i as int] == value@); }
                    return true;
                }
                i += 1;
            }
            false
        }
        }
    };
}
record_membership!(observation_member, Observation);
record_membership!(claim_member, Claim);
record_membership!(attestation_member, Attestation);
record_membership!(enumeration_member, Enumeration);
record_membership!(correction_member, Correction);
