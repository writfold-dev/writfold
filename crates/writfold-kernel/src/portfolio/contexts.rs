//! Context lookup, exact retirement and immutable query inputs.
#[cfg(verus_keep_ghost)]
use super::model::*;
use crate::checker::context_matches;
#[cfg(verus_keep_ghost)]
use crate::workflow::admission_model::{context_matches_spec, prefix};
use crate::workflow::{actor_authorized, unique};
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::cmp::PartialEqSpec;
use writfold_domain::*;

verus! {
pub fn find_context<'a>(contexts: &'a [NamedContext], identity: &ContextId) -> (found: Option<&'a NamedContext>)
    ensures found.deep_view() == first_context(contexts.deep_view(), identity@),
{
    proof { reveal(first_context); }
    let mut i = 0;
    while i < contexts.len()
        invariant i <= contexts.len(),
            forall|j: int| 0 <= j < i ==> contexts.deep_view()[j].id != identity@,
        decreases contexts.len() - i,
    {
        let item = &contexts[i];
        proof { assert(item@ == contexts.deep_view()[i as int]); }
        if item.id() == identity {
            proof {
                reveal(first_context);
                assert(exists|k: int| 0 <= k < contexts.len() && contexts.deep_view()[k].id == identity@);
                assert(exists|k: int| 0 <= k < contexts.len() && contexts.deep_view()[k].id == identity@
                    && forall|j: int| 0 <= j < k ==> contexts.deep_view()[j].id != identity@);
                let k = choose|k: int| 0 <= k < contexts.len() && contexts.deep_view()[k].id == identity@
                    && forall|j: int| 0 <= j < k ==> contexts.deep_view()[j].id != identity@;
                assert(k == i);
            }
            return Some(item);
        }
        i += 1;
    }
    proof { reveal(first_context); }
    None
}
pub fn context_family_valid(kb: &KB, contexts: &[NamedContext]) -> (valid: bool)
    ensures valid == context_family_valid_spec(kb@, contexts.deep_view()),
{
    let mut i = 0;
    proof { assert(contexts.deep_view().len() == contexts.len()); }
    while i < contexts.len()
        invariant i <= contexts.len(),
            contexts.deep_view().len() == contexts.len(),
            forall|a: int, b: int| 0 <= b < a < i ==> contexts.deep_view()[a].id != contexts.deep_view()[b].id,
            forall|a: int| 0 <= a < i ==> context_matches_spec(kb@, contexts.deep_view()[a].context),
        decreases contexts.len() - i,
    {
        let item = &contexts[i];
        proof { assert(item@ == contexts.deep_view()[i as int]); }
        let mut j = 0;
        while j < i
            invariant j <= i < contexts.len(), item@ == contexts.deep_view()[i as int],
                contexts.deep_view().len() == contexts.len(),
                forall|a: int, b: int| 0 <= b < a < i ==> contexts.deep_view()[a].id != contexts.deep_view()[b].id,
                forall|a: int| 0 <= a < i ==> context_matches_spec(kb@, contexts.deep_view()[a].context),
                forall|a: int| 0 <= a < j ==> contexts.deep_view()[a].id != item@.id,
            decreases i - j,
        {
            let prior = &contexts[j];
            proof { assert(prior@ == contexts.deep_view()[j as int]); }
            if prior.id() == item.id() {
                proof { if context_family_valid_spec(kb@, contexts.deep_view()) {
                    assert(contexts.deep_view()[j as int].id != contexts.deep_view()[i as int].id);
                } }
                return false;
            }
            j += 1;
        }
        if !context_matches(kb, item.context()) { return false; }
        i += 1;
    }
    true
}
pub(super) fn all_authorized(contexts: &[NamedContext], actor: &ActorId) -> (valid: bool)
    ensures valid == actors_authorized(contexts.deep_view(), actor@),
{
    let mut i = 0;
    proof { assert(contexts.deep_view().len() == contexts.len()); }
    while i < contexts.len()
        invariant i <= contexts.len(), forall|j: int| 0 <= j < i ==>
            contexts.deep_view()[j].context.policy.authorized_actors.contains(actor@),
            contexts.deep_view().len() == contexts.len(),
        decreases contexts.len() - i,
    {
        let named = &contexts[i];
        proof { assert(named@ == contexts.deep_view()[i as int]); }
        if !actor_authorized(named.context(), actor) {
            proof { if actors_authorized(contexts.deep_view(), actor@) {
                assert(contexts.deep_view()[i as int].context.policy.authorized_actors.contains(actor@));
            } }
            return false;
        }
        i += 1;
    }
    true
}
proof fn retirement_step(before: Seq<NamedContextView>, after: Seq<NamedContextView>, i: int)
    requires 0 <= i < before.len(),
    ensures retired_context_ids_spec(before.take(i + 1), after)
        == retired_context_ids_spec(before.take(i), after)
            + if first_context(after, before[i].id).is_none() { seq![before[i].id] } else { seq![] },
{
    assert(before.take(i + 1).drop_last() =~= before.take(i));
}
proof fn retirement_prefix(before: Seq<NamedContextView>, after: Seq<NamedContextView>, i: int)
    requires 0 <= i <= before.len(),
    ensures prefix(retired_context_ids_spec(before.take(i), after), retired_context_ids_spec(before, after)),
    decreases before.len() - i,
{
    if i < before.len() {
        retirement_prefix(before.drop_last(), after, i);
        assert(before.drop_last().take(i) =~= before.take(i));
    } else { assert(before.take(i) =~= before); }
}
pub fn retired_context_ids(before: &[NamedContext], after: &[NamedContext]) -> (retired: Vec<ContextId>)
    ensures retired.deep_view() == retired_context_ids_spec(before.deep_view(), after.deep_view()),
{
    let mut retired = Vec::new();
    let mut i = 0;
    proof {
        assert(before.deep_view().len() == before.len());
        assert(retired.deep_view() =~= Seq::<Seq<char>>::empty());
    }
    while i < before.len()
        invariant i <= before.len(), before.deep_view().len() == before.len(),
            retired.deep_view() == retired_context_ids_spec(before.deep_view().take(i as int), after.deep_view()),
        decreases before.len() - i,
    {
        let old = &before[i];
        let ghost previous = retired.deep_view();
        proof {
            assert(old@ == before.deep_view()[i as int]);
            retirement_step(before.deep_view(), after.deep_view(), i as int);
        }
        if find_context(after, old.id()).is_none() {
            let id = old.id().clone();
            proof { assert(id@ == before.deep_view()[i as int].id); }
            retired.push(id);
            proof {
                assert(retired.deep_view() =~= previous.push(id@));
                assert(retired_context_ids_spec(before.deep_view().take(i as int + 1), after.deep_view())
                    =~= previous.push(id@));
            }
        }
        i += 1;
    }
    proof { assert(before.deep_view().take(before.len() as int) == before.deep_view()); }
    retired
}
fn retired_unique(retired: &[ContextId]) -> (valid: bool)
    ensures valid == (forall|a: int, b: int| 0 <= b < a < retired.len()
        ==> retired.deep_view()[a] != retired.deep_view()[b]),
{
    let valid = unique(retired);
    proof {
        assert(retired.deep_view().len() == retired.len());
        if valid {
            assert forall|a: int, b: int| 0 <= b < a < retired.len()
                implies retired.deep_view()[a] != retired.deep_view()[b] by {
                assert(!retired@[b].eq_spec(&retired@[a]));
            }
        } else {
            let (b, a) = choose|b: int, a: int| 0 <= b < a < retired.len()
                && retired@[b].eq_spec(&retired@[a]);
            assert(retired.deep_view()[a] == retired.deep_view()[b]);
        }
    }
    valid
}
pub fn context_evolution_valid(before: &[NamedContext], after: &[NamedContext], retired: &[ContextId]) -> (valid: bool)
    ensures valid == context_evolution_valid_spec(before.deep_view(), after.deep_view(), retired.deep_view()),
{
    if !retired_unique(retired) { return false; }
    let mut i = 0;
    let mut removed = 0;
    proof {
        assert(before.deep_view().len() == before.len());
        assert(retired.deep_view().len() == retired.len());
    }
    while i < before.len()
        invariant i <= before.len(), removed <= retired.len(),
            before.deep_view().len() == before.len(), retired.deep_view().len() == retired.len(),
            forall|a: int, b: int| 0 <= b < a < retired.len() ==> retired.deep_view()[a] != retired.deep_view()[b],
            retired.deep_view().take(removed as int)
                == retired_context_ids_spec(before.deep_view().take(i as int), after.deep_view()),
            forall|j: int| 0 <= j < i
                ==> #[trigger] retained_context_valid(before.deep_view()[j], after.deep_view()),
        decreases before.len() - i,
    {
        let old = &before[i];
        proof {
            assert(old@ == before.deep_view()[i as int]);
            retirement_step(before.deep_view(), after.deep_view(), i as int);
            retirement_prefix(before.deep_view(), after.deep_view(), i as int + 1);
        }
        match find_context(after, old.id()) {
            None => {
                proof {
                    let partial = retired_context_ids_spec(before.deep_view().take(i as int + 1), after.deep_view());
                    let full = retired_context_ids_spec(before.deep_view(), after.deep_view());
                    assert(partial =~= retired.deep_view().take(removed as int).push(old@.id));
                    assert(partial.len() == removed + 1);
                    assert(partial[removed as int] == old@.id);
                    assert(full.len() >= removed + 1);
                    assert(full[removed as int] == old@.id);
                }
                if removed == retired.len() || &retired[removed] != old.id() { return false; }
                removed += 1;
            },
            Some(current) => {
                if current.context().target() != old.context().target()
                    || !current.context().intended_use().eq(old.context().intended_use()) {
                    proof { if context_evolution_valid_spec(before.deep_view(), after.deep_view(), retired.deep_view()) {
                        assert(retained_context_valid(before.deep_view()[i as int], after.deep_view()));
                    } }
                    return false;
                }
            },
        }
        proof { assert(retained_context_valid(before.deep_view()[i as int], after.deep_view())); }
        i += 1;
    }
    proof {
        assert(before.deep_view().take(before.len() as int) == before.deep_view());
        if removed == retired.len() {
            assert(retired.deep_view().take(removed as int) == retired.deep_view());
        } else {
            assert(retired.deep_view().take(removed as int).len() < retired.deep_view().len());
        }
    }
    removed == retired.len()
}
pub(crate) fn context_fixed_fields_equal(left: &Context, right: &Context) -> (equal: bool)
    ensures equal == context_fixed_fields_equal_spec(left@, right@),
{
    proof { broadcast use writfold_domain::equality::vector_equality; }
    left.semantic_version().eq(right.semantic_version()) && left.ruleset() == right.ruleset()
        && left.checker() == right.checker() && left.policy() == right.policy()
        && left.intended_use().eq(right.intended_use()) && left.sources().eq(right.sources())
}
pub(super) fn clone_contexts(rows: &[NamedContext]) -> (cloned: Vec<NamedContext>)
    ensures cloned.deep_view() == rows.deep_view(),
{
    let mut cloned = Vec::with_capacity(rows.len());
    let mut i = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while i < rows.len()
        invariant i <= rows.len(), rows.deep_view().len() == rows.len(),
            cloned.deep_view() == rows.deep_view().take(i as int),
        decreases rows.len() - i,
    {
        let ghost prefix = rows.deep_view().take(i as int);
        let row = rows[i].clone();
        proof { assert(row@ == rows.deep_view()[i as int]); }
        cloned.push(row);
        proof {
            assert(cloned.deep_view() =~= prefix.push(row@));
            assert(rows.deep_view().take(i as int + 1) =~= prefix.push(row@));
        }
        i += 1;
    }
    proof { assert(rows.deep_view().take(rows.len() as int) == rows.deep_view()); }
    cloned
}
} // verus!
