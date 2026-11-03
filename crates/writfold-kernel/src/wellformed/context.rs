use super::{collections, natural, nonempty, scope_wf, ttl_wf};
#[cfg(verus_keep_ghost)]
use super::spec::*;
use vstd::prelude::*;
use writfold_domain::*;

verus! {
fn sources_wf(sources: &Vec<SourceSelection>) -> (valid: bool)
    ensures valid == (forall|i: int| 0 <= i < sources.len() ==> source_wf_spec(sources.deep_view()[i])),
{
    let mut i = 0;
    while i < sources.len()
        invariant i <= sources.len(),
            forall|j: int| 0 <= j < i ==> source_wf_spec(sources.deep_view()[j]),
        decreases sources.len() - i,
    {
        let source = &sources[i];
        proof { assert(source@ == sources.deep_view()[i as int]); }
        if !nonempty(source.source().as_str()) || !nonempty(source.revision().as_str())
            || !nonempty(source.content().as_str()) {
            proof { assert(!source_wf_spec(sources.deep_view()[i as int])); }
            return false;
        }
        i += 1;
    }
    true
}
fn policy_wf(policy: &Policy) -> (valid: bool)
    ensures valid == policy_wf_spec(policy@),
{
    let actors = policy.authorized_actors();
    if !collections::unique_actors(actors) { return false; }
    proof {
        assert(actors.deep_view().map_values(|actor: Seq<char>| actor) =~= actors.deep_view());
    }
    let mut i = 0;
    while i < actors.len()
        invariant i <= actors.len(), actors.deep_view() == policy@.authorized_actors,
            policy@.authorized_actors.no_duplicates(),
            forall|j: int| 0 <= j < i ==> nonempty_spec(policy@.authorized_actors[j]),
        decreases actors.len() - i,
    {
        let actor = &actors[i];
        proof { assert(actor@ == policy@.authorized_actors[i as int]); }
        if !nonempty(actor.as_str()) {
            proof { assert(!nonempty_spec(policy@.authorized_actors[i as int])); }
            return false;
        }
        i += 1;
    }
    ttl_wf(policy.ttl())
}
pub fn context_wf(context: &Context) -> (valid: bool)
    ensures valid == context_wf_spec(context@),
{
    context.semantic_version().as_str().eq(semantic_version())
        && natural(context.now())
        && context.ruleset().as_str().eq("core-2")
        && context.checker().as_str().eq("kctx-2")
        && scope_wf(context.target()) && nonempty(context.intended_use())
        && collections::unique_sources(context.sources()) && sources_wf(context.sources())
        && policy_wf(context.policy())
}
}
