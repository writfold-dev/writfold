//! Dependency content remains exact across cloning and equality checks.
use super::Dependency;
use vstd::prelude::*;

verus! {

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Dependency {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { self@ == other@ }
}

impl Clone for Dependency {
    fn clone(&self) -> (cloned: Self)
        ensures cloned@ == self@,
    {
        proof { broadcast use vstd::std_specs::vec::group_vec_axioms; }
        match self {
            Self::Observation(value) => Self::Observation(value.clone()),
            Self::Claim(value) => Self::Claim(value.clone()),
            Self::Attestation(value) => Self::Attestation(value.clone()),
            Self::Enumeration(value) => Self::Enumeration(value.clone()),
            Self::Corrections { target, events } => Self::Corrections {
                target: target.clone(), events: events.clone(),
            },
            Self::Context(value) => Self::Context(value.clone()),
        }
    }
}

impl PartialEq for Dependency {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (self@ == other@),
    {
        proof { broadcast use crate::equality::vector_equality; }
        match (self, other) {
            (Self::Observation(left), Self::Observation(right)) => left == right,
            (Self::Claim(left), Self::Claim(right)) => left == right,
            (Self::Attestation(left), Self::Attestation(right)) => left == right,
            (Self::Enumeration(left), Self::Enumeration(right)) => left == right,
            (Self::Corrections { target, events }, Self::Corrections { target: other_target, events: other_events }) =>
                target == other_target && events == other_events,
            (Self::Context(left), Self::Context(right)) => left == right,
            _ => false,
        }
    }
}
impl Eq for Dependency {}

} // verus!
