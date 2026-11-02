//! The unchanged version-2 finite-witness language.
use crate::{ids::*, records::record};

record!(Scope {
    project: ProjectId,
    selector: String
});

pub fn narrower(target: &Scope, source: &Scope) -> bool {
    target.project() == source.project()
        && (source.selector().is_empty() || target.selector() == source.selector())
}

record!(Atom { subject: String, predicate: String, args: Vec<String> });

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Formula {
    Atomic(Atom),
    Truth,
    Conjunction {
        left: Box<Formula>,
        right: Box<Formula>,
    },
    All {
        enumeration: EnumerationId,
        predicate: String,
    },
    Absent {
        enumeration: EnumerationId,
    },
    Inventory {
        enumeration: EnumerationId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Polarity {
    Positive,
    Negative,
}

record!(Statement {
    formula: Formula,
    scope: Scope,
    polarity: Polarity
});
record!(ClaimRef {
    id: ClaimId,
    rev: ClaimRevision
});
record!(Claim {
    id: ClaimId,
    revision: ClaimRevision,
    statement: Statement
});

impl Claim {
    pub fn claim_ref(&self) -> ClaimRef {
        ClaimRef::new(self.id().clone(), self.revision().clone())
    }
    pub fn matches_ref(&self, reference: &ClaimRef) -> bool {
        self.id() == reference.id() && self.revision() == reference.rev()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Witness {
    Attest {
        attestation: AttestationId,
    },
    Truth {
        scope: Scope,
    },
    And {
        left: Box<Witness>,
        right: Box<Witness>,
    },
    Specialize {
        scope: Scope,
        premise: Box<Witness>,
    },
    All {
        enumeration: EnumerationId,
        predicate: String,
        premise: Box<Witness>,
    },
    Absent {
        enumeration: EnumerationId,
        premise: Box<Witness>,
    },
}

record!(Derivation {
    id: DerivationId,
    claim: ClaimRef,
    witness: Witness
});

pub fn member_formula(members: &[String], predicate: &str) -> Formula {
    members
        .iter()
        .rev()
        .fold(Formula::Truth, |right, member| Formula::Conjunction {
            left: Box::new(Formula::Atomic(Atom::new(
                member.clone(),
                predicate.to_owned(),
                Vec::new(),
            ))),
            right: Box::new(right),
        })
}
