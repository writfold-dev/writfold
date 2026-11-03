//! The unchanged version-2 finite-witness language.
use crate::{ids::*, records::record};
mod traits;
use vstd::prelude::*;

record!( Scope, ScopeView {
    project: ProjectId,
    selector: String
});

verus! {
pub open spec fn narrower_spec(target: ScopeView, source: ScopeView) -> bool {
    target.project == source.project && (source.selector.len() == 0 || target.selector == source.selector)
}
pub fn narrower(target: &Scope, source: &Scope) -> (narrow: bool)
    ensures narrow == narrower_spec(target@, source@),
{
    target.project() == source.project()
        && (source.selector().as_str().is_empty()
            || target.selector().as_str().eq(source.selector().as_str()))
}
}

record!(Atom, AtomView {
    subject: String, predicate: String, args: Vec<String> => crate::equality::strings_equal
});

verus! {
#[derive(Debug)]
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

#[cfg(verus_keep_ghost)]
pub ghost enum FormulaView {
    Atomic(AtomView),
    Truth,
    Conjunction { left: Box<FormulaView>, right: Box<FormulaView> },
    All { enumeration: Seq<char>, predicate: Seq<char> },
    Absent { enumeration: Seq<char> },
    Inventory { enumeration: Seq<char> },
}

#[cfg(verus_keep_ghost)]
impl View for Formula {
    type V = FormulaView;
    open spec fn view(&self) -> FormulaView
        decreases self,
    {
        match self {
            Self::Atomic(atom) => FormulaView::Atomic(atom@),
            Self::Truth => FormulaView::Truth,
            Self::Conjunction { left, right } => FormulaView::Conjunction {
                left: Box::new((**left)@), right: Box::new((**right)@),
            },
            Self::All { enumeration, predicate } => FormulaView::All {
                enumeration: enumeration@, predicate: predicate@,
            },
            Self::Absent { enumeration } => FormulaView::Absent { enumeration: enumeration@ },
            Self::Inventory { enumeration } => FormulaView::Inventory { enumeration: enumeration@ },
        }
    }
}
#[cfg(verus_keep_ghost)]
impl DeepView for Formula {
    type V = FormulaView;
    open spec fn deep_view(&self) -> FormulaView { self@ }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Polarity {
    Positive,
    Negative,
}
}

record!(Statement, StatementView {
    formula: Formula,
    scope: Scope,
    polarity: Polarity
});
record!(ClaimRef, ClaimRefView {
    id: ClaimId,
    rev: ClaimRevision
});
record!(Claim, ClaimView {
    id: ClaimId,
    revision: ClaimRevision,
    statement: Statement
});

verus! {
pub open spec fn claim_matches_spec(claim: ClaimView, reference: ClaimRefView) -> bool {
    claim.id == reference.id && claim.revision == reference.rev
}
impl Claim {
    pub fn claim_ref(&self) -> (reference: ClaimRef)
        ensures reference@.id == self@.id, reference@.rev == self@.revision,
    {
        ClaimRef::new(self.id().clone(), self.revision().clone())
    }
    pub fn matches_ref(&self, reference: &ClaimRef) -> (matches: bool)
        ensures matches == claim_matches_spec(self@, reference@),
    {
        self.id() == reference.id() && self.revision() == reference.rev()
    }
}
}

verus! {
#[derive(Debug)]
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

#[cfg(verus_keep_ghost)]
pub ghost enum WitnessView {
    Attest { attestation: Seq<char> },
    Truth { scope: ScopeView },
    And { left: Box<WitnessView>, right: Box<WitnessView> },
    Specialize { scope: ScopeView, premise: Box<WitnessView> },
    All { enumeration: Seq<char>, predicate: Seq<char>, premise: Box<WitnessView> },
    Absent { enumeration: Seq<char>, premise: Box<WitnessView> },
}

#[cfg(verus_keep_ghost)]
impl View for Witness {
    type V = WitnessView;
    open spec fn view(&self) -> WitnessView
        decreases self,
    {
        match self {
            Self::Attest { attestation } => WitnessView::Attest { attestation: attestation@ },
            Self::Truth { scope } => WitnessView::Truth { scope: scope@ },
            Self::And { left, right } => WitnessView::And {
                left: Box::new((**left)@), right: Box::new((**right)@),
            },
            Self::Specialize { scope, premise } => WitnessView::Specialize {
                scope: scope@, premise: Box::new((**premise)@),
            },
            Self::All { enumeration, predicate, premise } => WitnessView::All {
                enumeration: enumeration@, predicate: predicate@, premise: Box::new((**premise)@),
            },
            Self::Absent { enumeration, premise } => WitnessView::Absent {
                enumeration: enumeration@, premise: Box::new((**premise)@),
            },
        }
    }
}
#[cfg(verus_keep_ghost)]
impl DeepView for Witness {
    type V = WitnessView;
    open spec fn deep_view(&self) -> WitnessView { self@ }
}
}

record!(Derivation, DerivationView {
    id: DerivationId,
    claim: ClaimRef,
    witness: Witness
});

verus! {
pub open spec fn member_formula_spec(members: Seq<Seq<char>>, predicate: Seq<char>) -> FormulaView
    decreases members.len(),
{
    if members.len() == 0 {
        FormulaView::Truth
    } else {
        FormulaView::Conjunction {
            left: Box::new(FormulaView::Atomic(AtomView {
                subject: members[0], predicate, args: Seq::empty(),
            })),
            right: Box::new(member_formula_spec(members.drop_first(), predicate)),
        }
    }
}

pub fn member_formula(members: &[String], predicate: &str) -> (formula: Formula)
    ensures formula@ == member_formula_spec(members.deep_view(), predicate@),
{
    let mut index = members.len();
    let mut result = Formula::Truth;
    proof {
        reveal_with_fuel(<Formula as View>::view, 2);
        assert(members.deep_view().len() == members.len());
    }
    while index > 0
        invariant
            index <= members.len(),
            members.deep_view().len() == members.len(),
            result@ == member_formula_spec(
                members.deep_view().subrange(index as int, members.len() as int), predicate@),
        decreases index,
    {
        index -= 1;
        let arguments: Vec<String> = Vec::new();
        proof {
            reveal_with_fuel(<Formula as View>::view, 2);
            assert(members.deep_view().subrange(index as int, members.len() as int).len() > 0);
            assert(members.deep_view().subrange(index as int, members.len() as int).drop_first()
                == members.deep_view().subrange(index as int + 1, members.len() as int));
            assert(members.deep_view().subrange(index as int, members.len() as int)[0]
                == members[index as int]@);
            assert(arguments.deep_view() == Seq::<Seq<char>>::empty());
        }
        result = Formula::Conjunction {
            left: Box::new(Formula::Atomic(Atom::new(
                members[index].clone(), predicate.to_owned(), arguments,
            ))),
            right: Box::new(result),
        };
        proof {
            assert(result@ == member_formula_spec(
                members.deep_view().subrange(index as int, members.len() as int), predicate@));
        }
    }
    proof {
        assert(members.deep_view().subrange(0, members.len() as int) == members.deep_view());
    }
    result
}
}

crate::records::scalar_views!(Polarity);
