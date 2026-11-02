//! Pure admission, replay and authored judgments from the reference workflow.
mod authored;
pub mod result;
mod selection;
use crate::checker::{
    check_matches_snapshot, context_matches, retracted, reusable, usable_for, usable_for_snapshot,
};
use crate::wellformed::well_formed;
pub use authored::*;
use result::replay_outcome;
pub use result::{Application, SelectionResult};
pub use selection::*;
use writfold_domain::*;

pub fn preserves_prefix(base: &KB, candidate: &KB) -> bool {
    candidate.observations().starts_with(base.observations())
        && candidate.claims().starts_with(base.claims())
        && candidate.attestations().starts_with(base.attestations())
        && candidate.enumerations().starts_with(base.enumerations())
        && candidate.derivations().starts_with(base.derivations())
        && candidate.corrections().starts_with(base.corrections())
        && candidate.receipts().starts_with(base.receipts())
}

pub(crate) fn kb_at(kb: &KB, snapshot: &SnapshotId) -> KB {
    KB::new(
        snapshot.clone(),
        kb.observations().clone(),
        kb.claims().clone(),
        kb.attestations().clone(),
        kb.enumerations().clone(),
        kb.derivations().clone(),
        kb.corrections().clone(),
        kb.receipts().clone(),
    )
}

pub(crate) fn unique<T: PartialEq>(items: &[T]) -> bool {
    items
        .iter()
        .enumerate()
        .all(|(index, item)| !items[..index].contains(item))
}

pub fn plan_valid(kb: &KB, context: &Context, plan: &Plan) -> bool {
    if !context_matches(kb, context)
        || !well_formed(plan.candidate())
        || plan.candidate().snapshot() == kb.snapshot()
        || !preserves_prefix(kb, plan.candidate())
    {
        return false;
    }
    let Some(claim) = find_claim(plan.candidate(), plan.claim()) else {
        return false;
    };
    let authority = plan.authority();
    let added = &plan.candidate().claims()[kb.claims().len()..];
    if authority.actor().as_str().trim().is_empty()
        || authority.target() != plan.claim()
        || !context
            .policy()
            .authorized_actors()
            .contains(authority.actor())
        || authority.statement() != claim.statement()
        || authority.policy() != context.policy().id()
        || authority.intended_use() != context.intended_use()
        || authority.context() != context
        || authority.approved() != plan.candidate()
        || authority.approvals() != added
    {
        return false;
    }
    let candidate = plan.candidate();
    let snapshot = candidate.snapshot();
    !retracted(candidate, context, plan.claim())
        && candidate.derivations().iter().any(|derivation| {
            derivation.claim() == plan.claim() && derivation.witness() == plan.witness()
        })
        && check_matches_snapshot(
            candidate,
            context,
            snapshot,
            plan.witness(),
            claim.statement(),
        )
        && usable_for_snapshot(candidate, context, snapshot, plan.claim())
        && added.iter().all(|item| {
            candidate.derivations().iter().any(|derivation| {
                item.matches_ref(derivation.claim())
                    && !retracted(candidate, context, derivation.claim())
                    && check_matches_snapshot(
                        candidate,
                        context,
                        snapshot,
                        derivation.witness(),
                        item.statement(),
                    )
                    && usable_for_snapshot(candidate, context, snapshot, derivation.claim())
            })
        })
}

pub fn apply_plan<'a>(
    kb: &'a KB,
    context: &Context,
    journal: &'a [JournalEntry],
    plan: &Plan,
    trusted_approvals: &[ModelApproval],
) -> Application<'a> {
    if let Some(entry) = journal.iter().find(|entry| entry.plan().id() == plan.id()) {
        return Application::unchanged(kb, journal, replay_outcome(entry.plan(), plan));
    }
    if plan.expected_snapshot() != kb.snapshot() {
        return Application::unchanged(kb, journal, CommitOutcome::PredecessorConflict);
    }
    if !trusted_approvals
        .iter()
        .any(|approval| approval.plan() == plan)
        || !plan_valid(kb, context, plan)
    {
        return Application::unchanged(kb, journal, CommitOutcome::Rejected);
    }
    let mut updated = Vec::with_capacity(journal.len() + 1);
    updated.push(JournalEntry::new(plan.clone()));
    updated.extend_from_slice(journal);
    Application::applied(plan.candidate().clone(), updated)
}

pub fn transition(kb: &KB, context: &Context, proposal: &Proposal, target: ProposalStatus) -> bool {
    use ProposalStatus::*;
    let legal = matches!(
        (*proposal.status(), target),
        (Draft, Proposed)
            | (Proposed, Approved)
            | (Proposed, Rejected)
            | (Approved, Applied)
            | (Approved, Rejected)
            | (Applied, Verified)
    );
    if !legal || !actor_present(proposal.actor()) || !usable_for(kb, context, proposal.target()) {
        return false;
    }
    if !proposal.gates().iter().all(|gate| {
        kb.receipts()
            .iter()
            .find(|r| r.id() == gate)
            .is_some_and(|receipt| reusable(kb, context, proposal.target(), receipt))
    }) {
        return false;
    }
    if matches!(target, Approved | Applied | Verified) && proposal.gates().is_empty() {
        return false;
    }
    if target == Applied && !actor_present(proposal.executor()) {
        return false;
    }
    target != Verified
        || (actor_present(proposal.executor())
            && actor_present(proposal.acceptor())
            && proposal.executor() != proposal.acceptor())
}

pub fn conclude(investigation: &Investigation) -> bool {
    !investigation.checked_against().trim().is_empty()
        && !investigation.scope_and_limits().trim().is_empty()
        && investigation.hypotheses().iter().all(|hypothesis| {
            !hypothesis.text().trim().is_empty() && *hypothesis.status() != HypothesisStatus::Open
        })
}

pub(crate) fn actor_present(actor: &Option<ActorId>) -> bool {
    actor
        .as_ref()
        .is_some_and(|actor| !actor.as_str().trim().is_empty())
}
