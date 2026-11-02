//! Design acceptance is separate from implementation binding and release authority.
use crate::workflow::{authored_candidate_problems, authored_candidate_valid, authored_edge};
use writfold_domain::*;

fn active(status: AuthoredStatus) -> bool {
    matches!(
        status,
        AuthoredStatus::Accepted
            | AuthoredStatus::Landing
            | AuthoredStatus::Applied
            | AuthoredStatus::Verified
    )
}
fn finalized(status: AuthoredStatus) -> bool {
    matches!(status, AuthoredStatus::Applied | AuthoredStatus::Verified)
}
fn bindable(status: AuthoredStatus) -> bool {
    matches!(status, AuthoredStatus::Accepted | AuthoredStatus::Landing)
}

pub fn implementation_binding_valid(
    proposal: &DesignProposal,
    binding: &ImplementationBinding,
    now: &Integer,
) -> bool {
    // The reference compares the spelling of design identity carried in the authored material slot.
    // These nominal types deliberately remain distinct at the Rust boundary.
    proposal
        .authored()
        .proposal_revision()
        .as_ref()
        .is_some_and(|revision| revision.as_str() == binding.design_revision().as_str())
        && proposal
            .authored()
            .accepted_revision()
            .as_ref()
            .is_some_and(|revision| revision.as_str() == binding.design_revision().as_str())
        && binding.source() == proposal.target_source()
        && !binding.actor().as_str().trim().is_empty()
        && !binding.source().as_str().trim().is_empty()
        && !binding.design_revision().as_str().trim().is_empty()
        && !binding.target_revision().as_str().trim().is_empty()
        && binding.observed_at() <= now
}

pub fn implementation_bound(proposal: &DesignProposal, now: &Integer) -> bool {
    proposal.binding().as_ref().is_some_and(|binding| {
        implementation_binding_valid(proposal, binding, now)
            && proposal.authored().target_revision().as_ref() == Some(binding.target_revision())
    })
}

pub fn design_candidate_valid(
    proposal: &DesignProposal,
    target: AuthoredStatus,
    now: &Integer,
) -> bool {
    authored_candidate_valid(proposal.authored(), target, now)
        && (!active(target) || !proposal.target_source().as_str().trim().is_empty())
        && (!finalized(target) || implementation_bound(proposal, now))
}

pub fn design_candidate_problems(
    proposal: &DesignProposal,
    target: AuthoredStatus,
    now: &Integer,
) -> Vec<&'static str> {
    let mut problems = authored_candidate_problems(proposal.authored(), target, now);
    if active(target) && proposal.target_source().as_str().trim().is_empty() {
        problems.push("known implementation source");
    }
    if finalized(target) && !implementation_bound(proposal, now) {
        problems.push("implementation binding to the accepted design and current target");
    }
    problems
}

pub fn design_transition(proposal: &DesignProposal, target: AuthoredStatus, now: &Integer) -> bool {
    authored_edge(*proposal.authored().status(), target)
        && design_candidate_valid(proposal, target, now)
}

pub fn design_with_binding(
    proposal: &DesignProposal,
    binding: &ImplementationBinding,
) -> DesignProposal {
    let old = proposal.authored();
    let authored = AuthoredProposal::new(
        *old.status(),
        *old.profile(),
        *old.research_present(),
        old.alternatives_count().clone(),
        Some(binding.target_revision().clone()),
        old.gate_definitions().clone(),
        old.gate_results().clone(),
        old.actor().clone(),
        old.acceptor().clone(),
        old.executor().clone(),
        old.verifier().clone(),
        Vec::new(),
        old.regression_note().clone(),
        old.id().clone(),
        old.superseded_by().clone(),
        old.proposals().clone(),
        old.accepted_revision().clone(),
        old.proposal_revision().clone(),
        old.environment().clone(),
        old.scope().clone(),
        Vec::new(),
    );
    DesignProposal::new(
        authored,
        proposal.target_source().clone(),
        Some(binding.clone()),
    )
}

pub fn bind_implementation(
    current: &DesignProposal,
    command: &DesignBindingCommand,
    now: &Integer,
) -> Option<DesignProposal> {
    if current == command.expected()
        && bindable(*current.authored().status())
        && design_candidate_valid(current, *current.authored().status(), now)
        && implementation_binding_valid(current, command.binding(), now)
    {
        Some(design_with_binding(current, command.binding()))
    } else {
        None
    }
}

pub fn design_reopened(proposal: &DesignProposal, actor: &ActorId) -> DesignProposal {
    let old = proposal.authored();
    let authored = AuthoredProposal::new(
        AuthoredStatus::Proposed,
        *old.profile(),
        *old.research_present(),
        old.alternatives_count().clone(),
        None,
        old.gate_definitions().clone(),
        old.gate_results().clone(),
        Some(actor.clone()),
        None,
        None,
        None,
        Vec::new(),
        old.regression_note().clone(),
        old.id().clone(),
        old.superseded_by().clone(),
        old.proposals().clone(),
        None,
        old.proposal_revision().clone(),
        old.environment().clone(),
        old.scope().clone(),
        Vec::new(),
    );
    DesignProposal::new(authored, proposal.target_source().clone(), None)
}

pub fn reopen_design(
    current: &DesignProposal,
    command: &DesignReopenCommand,
) -> Option<DesignProposal> {
    if current == command.expected()
        && bindable(*current.authored().status())
        && !command.actor().as_str().trim().is_empty()
        && !command.reason().trim().is_empty()
    {
        Some(design_reopened(current, command.actor()))
    } else {
        None
    }
}
