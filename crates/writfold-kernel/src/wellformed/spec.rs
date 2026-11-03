//! Declarative counterparts of Python wellformed.py and Lean WellFormed.
//! Nominal grammar is enforced by domain constructors, not strengthened here.
//! Python performs reference resolution as part of formula/witness checks;
//! Lean splits it into formulaWf/formulaRefs and witnessWf/witnessRefs.
//! Runtime integer types exclude Python's bool-as-int case by construction.
//! Snapshot alignment belongs to context_matches, not context_wf or receipts.
//! Policy requirement flags, observation outcomes and receipt results are not
//! well-formedness restrictions: changing them alone cannot reject a candidate.
use vstd::prelude::*;
use writfold_domain::*;

verus! {
pub open spec fn nonempty_spec(text: Seq<char>) -> bool { text.len() != 0 }
pub open spec fn natural_spec(value: int) -> bool { value >= 0 }
pub open spec fn ttl_wf_spec(ttl: TtlView) -> bool {
    match ttl { TtlView::ExpireAfter(seconds) => natural_spec(seconds), _ => true }
}
pub open spec fn scope_wf_spec(scope: ScopeView) -> bool { nonempty_spec(scope.project) }
pub open spec fn enumeration_present(kb: KBView, id: Seq<char>) -> bool {
    exists|i: int| 0 <= i < kb.enumerations.len() && kb.enumerations[i].id == id
}
pub open spec fn attestation_present(kb: KBView, id: Seq<char>) -> bool {
    exists|i: int| 0 <= i < kb.attestations.len() && kb.attestations[i].id == id
}
pub open spec fn claim_present(kb: KBView, reference: ClaimRefView) -> bool {
    exists|i: int| 0 <= i < kb.claims.len() && claim_matches_spec(kb.claims[i], reference)
}
pub open spec fn observation_present(kb: KBView, id: Seq<char>) -> bool {
    exists|i: int| 0 <= i < kb.observations.len() && kb.observations[i].id == id
}
pub open spec fn formula_wf_spec(formula: FormulaView, kb: KBView) -> bool
    decreases formula,
{
    match formula {
        FormulaView::Atomic(atom) => nonempty_spec(atom.subject) && nonempty_spec(atom.predicate),
        FormulaView::Truth => true,
        FormulaView::Conjunction { left, right } => formula_wf_spec(*left, kb) && formula_wf_spec(*right, kb),
        FormulaView::All { enumeration, predicate } => enumeration_present(kb, enumeration) && nonempty_spec(predicate),
        FormulaView::Absent { enumeration } | FormulaView::Inventory { enumeration } => enumeration_present(kb, enumeration),
    }
}
pub open spec fn statement_wf_spec(statement: StatementView, kb: KBView) -> bool {
    scope_wf_spec(statement.scope) && formula_wf_spec(statement.formula, kb)
        && (statement.polarity == Polarity::Positive || statement.formula is Atomic)
}
pub open spec fn source_wf_spec(source: SourceSelectionView) -> bool {
    nonempty_spec(source.source) && nonempty_spec(source.revision) && nonempty_spec(source.content)
}
pub open spec fn policy_wf_spec(policy: PolicyView) -> bool {
    policy.authorized_actors.no_duplicates()
        && (forall|i: int| 0 <= i < policy.authorized_actors.len() ==> nonempty_spec(policy.authorized_actors[i]))
        && ttl_wf_spec(policy.ttl)
}
pub open spec fn context_wf_spec(context: ContextView) -> bool {
    context.semantic_version == "2.0.0"@ && natural_spec(context.now)
        && context.ruleset == "core-2"@ && context.checker == "kctx-2"@
        && scope_wf_spec(context.target) && nonempty_spec(context.intended_use)
        && context.sources.map_values(|s: SourceSelectionView| s.source).no_duplicates()
        && (forall|i: int| 0 <= i < context.sources.len() ==> source_wf_spec(context.sources[i]))
        && policy_wf_spec(context.policy)
}
pub open spec fn witness_wf_spec(witness: WitnessView, kb: KBView) -> bool
    decreases witness,
{
    match witness {
        WitnessView::Attest { attestation } => attestation_present(kb, attestation),
        WitnessView::Truth { scope } => scope_wf_spec(scope),
        WitnessView::And { left, right } => witness_wf_spec(*left, kb) && witness_wf_spec(*right, kb),
        WitnessView::Specialize { scope, premise } => scope_wf_spec(scope) && witness_wf_spec(*premise, kb),
        WitnessView::All { enumeration, predicate, premise } => enumeration_present(kb, enumeration)
            && nonempty_spec(predicate) && witness_wf_spec(*premise, kb),
        WitnessView::Absent { enumeration, premise } => enumeration_present(kb, enumeration) && witness_wf_spec(*premise, kb),
    }
}
pub open spec fn dependency_wf_spec(dependency: DependencyView, kb: KBView) -> bool {
    match dependency {
        DependencyView::Observation(value) => kb.observations.contains(value),
        DependencyView::Claim(value) => kb.claims.contains(value),
        DependencyView::Attestation(value) => kb.attestations.contains(value),
        DependencyView::Enumeration(value) => kb.enumerations.contains(value),
        DependencyView::Corrections { target, events } => claim_present(kb, target)
            && forall|i: int| 0 <= i < events.len() ==> kb.corrections.contains(events[i]) && events[i].target == target,
        DependencyView::Context(value) => context_wf_spec(value) && value.now == 0,
    }
}
pub open spec fn observation_wf_spec(value: ObservationView) -> bool {
    nonempty_spec(value.source) && nonempty_spec(value.revision)
        && nonempty_spec(value.content) && natural_spec(value.observed_at)
}
pub open spec fn attestation_wf_spec(value: AttestationView, kb: KBView) -> bool {
    (exists|claim: ClaimView| first_claim_match(kb.claims, value.claim, Some(claim)) && claim.statement == value.statement)
        && nonempty_spec(value.authority) && observation_present(kb, value.observation)
        && match value.statement.formula {
            FormulaView::Atomic(_) => true,
            FormulaView::Inventory { enumeration } => exists|i: int| 0 <= i < kb.enumerations.len()
                && kb.enumerations[i].id == enumeration && value.statement.polarity == Polarity::Positive
                && value.observation == kb.enumerations[i].observation && value.statement.scope == kb.enumerations[i].scope,
            _ => false,
        }
}
pub open spec fn enumeration_wf_spec(value: EnumerationView, kb: KBView) -> bool {
    (exists|observation: ObservationView| first_observation_match(kb.observations, value.observation, Some(observation))
        && observation.source == value.source && observation.revision == value.revision)
        && nonempty_spec(value.query) && nonempty_spec(value.method) && scope_wf_spec(value.scope)
        && value.members == value.universe && value.members.no_duplicates()
        && forall|i: int| 0 <= i < value.members.len() ==> nonempty_spec(value.members[i])
}
pub open spec fn correction_wf_spec(value: CorrectionView, kb: KBView) -> bool {
    claim_present(kb, value.target) && natural_spec(value.recorded_at)
        && match (value.kind, value.replacement) {
            (CorrectionKind::Supersede, Some(reference)) => claim_present(kb, reference),
            (CorrectionKind::Supersede, None) => false,
            (_, Some(_)) => false,
            (_, None) => true,
        }
}
pub open spec fn derivation_wf_spec(value: DerivationView, kb: KBView) -> bool {
    claim_present(kb, value.claim) && witness_wf_spec(value.witness, kb)
}
pub open spec fn receipt_wf_spec(value: ReceiptView, kb: KBView) -> bool {
    claim_present(kb, value.claim) && context_wf_spec(value.context) && witness_wf_spec(value.witness, kb)
        && natural_spec(value.expires_at)
        && forall|i: int| 0 <= i < value.dependencies.len() ==> dependency_wf_spec(value.dependencies[i], kb)
}
pub open spec fn identities_unique(kb: KBView) -> bool {
    kb.observations.map_values(|r: ObservationView| r.id).no_duplicates()
        && kb.claims.map_values(|r: ClaimView| (r.id, r.revision)).no_duplicates()
        && kb.attestations.map_values(|r: AttestationView| r.id).no_duplicates()
        && kb.enumerations.map_values(|r: EnumerationView| r.id).no_duplicates()
        && kb.derivations.map_values(|r: DerivationView| r.id).no_duplicates()
        && kb.corrections.map_values(|r: CorrectionView| r.id).no_duplicates()
        && kb.receipts.map_values(|r: ReceiptView| r.id).no_duplicates()
}
pub open spec fn well_formed_spec(kb: KBView) -> bool {
    identities_unique(kb)
        && (forall|i: int| 0 <= i < kb.observations.len() ==> observation_wf_spec(kb.observations[i]))
        && (forall|i: int| 0 <= i < kb.claims.len() ==> statement_wf_spec(kb.claims[i].statement, kb))
        && (forall|i: int| 0 <= i < kb.attestations.len() ==> attestation_wf_spec(kb.attestations[i], kb))
        && (forall|i: int| 0 <= i < kb.enumerations.len() ==> enumeration_wf_spec(kb.enumerations[i], kb))
        && (forall|i: int| 0 <= i < kb.derivations.len() ==> derivation_wf_spec(kb.derivations[i], kb))
        && (forall|i: int| 0 <= i < kb.corrections.len() ==> correction_wf_spec(kb.corrections[i], kb))
        && super::graph::acyclic(kb)
        && (forall|i: int| 0 <= i < kb.receipts.len() ==> receipt_wf_spec(kb.receipts[i], kb))
}
}
