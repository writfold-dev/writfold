//! Structural trait bodies over the mathematical language views.
use super::{Formula, Witness};
use vstd::prelude::*;

verus! {

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Formula {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { self@ == other@ }
}

impl Clone for Formula {
    fn clone(&self) -> (cloned: Self)
        ensures cloned@ == self@,
        decreases self,
    {
        match self {
            Self::Atomic(atom) => Self::Atomic(atom.clone()),
            Self::Truth => Self::Truth,
            Self::Conjunction { left, right } => Self::Conjunction {
                left: Box::new((**left).clone()), right: Box::new((**right).clone()),
            },
            Self::All { enumeration, predicate } => Self::All {
                enumeration: enumeration.clone(), predicate: predicate.clone(),
            },
            Self::Absent { enumeration } => Self::Absent { enumeration: enumeration.clone() },
            Self::Inventory { enumeration } => Self::Inventory { enumeration: enumeration.clone() },
        }
    }
}

impl PartialEq for Formula {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (self@ == other@),
        decreases self,
    {
        match (self, other) {
            (Self::Atomic(left), Self::Atomic(right)) => left == right,
            (Self::Truth, Self::Truth) => true,
            (Self::Conjunction { left, right }, Self::Conjunction { left: other_left, right: other_right }) =>
                (**left).eq(&**other_left) && (**right).eq(&**other_right),
            (Self::All { enumeration, predicate }, Self::All { enumeration: other_id, predicate: other_predicate }) =>
                enumeration == other_id && predicate.eq(other_predicate),
            (Self::Absent { enumeration }, Self::Absent { enumeration: other_id }) => enumeration == other_id,
            (Self::Inventory { enumeration }, Self::Inventory { enumeration: other_id }) => enumeration == other_id,
            _ => false,
        }
    }
}
impl Eq for Formula {}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Witness {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { self@ == other@ }
}

impl Clone for Witness {
    fn clone(&self) -> (cloned: Self)
        ensures cloned@ == self@,
        decreases self,
    {
        match self {
            Self::Attest { attestation } => Self::Attest { attestation: attestation.clone() },
            Self::Truth { scope } => Self::Truth { scope: scope.clone() },
            Self::And { left, right } => Self::And {
                left: Box::new((**left).clone()), right: Box::new((**right).clone()),
            },
            Self::Specialize { scope, premise } => Self::Specialize {
                scope: scope.clone(), premise: Box::new((**premise).clone()),
            },
            Self::All { enumeration, predicate, premise } => Self::All {
                enumeration: enumeration.clone(), predicate: predicate.clone(),
                premise: Box::new((**premise).clone()),
            },
            Self::Absent { enumeration, premise } => Self::Absent {
                enumeration: enumeration.clone(), premise: Box::new((**premise).clone()),
            },
        }
    }
}

impl PartialEq for Witness {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (self@ == other@),
        decreases self,
    {
        match (self, other) {
            (Self::Attest { attestation }, Self::Attest { attestation: other_id }) => attestation == other_id,
            (Self::Truth { scope }, Self::Truth { scope: other_scope }) => scope == other_scope,
            (Self::And { left, right }, Self::And { left: other_left, right: other_right }) =>
                (**left).eq(&**other_left) && (**right).eq(&**other_right),
            (Self::Specialize { scope, premise }, Self::Specialize { scope: other_scope, premise: other_premise }) =>
                scope == other_scope && (**premise).eq(&**other_premise),
            (Self::All { enumeration, predicate, premise },
             Self::All { enumeration: other_id, predicate: other_predicate, premise: other_premise }) =>
                enumeration == other_id && predicate.eq(other_predicate) && (**premise).eq(&**other_premise),
            (Self::Absent { enumeration, premise }, Self::Absent { enumeration: other_id, premise: other_premise }) =>
                enumeration == other_id && (**premise).eq(&**other_premise),
            _ => false,
        }
    }
}
impl Eq for Witness {}

} // verus!
