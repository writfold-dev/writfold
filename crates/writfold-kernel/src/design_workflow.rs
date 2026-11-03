//! Design acceptance is separate from implementation binding and release authority.
#[cfg(verus_keep_ghost)]
use crate::workflow::{
    AuthoredProblem, authored_candidate_problems_spec, authored_candidate_valid_spec,
    authored_edge_spec, problem_text,
};
use crate::workflow::{authored_candidate_problems, authored_candidate_valid, authored_edge};
use vstd::prelude::*;
use writfold_domain::{Integer, *};

verus! {
pub open spec fn active_spec(status: AuthoredStatus) -> bool {
    matches!(status, AuthoredStatus::Accepted | AuthoredStatus::Landing
        | AuthoredStatus::Applied | AuthoredStatus::Verified)
}
pub open spec fn finalized_spec(status: AuthoredStatus) -> bool {
    matches!(status, AuthoredStatus::Applied | AuthoredStatus::Verified)
}
pub open spec fn bindable_spec(status: AuthoredStatus) -> bool {
    matches!(status, AuthoredStatus::Accepted | AuthoredStatus::Landing)
}
pub open spec fn implementation_binding_valid_spec(p: DesignProposalView,
    b: ImplementationBindingView, now: int) -> bool {
    p.authored.proposal_revision == Some(b.design_revision)
        && p.authored.accepted_revision == Some(b.design_revision)
        && b.source == p.target_source
        && writfold_domain::text::nonblank_spec(b.actor)
        && writfold_domain::text::nonblank_spec(b.source)
        && writfold_domain::text::nonblank_spec(b.design_revision)
        && writfold_domain::text::nonblank_spec(b.target_revision)
        && b.observed_at <= now
}
pub open spec fn implementation_bound_spec(p: DesignProposalView, now: int) -> bool {
    match p.binding {
        Some(b) => implementation_binding_valid_spec(p, b, now)
            && p.authored.target_revision == Some(b.target_revision),
        None => false,
    }
}
pub open spec fn design_candidate_valid_spec(p: DesignProposalView,
    target: AuthoredStatus, now: int) -> bool {
    authored_candidate_valid_spec(p.authored, target, now)
        && (active_spec(target) ==> writfold_domain::text::nonblank_spec(p.target_source))
        && (finalized_spec(target) ==> implementation_bound_spec(p, now))
}
pub open spec fn design_candidate_problems_spec(p: DesignProposalView,
    target: AuthoredStatus, now: int) -> Seq<Seq<char>> {
    authored_candidate_problems_spec(p.authored, target, now).map_values(
        |problem: AuthoredProblem| problem_text(problem))
        + (if active_spec(target) && !writfold_domain::text::nonblank_spec(p.target_source) {
            seq!["known implementation source"@]
        } else { seq![] })
        + (if finalized_spec(target) && !implementation_bound_spec(p, now) {
            seq!["implementation binding to the accepted design and current target"@]
        } else { seq![] })
}
pub open spec fn design_with_binding_spec(p: DesignProposalView,
    b: ImplementationBindingView) -> DesignProposalView {
    DesignProposalView {
        authored: AuthoredProposalView { target_revision: Some(b.target_revision),
            landed_refs: seq![], resolved_landed: seq![], ..p.authored },
        binding: Some(b), ..p
    }
}
pub open spec fn design_reopened_spec(p: DesignProposalView, actor: Seq<char>) -> DesignProposalView {
    DesignProposalView {
        authored: AuthoredProposalView { status: AuthoredStatus::Proposed, target_revision: None,
            actor: Some(actor), acceptor: None, executor: None, verifier: None,
            landed_refs: seq![], accepted_revision: None, resolved_landed: seq![], ..p.authored },
        binding: None, ..p
    }
}
pub open spec fn bind_implementation_spec(current: DesignProposalView,
    command: DesignBindingCommandView, now: int) -> Option<DesignProposalView> {
    if current == command.expected && bindable_spec(current.authored.status)
        && design_candidate_valid_spec(current, current.authored.status, now)
        && implementation_binding_valid_spec(current, command.binding, now) {
        Some(design_with_binding_spec(current, command.binding))
    } else { None }
}
pub open spec fn reopen_design_spec(current: DesignProposalView,
    command: DesignReopenCommandView) -> Option<DesignProposalView> {
    if current == command.expected && bindable_spec(current.authored.status)
        && writfold_domain::text::nonblank_spec(command.actor)
        && writfold_domain::text::nonblank_spec(command.reason) {
        Some(design_reopened_spec(current, command.actor))
    } else { None }
}
fn active(status: AuthoredStatus) -> (active: bool)
    ensures active == active_spec(status),
{
    matches!(status, AuthoredStatus::Accepted | AuthoredStatus::Landing
        | AuthoredStatus::Applied | AuthoredStatus::Verified)
}
fn finalized(status: AuthoredStatus) -> (finalized: bool)
    ensures finalized == finalized_spec(status),
{
    matches!(status, AuthoredStatus::Applied | AuthoredStatus::Verified)
}
fn bindable(status: AuthoredStatus) -> (bindable: bool)
    ensures bindable == bindable_spec(status),
{
    matches!(status, AuthoredStatus::Accepted | AuthoredStatus::Landing)
}
pub fn implementation_binding_valid(proposal: &DesignProposal,
    binding: &ImplementationBinding, now: &Integer) -> (valid: bool)
    ensures valid == implementation_binding_valid_spec(proposal@, binding@, now@),
{
    // Compare spelling only here; the two nominal revision types remain distinct.
    (match proposal.authored().proposal_revision() {
        Some(revision) => revision.as_str() == binding.design_revision().as_str(),
        None => false,
    }) && (match proposal.authored().accepted_revision() {
        Some(revision) => revision.as_str() == binding.design_revision().as_str(),
        None => false,
    }) && binding.source() == proposal.target_source()
        && writfold_domain::text::nonblank(binding.actor().as_str())
        && writfold_domain::text::nonblank(binding.source().as_str())
        && writfold_domain::text::nonblank(binding.design_revision().as_str())
        && writfold_domain::text::nonblank(binding.target_revision().as_str())
        && binding.observed_at() <= now
}
pub fn implementation_bound(proposal: &DesignProposal, now: &Integer) -> (bound: bool)
    ensures bound == implementation_bound_spec(proposal@, now@),
{
    match proposal.binding() {
        Some(binding) => implementation_binding_valid(proposal, binding, now)
            && (match proposal.authored().target_revision() {
                Some(target) => target == binding.target_revision(),
                None => false,
            }),
        None => false,
    }
}
pub fn design_candidate_valid(proposal: &DesignProposal,
    target: AuthoredStatus, now: &Integer) -> (valid: bool)
    ensures valid == design_candidate_valid_spec(proposal@, target, now@),
{
    authored_candidate_valid(proposal.authored(), target, now)
        && (!active(target) || writfold_domain::text::nonblank(proposal.target_source().as_str()))
        && (!finalized(target) || implementation_bound(proposal, now))
}
pub fn design_candidate_problems(proposal: &DesignProposal,
    target: AuthoredStatus, now: &Integer) -> (problems: Vec<&'static str>)
    ensures problems.deep_view() == design_candidate_problems_spec(proposal@, target, now@),
{
    let mut problems = authored_candidate_problems(proposal.authored(), target, now);
    proof {
        assert(problems.deep_view() =~= authored_candidate_problems_spec(proposal@.authored, target, now@)
            .map_values(|problem: AuthoredProblem| problem_text(problem)));
    }
    if active(target) && !writfold_domain::text::nonblank(proposal.target_source().as_str()) {
        problems.push("known implementation source");
    }
    if finalized(target) && !implementation_bound(proposal, now) {
        problems.push("implementation binding to the accepted design and current target");
    }
    problems
}
pub fn design_transition(proposal: &DesignProposal, target: AuthoredStatus, now: &Integer) -> (valid: bool)
    ensures valid == (authored_edge_spec(proposal@.authored.status, target)
        && design_candidate_valid_spec(proposal@, target, now@)),
{
    authored_edge(*proposal.authored().status(), target)
        && design_candidate_valid(proposal, target, now)
}
fn clone_gate_definitions(rows: &[AuthoredGateDefinition]) -> (cloned: Vec<AuthoredGateDefinition>)
    ensures cloned.deep_view() == rows.deep_view(),
{
    let mut cloned = Vec::with_capacity(rows.len());
    let mut i = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while i < rows.len()
        invariant i <= rows.len(), cloned.deep_view() == rows.deep_view().take(i as int),
            rows.deep_view().len() == rows.len(),
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
fn clone_gate_results(rows: &[AuthoredGateResult]) -> (cloned: Vec<AuthoredGateResult>)
    ensures cloned.deep_view() == rows.deep_view(),
{
    let mut cloned = Vec::with_capacity(rows.len());
    let mut i = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while i < rows.len()
        invariant i <= rows.len(), cloned.deep_view() == rows.deep_view().take(i as int),
            rows.deep_view().len() == rows.len(),
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
fn clone_replacements(rows: &[AuthoredReplacement]) -> (cloned: Vec<AuthoredReplacement>)
    ensures cloned.deep_view() == rows.deep_view(),
{
    let mut cloned = Vec::with_capacity(rows.len());
    let mut i = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while i < rows.len()
        invariant i <= rows.len(), cloned.deep_view() == rows.deep_view().take(i as int),
            rows.deep_view().len() == rows.len(),
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
pub fn design_with_binding(proposal: &DesignProposal,
    binding: &ImplementationBinding) -> (result: DesignProposal)
    ensures result@ == design_with_binding_spec(proposal@, binding@),
{
    proof { broadcast use vstd::std_specs::vec::group_vec_axioms; }
    let old = proposal.authored();
    let cleared_refs = Vec::<String>::new();
    let cleared_resolutions = Vec::<AuthoredLanded>::new();
    proof {
        assert(cleared_refs.deep_view() =~= Seq::<Seq<char>>::empty());
        assert(cleared_resolutions.deep_view() =~= Seq::<AuthoredLandedView>::empty());
    }
    let authored = AuthoredProposal::new(
        *old.status(), *old.profile(), *old.research_present(), old.alternatives_count().clone(),
        Some(binding.target_revision().clone()), clone_gate_definitions(old.gate_definitions()), clone_gate_results(old.gate_results()),
        old.actor().clone(), old.acceptor().clone(), old.executor().clone(), old.verifier().clone(),
        cleared_refs, old.regression_note().clone(), old.id().clone(), old.superseded_by().clone(),
        clone_replacements(old.proposals()), old.accepted_revision().clone(), old.proposal_revision().clone(),
        old.environment().clone(), old.scope().clone(), cleared_resolutions,
    );
    proof { assert(authored@ == design_with_binding_spec(proposal@, binding@).authored); }
    DesignProposal::new(authored, proposal.target_source().clone(), Some(binding.clone()))
}
pub fn bind_implementation(current: &DesignProposal,
    command: &DesignBindingCommand, now: &Integer) -> (result: Option<DesignProposal>)
    ensures result.deep_view() == bind_implementation_spec(current@, command@, now@),
{
    if current == command.expected()
        && bindable(*current.authored().status())
        && design_candidate_valid(current, *current.authored().status(), now)
        && implementation_binding_valid(current, command.binding(), now)
    {
        Some(design_with_binding(current, command.binding()))
    } else { None }
}
pub fn design_reopened(proposal: &DesignProposal, actor: &ActorId) -> (result: DesignProposal)
    ensures result@ == design_reopened_spec(proposal@, actor@),
{
    proof { broadcast use vstd::std_specs::vec::group_vec_axioms; }
    let old = proposal.authored();
    let cleared_refs = Vec::<String>::new();
    let cleared_resolutions = Vec::<AuthoredLanded>::new();
    proof {
        assert(cleared_refs.deep_view() =~= Seq::<Seq<char>>::empty());
        assert(cleared_resolutions.deep_view() =~= Seq::<AuthoredLandedView>::empty());
    }
    let authored = AuthoredProposal::new(
        AuthoredStatus::Proposed, *old.profile(), *old.research_present(), old.alternatives_count().clone(),
        None, clone_gate_definitions(old.gate_definitions()), clone_gate_results(old.gate_results()), Some(actor.clone()),
        None, None, None, cleared_refs, old.regression_note().clone(), old.id().clone(),
        old.superseded_by().clone(), clone_replacements(old.proposals()), None, old.proposal_revision().clone(),
        old.environment().clone(), old.scope().clone(), cleared_resolutions,
    );
    proof { assert(authored@ == design_reopened_spec(proposal@, actor@).authored); }
    DesignProposal::new(authored, proposal.target_source().clone(), None)
}
pub fn reopen_design(current: &DesignProposal,
    command: &DesignReopenCommand) -> (result: Option<DesignProposal>)
    ensures result.deep_view() == reopen_design_spec(current@, command@),
{
    if current == command.expected()
        && bindable(*current.authored().status())
        && writfold_domain::text::nonblank(command.actor().as_str())
        && writfold_domain::text::nonblank(command.reason())
    {
        Some(design_reopened(current, command.actor()))
    } else { None }
}
} // verus!
