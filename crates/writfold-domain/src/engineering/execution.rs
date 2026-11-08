//! Closed engineering vocabulary corresponding to the existing calculus.
//! Values name capabilities and evidence roles; they grant no live authority.

vstd::prelude::verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    Read,
    Check,
    Write,
    Publish,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Gate {
    Test,
    Review,
    Compatibility,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueType {
    Unit,
    Artifact,
    Evidence,
}
}

crate::records::scalar_views!(Capability, Gate, ValueType);
