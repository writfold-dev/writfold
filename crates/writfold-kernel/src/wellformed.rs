//! Whole-candidate invariants, independent of support and admission policy.
use std::collections::BTreeSet;
use writfold_domain::*;

pub fn nonempty(value: &str) -> bool {
    !value.is_empty()
}
pub fn natural(value: &Integer) -> bool {
    !value.is_negative()
}
pub fn ttl_wf(ttl: &Ttl) -> bool {
    match ttl {
        Ttl::ExpireAfter(secs) => natural(secs),
        _ => true,
    }
}
pub fn scope_wf(scope: &Scope) -> bool {
    nonempty(scope.project().as_str())
}

pub(crate) fn unique<T: Ord>(values: impl IntoIterator<Item = T>) -> bool {
    let mut seen = BTreeSet::new();
    values.into_iter().all(|value| seen.insert(value))
}

pub fn formula_wf(formula: &Formula, kb: &KB) -> bool {
    match formula {
        Formula::Atomic(atom) => nonempty(atom.subject()) && nonempty(atom.predicate()),
        Formula::Truth => true,
        Formula::Conjunction { left, right } => formula_wf(left, kb) && formula_wf(right, kb),
        Formula::All {
            enumeration,
            predicate,
        } => nonempty(predicate) && kb.enumerations().iter().any(|e| e.id() == enumeration),
        Formula::Absent { enumeration } | Formula::Inventory { enumeration } => {
            kb.enumerations().iter().any(|e| e.id() == enumeration)
        }
    }
}

pub fn statement_wf(statement: &Statement, kb: &KB) -> bool {
    scope_wf(statement.scope())
        && formula_wf(statement.formula(), kb)
        && (*statement.polarity() == Polarity::Positive
            || matches!(statement.formula(), Formula::Atomic(_)))
}

pub fn context_wf(context: &Context) -> bool {
    context.semantic_version() == SEMANTIC_VERSION
        && natural(context.now())
        && context.ruleset().as_str() == "core-2"
        && context.checker().as_str() == "kctx-2"
        && scope_wf(context.target())
        && nonempty(context.intended_use())
        && unique(context.sources().iter().map(SourceSelection::source))
        && context.sources().iter().all(|source| {
            nonempty(source.source().as_str())
                && nonempty(source.revision().as_str())
                && nonempty(source.content().as_str())
        })
        && unique(context.policy().authorized_actors())
        && context
            .policy()
            .authorized_actors()
            .iter()
            .all(|actor| nonempty(actor.as_str()))
        && ttl_wf(context.policy().ttl())
}

pub fn witness_wf(witness: &Witness, kb: &KB) -> bool {
    match witness {
        Witness::Attest { attestation } => kb.attestations().iter().any(|a| a.id() == attestation),
        Witness::Truth { scope } => scope_wf(scope),
        Witness::And { left, right } => witness_wf(left, kb) && witness_wf(right, kb),
        Witness::Specialize { scope, premise } => scope_wf(scope) && witness_wf(premise, kb),
        Witness::All {
            enumeration,
            predicate,
            premise,
        } => {
            kb.enumerations().iter().any(|e| e.id() == enumeration)
                && nonempty(predicate)
                && witness_wf(premise, kb)
        }
        Witness::Absent {
            enumeration,
            premise,
        } => kb.enumerations().iter().any(|e| e.id() == enumeration) && witness_wf(premise, kb),
    }
}

pub fn dependency_wf(dependency: &Dependency, kb: &KB) -> bool {
    match dependency {
        Dependency::Observation(value) => kb.observations().contains(value),
        Dependency::Claim(value) => kb.claims().contains(value),
        Dependency::Attestation(value) => kb.attestations().contains(value),
        Dependency::Enumeration(value) => kb.enumerations().contains(value),
        Dependency::Corrections { target, events } => {
            find_claim(kb, target).is_some()
                && events
                    .iter()
                    .all(|event| kb.corrections().contains(event) && event.target() == target)
        }
        Dependency::Context(value) => context_wf(value) && value.now() == &Integer::from(0),
    }
}

