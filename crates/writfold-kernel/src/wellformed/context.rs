#[cfg(verus_keep_ghost)]
use super::spec::*;
use super::{collections, natural, nonempty, scope_wf, ttl_wf};
use vstd::prelude::*;
use writfold_domain::*;

verus! {
fn sources_wf(sources: &[SourceSelection]) -> (valid: bool)
    ensures valid == (forall|i: int| 0 <= i < sources.len() ==> source_wf_spec(sources.deep_view()[i])),
{
    let mut i = 0;
    proof { assert(sources.deep_view().len() == sources.len()); }
    while i < sources.len()
        invariant i <= sources.len(),
            sources.deep_view().len() == sources.len(),
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
        assert(actors.deep_view().len() == actors.len());
    }
    let mut i = 0;
    while i < actors.len()
        invariant i <= actors.len(), actors.deep_view() == policy@.authorized_actors,
            actors.len() == policy@.authorized_actors.len(),
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
    let sources: &[SourceSelection] = context.sources();
    proof {
        assert(sources.deep_view() =~= context@.sources);
        assert(sources.deep_view().len() == sources.len());
        assert(sources.len() == context@.sources.len());
    }
    context.semantic_version().as_str().eq(semantic_version())
        && natural(context.now())
        && context.ruleset().as_str().eq("core-2")
        && context.checker().as_str().eq("kctx-2")
        && scope_wf(context.target()) && nonempty(context.intended_use())
        && collections::unique_sources(sources) && sources_wf(sources)
        && policy_wf(context.policy())
}
}
