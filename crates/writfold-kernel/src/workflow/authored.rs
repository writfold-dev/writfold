use super::actor_present;
use writfold_domain::*;

pub fn authored_gate_valid(definition: &AuthoredGateDefinition) -> bool {
    actor_present(definition.by())
        && !definition.id().as_str().trim().is_empty()
        && !definition.claim().trim().is_empty()
        && !definition.check().trim().is_empty()
}

pub fn authored_ready(proposal: &AuthoredProposal) -> bool {
    *proposal.profile() == AuthoredProfile::LegacyLimited
        || (*proposal.research_present()
            && proposal.alternatives_count() >= &Integer::from(2)
            && !proposal.gate_definitions().is_empty()
            && proposal
                .gate_definitions()
                .iter()
                .enumerate()
                .all(|(index, definition)| {
                    authored_gate_valid(definition)
                        && !proposal.gate_definitions()[..index]
                            .iter()
                            .any(|prior| prior.id() == definition.id())
                }))
}

pub fn authored_gate_passes(
    proposal: &AuthoredProposal,
    definition: &AuthoredGateDefinition,
    now: &Integer,
) -> bool {
    if proposal.environment().trim().is_empty()
        || proposal.scope().trim().is_empty()
        || !material_present(proposal.proposal_revision())
    {
        return false;
    }
    let result = proposal.gate_results().iter().rev().find(|result| {
        result.definition() == definition
            && result.target_revision() == proposal.target_revision()
            && result.proposal_revision() == proposal.proposal_revision()
            && result.environment() == proposal.environment()
            && result.scope() == proposal.scope()
            && result
                .checker()
                .as_ref()
                .is_some_and(|id| !id.as_str().trim().is_empty())
            && (!definition
                .checker()
                .as_ref()
                .is_some_and(|id| !id.as_str().trim().is_empty())
                || result.checker() == definition.checker())
            && result.observed_at() <= now
    });
    result.is_some_and(|result| {
        *result.result() == GateOutcome::Pass
            && now <= result.expires_at()
            && !result.evidence_refs().is_empty()
            && result.evidence_refs().iter().all(|r| !r.trim().is_empty())
            && actor_present(result.actor())
    })
}

pub fn authored_edge(current: AuthoredStatus, target: AuthoredStatus) -> bool {
    use AuthoredStatus::*;
    matches!(
        (current, target),
        (Draft, Proposed)
            | (Proposed, Draft)
            | (Proposed, Accepted)
            | (Proposed, Rejected)
            | (Accepted, Landing)
            | (Accepted, Deferred)
            | (Landing, Applied)
            | (Applied, Verified)
            | (Applied, Regressed)
            | (Verified, Regressed)
    ) || (target == Superseded && current != target)
}

pub fn authored_supersession_valid(proposal: &AuthoredProposal) -> bool {
    if proposal.proposals().iter().enumerate().any(|(index, row)| {
        proposal.proposals()[..index]
            .iter()
            .any(|prior| prior.id() == row.id())
    }) || !proposal
        .proposals()
        .iter()
        .any(|row| row.id() == proposal.id())
    {
        return false;
    }
    let Some(mut target) = proposal.superseded_by().as_ref() else {
        return false;
    };
    // A resolving chain traverses at most every row once; a longer chain is cyclic.
    for _ in 0..proposal.proposals().len() {
        if target == proposal.id() {
            return false;
        }
        let Some(row) = proposal.proposals().iter().find(|row| row.id() == target) else {
            return false;
        };
        match row.superseded_by() {
            Some(next) => target = next,
            None => return true,
        }
    }
    false
}

pub fn authored_transition(
    proposal: &AuthoredProposal,
    target: AuthoredStatus,
    now: &Integer,
) -> bool {
    authored_edge(*proposal.status(), target) && authored_candidate_valid(proposal, target, now)
}

pub fn authored_candidate_problems(
    proposal: &AuthoredProposal,
    target: AuthoredStatus,
    now: &Integer,
) -> Vec<&'static str> {
    let mut problems = Vec::new();
    visit_candidate_problems(proposal, target, now, |problem| problems.push(problem));
    problems
}

pub fn authored_candidate_valid(
    proposal: &AuthoredProposal,
    target: AuthoredStatus,
    now: &Integer,
) -> bool {
    let mut valid = true;
    visit_candidate_problems(proposal, target, now, |_| valid = false);
    valid
}

fn material_present(revision: &Option<ProposalMaterialRevision>) -> bool {
    revision
        .as_ref()
        .is_some_and(|id| !id.as_str().trim().is_empty())
}

fn visit_candidate_problems(
    proposal: &AuthoredProposal,
    target: AuthoredStatus,
    now: &Integer,
    mut problem: impl FnMut(&'static str),
) {
    use AuthoredStatus::*;
    let decided = matches!(target, Accepted | Landing | Applied | Verified);
    if decided
        && (!material_present(proposal.accepted_revision())
            || proposal.accepted_revision() != proposal.proposal_revision())
    {
        problem("accepted proposal revision");
    }
    if target == Superseded && !authored_supersession_valid(proposal) {
        problem("resolving acyclic supersession");
    }
    if matches!(target, Accepted | Rejected | Landing | Applied) && !actor_present(proposal.actor())
    {
        problem("recorded actors");
    }
    if !authored_ready(proposal)
        && matches!(target, Proposed | Accepted | Landing | Applied | Verified)
    {
        problem("review readiness");
    }
    if decided && !actor_present(proposal.acceptor()) {
        problem("acceptor");
    }
    if matches!(target, Landing | Applied | Verified) && !actor_present(proposal.executor()) {
        problem("executor");
    }
    if matches!(target, Applied | Verified) {
        visit_landing_problems(proposal, target, now, &mut problem);
    }
    if target == Regressed && proposal.regression_note().trim().is_empty() {
        problem("regression note");
    }
}

fn visit_landing_problems(
    proposal: &AuthoredProposal,
    target: AuthoredStatus,
    now: &Integer,
    problem: &mut impl FnMut(&'static str),
) {
    if !proposal
        .target_revision()
        .as_ref()
        .is_some_and(|id| !id.as_str().trim().is_empty())
        || proposal.landed_refs().is_empty()
        || proposal.landed_refs().iter().any(|r| r.trim().is_empty())
    {
        problem("explicit landed references");
    } else if !proposal.landed_refs().iter().all(|reference| {
        proposal
            .resolved_landed()
            .iter()
            .any(|resolution| resolution.reference() == reference)
    }) {
        problem("resolvable landed references");
    } else if !proposal.resolved_landed().iter().any(|resolution| {
        proposal.landed_refs().contains(resolution.reference())
            && proposal.target_revision().as_ref() == Some(resolution.target_revision())
    }) {
        problem("landed references bound to the current target revision");
    } else {
        let definitions = proposal.gate_definitions();
        let applicable = |definition: &&AuthoredGateDefinition| {
            target != AuthoredStatus::Applied || *definition.when() == GateWhen::Land
        };
        if target == AuthoredStatus::Applied {
            if !definitions.iter().any(|d| *d.when() == GateWhen::Land) {
                problem("a land gate");
            }
        } else if !definitions.iter().any(|d| *d.when() == GateWhen::Deploy)
            || !definitions.iter().any(|d| *d.when() == GateWhen::Use)
        {
            problem("deploy and use gates");
        }
        if !definitions
            .iter()
            .filter(applicable)
            .all(|definition| authored_gate_passes(proposal, definition, now))
        {
            problem("current fully scoped gates");
        }
    }
}