fn attestation_wf(attestation: &Attestation, kb: &KB) -> bool {
    let Some(claim) = find_claim(kb, attestation.claim()) else {
        return false;
    };
    if claim.statement() != attestation.statement()
        || !matches!(
            attestation.statement().formula(),
            Formula::Atomic(_) | Formula::Inventory { .. }
        )
        || !nonempty(attestation.authority().as_str())
        || find_observation(kb, attestation.observation()).is_none()
    {
        return false;
    }
    if let Formula::Inventory { enumeration } = attestation.statement().formula() {
        return kb.enumerations().iter().any(|e| {
            e.id() == enumeration
                && *attestation.statement().polarity() == Polarity::Positive
                && attestation.observation() == e.observation()
                && attestation.statement().scope() == e.scope()
        });
    }
    true
}

fn enumeration_wf(enumeration: &Enumeration, kb: &KB) -> bool {
    let Some(observation) = find_observation(kb, enumeration.observation()) else {
        return false;
    };
    observation.source() == enumeration.source()
        && observation.revision() == enumeration.revision()
        && nonempty(enumeration.query())
        && nonempty(enumeration.method())
        && scope_wf(enumeration.scope())
        && enumeration.members() == enumeration.universe()
        && unique(enumeration.members())
        && enumeration.members().iter().all(|m| nonempty(m))
}

fn corrections_wf(kb: &KB) -> bool {
    for correction in kb.corrections() {
        if find_claim(kb, correction.target()).is_none() || !natural(correction.recorded_at()) {
            return false;
        }
        match (correction.kind(), correction.replacement()) {
            (CorrectionKind::Supersede, Some(replacement))
                if find_claim(kb, replacement).is_some() => {}
            (CorrectionKind::Supersede, _) => return false,
            (_, Some(_)) => return false,
            (_, None) => {}
        }
    }
    let mut frontier = Vec::new();
    let mut seen = BTreeSet::new();
    for claim in kb.claims() {
        let start = (claim.id(), claim.revision());
        frontier.clear();
        frontier.push(start);
        seen.clear();
        while let Some(current) = frontier.pop() {
            if !seen.insert(current) {
                continue;
            }
            for correction in kb.corrections() {
                if *correction.kind() == CorrectionKind::Supersede
                    && (correction.target().id(), correction.target().rev()) == current
                    && let Some(replacement) = correction.replacement()
                {
                    let next = (replacement.id(), replacement.rev());
                    if next == start {
                        return false;
                    }
                    frontier.push(next);
                }
            }
        }
    }
    true
}

pub fn well_formed(kb: &KB) -> bool {
    if !unique(kb.observations().iter().map(Observation::id))
        || !unique(kb.claims().iter().map(|c| (c.id(), c.revision())))
        || !unique(kb.attestations().iter().map(Attestation::id))
        || !unique(kb.enumerations().iter().map(Enumeration::id))
        || !unique(kb.derivations().iter().map(Derivation::id))
        || !unique(kb.corrections().iter().map(Correction::id))
        || !unique(kb.receipts().iter().map(Receipt::id))
    {
        return false;
    }
    kb.observations().iter().all(|o| {
        nonempty(o.source().as_str())
            && nonempty(o.revision().as_str())
            && nonempty(o.content().as_str())
            && natural(o.observed_at())
    }) && kb.claims().iter().all(|c| statement_wf(c.statement(), kb))
        && kb.attestations().iter().all(|a| attestation_wf(a, kb))
        && kb.enumerations().iter().all(|e| enumeration_wf(e, kb))
        && kb
            .derivations()
            .iter()
            .all(|d| find_claim(kb, d.claim()).is_some() && witness_wf(d.witness(), kb))
        && corrections_wf(kb)
        && kb.receipts().iter().all(|r| {
            find_claim(kb, r.claim()).is_some()
                && context_wf(r.context())
                && witness_wf(r.witness(), kb)
                && natural(r.expires_at())
                && r.dependencies().iter().all(|d| dependency_wf(d, kb))
        })
}
