use super::super::actor_present;
#[cfg(verus_keep_ghost)]
use super::super::actor_present_spec;
use vstd::prelude::*;
use writfold_domain::{Integer, *};
use writfold_domain::text::nonblank;
#[cfg(verus_keep_ghost)]
use writfold_domain::text::nonblank_spec;

verus! {

pub open spec fn authored_gate_valid_spec(d: AuthoredGateDefinitionView) -> bool {
    actor_present_spec(d.by) && nonblank_spec(d.id)
        && nonblank_spec(d.claim) && nonblank_spec(d.check)
}

pub open spec fn authored_ready_spec(p: AuthoredProposalView) -> bool {
    p.profile == AuthoredProfile::LegacyLimited || (
        p.research_present && p.alternatives_count >= 2 && p.gate_definitions.len() > 0
        && (forall|i: int| 0 <= i < p.gate_definitions.len() ==>
            authored_gate_valid_spec(p.gate_definitions[i]))
        && (forall|i: int, j: int| 0 <= i < j < p.gate_definitions.len() ==>
            p.gate_definitions[i].id != p.gate_definitions[j].id))
}

pub open spec fn gate_matches(p: AuthoredProposalView, d: AuthoredGateDefinitionView,
    r: AuthoredGateResultView, now: int) -> bool {
    r.definition == d && r.target_revision == p.target_revision
        && r.proposal_revision == p.proposal_revision && r.environment == p.environment
        && r.scope == p.scope && actor_present_spec(r.checker)
        && (!actor_present_spec(d.checker) || r.checker == d.checker)
        && r.observed_at <= now
}

pub open spec fn references_present(refs: Seq<Seq<char>>) -> bool {
    refs.len() > 0 && forall|i: int| 0 <= i < refs.len() ==> nonblank_spec(refs[i])
}

pub open spec fn passing_result(r: AuthoredGateResultView, now: int) -> bool {
    r.result == GateOutcome::Pass && now <= r.expires_at
        && references_present(r.evidence_refs) && actor_present_spec(r.actor)
}

pub open spec fn latest_matching_result(p: AuthoredProposalView,
    d: AuthoredGateDefinitionView, now: int, i: int) -> bool {
    0 <= i < p.gate_results.len() && gate_matches(p, d, p.gate_results[i], now)
        && forall|j: int| i < j < p.gate_results.len() ==>
            !gate_matches(p, d, p.gate_results[j], now)
}

// Select the greatest MATCHING index, not the last result with the same gate id.
pub open spec fn authored_gate_passes_spec(p: AuthoredProposalView,
    d: AuthoredGateDefinitionView, now: int) -> bool {
    nonblank_spec(p.environment) && nonblank_spec(p.scope)
        && actor_present_spec(p.proposal_revision)
        && exists|i: int| latest_matching_result(p, d, now, i)
            && passing_result(p.gate_results[i], now)
}

proof fn selected_result_decides(p: AuthoredProposalView, d: AuthoredGateDefinitionView,
    now: int, index: int)
    requires nonblank_spec(p.environment), nonblank_spec(p.scope),
        actor_present_spec(p.proposal_revision), latest_matching_result(p, d, now, index),
    ensures authored_gate_passes_spec(p, d, now) == passing_result(p.gate_results[index], now),
{
    if passing_result(p.gate_results[index], now) {
        assert(exists|i: int| latest_matching_result(p, d, now, i)
            && passing_result(p.gate_results[i], now));
    }
    if authored_gate_passes_spec(p, d, now) {
        let i = choose|i: int| latest_matching_result(p, d, now, i)
            && passing_result(p.gate_results[i], now);
        if i < index {
            assert(!gate_matches(p, d, p.gate_results[index], now));
        }
        if index < i {
            assert(!gate_matches(p, d, p.gate_results[i], now));
        }
        assert(i == index);
    }
}

pub fn authored_gate_valid(definition: &AuthoredGateDefinition) -> (valid: bool)
    ensures valid == authored_gate_valid_spec(definition@),
{
    actor_present(definition.by()) && nonblank(definition.id().as_str())
        && nonblank(definition.claim()) && nonblank(definition.check())
}

pub fn authored_ready(proposal: &AuthoredProposal) -> (ready: bool)
    ensures ready == authored_ready_spec(proposal@),
{
    if *proposal.profile() == AuthoredProfile::LegacyLimited { return true; }
    let definitions = proposal.gate_definitions();
    if !*proposal.research_present() || proposal.alternatives_count() < &Integer::from(2)
        || definitions.is_empty() { return false; }
    let mut i = 0;
    while i < definitions.len()
        invariant i <= definitions.len(),
            forall|k: int| 0 <= k < i ==> authored_gate_valid_spec(definitions.deep_view()[k]),
            forall|k: int, j: int| 0 <= k < j < i ==>
                definitions.deep_view()[k].id != definitions.deep_view()[j].id,
        decreases definitions.len() - i,
    {
        let definition = &definitions[i];
        if !authored_gate_valid(definition) {
            proof {
                assert(definition@ == proposal@.gate_definitions[i as int]);
                if authored_ready_spec(proposal@) {
                    assert(authored_gate_valid_spec(proposal@.gate_definitions[i as int]));
                }
            }
            return false;
        }
        let mut j = 0;
        while j < i
            invariant j <= i < definitions.len(),
                forall|k: int| 0 <= k < j ==>
                    definitions.deep_view()[k].id != definition@.id,
            decreases i - j,
        {
            if definitions[j].id() == definition.id() {
                proof {
                    assert(definitions.deep_view()[j as int].id == definition@.id);
                    assert(definition@ == proposal@.gate_definitions[i as int]);
                    if authored_ready_spec(proposal@) {
                        assert(proposal@.gate_definitions[j as int].id
                            != proposal@.gate_definitions[i as int].id);
                    }
                }
                return false;
            }
            j += 1;
        }
        i += 1;
    }
    true
}

pub(super) fn material_present(revision: &Option<ProposalMaterialRevision>) -> (present: bool)
    ensures present == actor_present_spec(revision.deep_view()),
{
    match revision { Some(id) => nonblank(id.as_str()), None => false }
}

fn checker_present(checker: &Option<CheckerId>) -> (present: bool)
    ensures present == actor_present_spec(checker.deep_view()),
{
    match checker { Some(id) => nonblank(id.as_str()), None => false }
}

pub(super) fn nonblank_references(refs: &Vec<String>) -> (present: bool)
    ensures present == references_present(refs.deep_view()),
{
    if refs.is_empty() { return false; }
    let mut i = 0;
    while i < refs.len()
        invariant i <= refs.len(),
            forall|j: int| 0 <= j < i ==> nonblank_spec(refs.deep_view()[j]),
        decreases refs.len() - i,
    {
        if !nonblank(&refs[i]) {
            proof {
                assert(refs[i as int]@ == refs.deep_view()[i as int]);
                if references_present(refs.deep_view()) {
                    assert(nonblank_spec(refs.deep_view()[i as int]));
                }
            }
            return false;
        }
        i += 1;
    }
    true
}

fn result_matches(proposal: &AuthoredProposal, definition: &AuthoredGateDefinition,
    result: &AuthoredGateResult, now: &Integer) -> (matches: bool)
    ensures matches == gate_matches(proposal@, definition@, result@, now@),
{
    result.definition() == definition && result.target_revision() == proposal.target_revision()
        && result.proposal_revision() == proposal.proposal_revision()
        && result.environment().eq(proposal.environment())
        && result.scope().eq(proposal.scope()) && checker_present(result.checker())
        && (!checker_present(definition.checker()) || result.checker() == definition.checker())
        && result.observed_at() <= now
}

pub fn authored_gate_passes(proposal: &AuthoredProposal,
    definition: &AuthoredGateDefinition, now: &Integer) -> (passes: bool)
    ensures passes == authored_gate_passes_spec(proposal@, definition@, now@),
{
    if !nonblank(proposal.environment()) || !nonblank(proposal.scope())
        || !material_present(proposal.proposal_revision()) { return false; }
    let results = proposal.gate_results();
    let mut end = results.len();
    while end > 0
        invariant end <= results.len(),
            forall|j: int| end <= j < results.len() ==>
                !gate_matches(proposal@, definition@, results.deep_view()[j], now@),
        decreases end,
    {
        end -= 1;
        let result = &results[end];
        if result_matches(proposal, definition, result, now) {
            let passes = *result.result() == GateOutcome::Pass && now <= result.expires_at()
                && nonblank_references(result.evidence_refs()) && actor_present(result.actor());
            proof {
                assert(result@ == proposal@.gate_results[end as int]);
                assert(latest_matching_result(proposal@, definition@, now@, end as int));
                selected_result_decides(proposal@, definition@, now@, end as int);
            }
            return passes;
        }
    }
    false
}

} // verus!
