//! Compare the exact finite-member formula without building a second tree.
#[cfg(verus_keep_ghost)]
use super::evaluation_model::inventory_expected;
use vstd::prelude::*;
use writfold_domain::*;

verus! {

fn members_match(formula: &Formula, members: &[String], predicate: &str) -> (matches: bool)
    ensures matches == (formula@ == member_formula_spec(members.deep_view(), predicate@)),
{
    let mut current = formula;
    let mut index = 0;
    proof {
        assert(members.deep_view().len() == members.len());
        assert(members.deep_view().skip(0) =~= members.deep_view());
    }
    while index < members.len()
        invariant
            index <= members.len(), members.deep_view().len() == members.len(),
            (formula@ == member_formula_spec(members.deep_view(), predicate@))
                == (current@ == member_formula_spec(members.deep_view().skip(index as int), predicate@)),
        decreases members.len() - index,
    {
        proof {
            reveal_with_fuel(member_formula_spec, 2);
            reveal_with_fuel(<Formula as View>::view, 2);
            assert(members.deep_view().skip(index as int)[0] == members.deep_view()[index as int]);
            assert(members.deep_view().skip(index as int).drop_first()
                =~= members.deep_view().skip(index as int + 1));
        }
        let Formula::Conjunction { left: head, right: tail } = current else { return false; };
        let Formula::Atomic(atom) = &**head else { return false; };
        let arguments = atom.args();
        proof {
            assert(arguments.deep_view().len() == arguments.len());
            assert(members[index as int]@ == members.deep_view()[index as int]);
        }
        if !atom.subject().as_str().eq(members[index].as_str())
            || !atom.predicate().as_str().eq(predicate) || !arguments.is_empty() {
            return false;
        }
        current = tail;
        index += 1;
    }
    proof { assert(members.deep_view().skip(index as int) =~= Seq::<Seq<char>>::empty()); }
    matches!(current, Formula::Truth)
}

fn inventory_matches(formula: &Formula, ident: &EnumerationId) -> (matches: bool)
    ensures matches == (formula@ == (FormulaView::Inventory { enumeration: ident@ })),
{
    match formula {
        Formula::Inventory { enumeration } => enumeration == ident,
        _ => false,
    }
}

pub(super) fn matches_expected(formula: &Formula, enumeration: &Enumeration, predicate: Option<&str>) -> (matches: bool)
    ensures matches == (formula@ == inventory_expected(enumeration@, predicate.deep_view())),
{
    match predicate {
        None => inventory_matches(formula, enumeration.id()),
        Some(predicate) => match formula {
            Formula::Conjunction { left, right } => inventory_matches(left, enumeration.id())
                && members_match(right, enumeration.members(), predicate),
            _ => false,
        },
    }
}

} // verus!
