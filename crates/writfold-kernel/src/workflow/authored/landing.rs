#[cfg(verus_keep_ghost)]
use super::super::actor_present_spec;
use super::gates::*;
use super::problems::*;
use vstd::prelude::*;
use writfold_domain::{Integer, *};

verus! {

pub open spec fn explicit_landed(p: AuthoredProposalView) -> bool {
    actor_present_spec(p.target_revision) && references_present(p.landed_refs)
}

pub open spec fn resolution_present(rows: Seq<AuthoredLandedView>, reference: Seq<char>) -> bool {
    exists|j: int| 0 <= j < rows.len() && rows[j].reference == reference
}

pub open spec fn resolved_landed(p: AuthoredProposalView) -> bool {
    forall|i: int| 0 <= i < p.landed_refs.len() ==>
        #[trigger] resolution_present(p.resolved_landed, p.landed_refs[i])
}

pub open spec fn bound_landed(p: AuthoredProposalView) -> bool {
    exists|i: int| 0 <= i < p.resolved_landed.len()
        && p.landed_refs.contains(p.resolved_landed[i].reference)
        && p.target_revision == Some(p.resolved_landed[i].target_revision)
}

pub open spec fn has_phase(p: AuthoredProposalView, phase: GateWhen) -> bool {
    exists|i: int| 0 <= i < p.gate_definitions.len() && p.gate_definitions[i].when == phase
}

pub open spec fn applicable_gates_pass(p: AuthoredProposalView, target: AuthoredStatus, now: int) -> bool {
    forall|i: int| 0 <= i < p.gate_definitions.len()
        && (target != AuthoredStatus::Applied || p.gate_definitions[i].when == GateWhen::Land)
        ==> authored_gate_passes_spec(p, p.gate_definitions[i], now)
}

pub open spec fn landing_problems_spec(p: AuthoredProposalView, target: AuthoredStatus,
    now: int) -> Seq<AuthoredProblem> {
    if !explicit_landed(p) { seq![AuthoredProblem::ExplicitLanded] }
    else if !resolved_landed(p) { seq![AuthoredProblem::ResolvableLanded] }
    else if !bound_landed(p) { seq![AuthoredProblem::BoundLanded] }
    else {
        (if target == AuthoredStatus::Applied {
            when_problem(!has_phase(p, GateWhen::Land), AuthoredProblem::LandGate)
        } else {
            when_problem(!has_phase(p, GateWhen::Deploy) || !has_phase(p, GateWhen::Use),
                AuthoredProblem::DeployUseGates)
        }) + when_problem(!applicable_gates_pass(p, target, now), AuthoredProblem::CurrentGates)
    }
}

fn target_present(target: &Option<TargetRevision>) -> (present: bool)
    ensures present == actor_present_spec(target.deep_view()),
{
    match target {
        Some(id) => writfold_domain::text::nonblank(id.as_str()),
        None => false,
    }
}

fn reference_resolves(rows: &[AuthoredLanded], reference: &String) -> (resolves: bool)
    ensures resolves == resolution_present(rows.deep_view(), reference@),
{
    let mut i = 0;
    while i < rows.len()
        invariant i <= rows.len(),
            forall|j: int| 0 <= j < i ==> rows.deep_view()[j].reference != reference@,
        decreases rows.len() - i,
    {
        if rows[i].reference().eq(reference) {
            proof {
                assert(rows.deep_view()[i as int].reference == reference@);
                assert(resolution_present(rows.deep_view(), reference@));
            }
            return true;
        }
        i += 1;
    }
    false
}

fn all_landed_resolve(proposal: &AuthoredProposal) -> (resolves: bool)
    ensures resolves == resolved_landed(proposal@),
{
    let refs = proposal.landed_refs();
    let mut i = 0;
    proof { assert(refs.deep_view().len() == refs.len()); }
    while i < refs.len()
        invariant i <= refs.len(),
            refs.deep_view() == proposal@.landed_refs, refs.deep_view().len() == refs.len(),
            forall|k: int| 0 <= k < i ==>
                #[trigger] resolution_present(proposal@.resolved_landed, refs.deep_view()[k]),
        decreases refs.len() - i,
    {
        if !reference_resolves(proposal.resolved_landed(), &refs[i]) {
            proof {
                assert(!resolution_present(proposal@.resolved_landed, proposal@.landed_refs[i as int]));
                if resolved_landed(proposal@) {
                    assert(resolution_present(proposal@.resolved_landed, proposal@.landed_refs[i as int]));
                }
            }
            return false;
        }
        i += 1;
    }
    true
}

fn contains_reference(refs: &[String], reference: &String) -> (contains: bool)
    ensures contains == refs.deep_view().contains(reference@),
{
    let mut i = 0;
    while i < refs.len()
        invariant i <= refs.len(),
            forall|j: int| 0 <= j < i ==> refs.deep_view()[j] != reference@,
        decreases refs.len() - i,
    {
        if refs[i].eq(reference) {
            proof {
                assert(refs.deep_view()[i as int] == reference@);
                assert(refs.deep_view().contains(reference@));
            }
            return true;
        }
        i += 1;
    }
    false
}

fn landed_bound(proposal: &AuthoredProposal) -> (bound: bool)
    ensures bound == bound_landed(proposal@),
{
    let Some(target) = proposal.target_revision() else { return false; };
    let rows = proposal.resolved_landed();
    let mut i = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while i < rows.len()
        invariant i <= rows.len(),
            rows.deep_view() == proposal@.resolved_landed, rows.deep_view().len() == rows.len(),
            proposal@.target_revision == Some(target@),
            forall|j: int| 0 <= j < i ==>
                !proposal@.landed_refs.contains(rows.deep_view()[j].reference)
                    || proposal@.target_revision != Some(rows.deep_view()[j].target_revision),
        decreases rows.len() - i,
    {
        let row = &rows[i];
        if contains_reference(proposal.landed_refs(), row.reference())
            && target == row.target_revision() {
            proof {
                assert(row@ == proposal@.resolved_landed[i as int]);
                assert(proposal@.landed_refs.contains(proposal@.resolved_landed[i as int].reference));
                assert(proposal@.target_revision == Some(proposal@.resolved_landed[i as int].target_revision));
                assert(bound_landed(proposal@));
            }
            return true;
        }
        i += 1;
    }
    false
}

fn contains_phase(proposal: &AuthoredProposal, phase: GateWhen) -> (present: bool)
    ensures present == has_phase(proposal@, phase),
{
    let definitions = proposal.gate_definitions();
    let mut i = 0;
    proof { assert(definitions.deep_view().len() == definitions.len()); }
    while i < definitions.len()
        invariant i <= definitions.len(),
            definitions.deep_view() == proposal@.gate_definitions, definitions.deep_view().len() == definitions.len(),
            forall|j: int| 0 <= j < i ==> definitions.deep_view()[j].when != phase,
        decreases definitions.len() - i,
    {
        if *definitions[i].when() == phase {
            proof {
                assert(proposal@.gate_definitions[i as int].when == phase);
                assert(has_phase(proposal@, phase));
            }
            return true;
        }
        i += 1;
    }
    false
}

fn all_applicable_pass(proposal: &AuthoredProposal, target: AuthoredStatus,
    now: &Integer) -> (passes: bool)
    ensures passes == applicable_gates_pass(proposal@, target, now@),
{
    let definitions = proposal.gate_definitions();
    let mut i = 0;
    proof { assert(definitions.deep_view().len() == definitions.len()); }
    while i < definitions.len()
        invariant i <= definitions.len(),
            definitions.deep_view() == proposal@.gate_definitions, definitions.deep_view().len() == definitions.len(),
            forall|j: int| 0 <= j < i
                && (target != AuthoredStatus::Applied || definitions.deep_view()[j].when == GateWhen::Land)
                ==> authored_gate_passes_spec(proposal@, definitions.deep_view()[j], now@),
        decreases definitions.len() - i,
    {
        let definition = &definitions[i];
        if (target != AuthoredStatus::Applied || *definition.when() == GateWhen::Land)
            && !authored_gate_passes(proposal, definition, now) {
            proof {
                assert(definition@ == proposal@.gate_definitions[i as int]);
                if applicable_gates_pass(proposal@, target, now@) {
                    assert(authored_gate_passes_spec(proposal@, proposal@.gate_definitions[i as int], now@));
                }
            }
            return false;
        }
        i += 1;
    }
    true
}

pub(super) fn visit_landing_problems(proposal: &AuthoredProposal, target: AuthoredStatus,
    now: &Integer, problems: &mut ProblemReducer)
    requires old(problems).invariant(),
    ensures final(problems).invariant(), final(problems).collect == old(problems).collect,
        final(problems).trace@ == old(problems).trace@ + landing_problems_spec(proposal@, target, now@),
{
    proof { broadcast use vstd::seq_lib::group_seq_properties; }
    let ghost initial = problems.trace@;
    if !target_present(proposal.target_revision()) || !nonblank_references(proposal.landed_refs()) {
        problems.emit(AuthoredProblem::ExplicitLanded);
    } else if !all_landed_resolve(proposal) {
        problems.emit(AuthoredProblem::ResolvableLanded);
    } else if !landed_bound(proposal) {
        problems.emit(AuthoredProblem::BoundLanded);
    } else {
        if target == AuthoredStatus::Applied {
            if !contains_phase(proposal, GateWhen::Land) { problems.emit(AuthoredProblem::LandGate); }
        } else if !contains_phase(proposal, GateWhen::Deploy) || !contains_phase(proposal, GateWhen::Use) {
            problems.emit(AuthoredProblem::DeployUseGates);
        }
        if !all_applicable_pass(proposal, target, now) { problems.emit(AuthoredProblem::CurrentGates); }
    }
    proof { assert(problems.trace@ =~= initial + landing_problems_spec(proposal@, target, now@)); }
}

} // verus!
