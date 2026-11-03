//! Ghost bridges for allocation-free validation of the actual resulting records.
#![cfg(verus_keep_ghost)]
use super::model::*;
use crate::wellformed::spec::*;
use crate::workflow::admission_model::{preserves_prefix_spec, selected_present};
use vstd::prelude::*;
use writfold_domain::*;

use crate::wellformed::graph::*;
use crate::workflow::admission_model::kb_at_spec;

verus! {
pub open spec fn appended_disjoint(state: PortfolioStateView, plan: PortfolioPlanView) -> bool {
    forall|i: int, j: int| state.kb.claims.len() <= i < plan.candidate.claims.len()
        && 0 <= j < state.selected.len() ==> !claim_matches_spec(plan.candidate.claims[i], state.selected[j])
}
pub proof fn admission_projection(state: PortfolioStateView, plan: PortfolioPlanView)
    requires portfolio_state_valid_spec(state), preserves_prefix_spec(state.kb, plan.candidate), plan.contexts.len() > 0,
    ensures portfolio_state_valid_spec(admitted_portfolio_spec(state, plan))
        == (well_formed_spec(plan.candidate) && context_family_valid_spec(plan.candidate, plan.contexts)
            && appended_disjoint(state, plan)),
{
    let added = added_refs_spec(state, plan);
    let selected = state.selected + added;
    let n = state.selected.len();
    let start = state.kb.claims.len();
    assert forall|i: int| 0 <= i < added.len() implies
        claim_matches_spec(plan.candidate.claims[start + i], added[i]) by {}
    assert forall|i: int| 0 <= i < selected.len() implies
        #[trigger] claim_present(plan.candidate, selected[i]) by {
        if i < n {
            assert(claim_present(state.kb, state.selected[i]));
            let j = choose|j: int| 0 <= j < state.kb.claims.len()
                && claim_matches_spec(state.kb.claims[j], state.selected[i]);
            assert(plan.candidate.claims[j] == state.kb.claims[j]);
        } else {
            assert(claim_matches_spec(plan.candidate.claims[start + i - n], selected[i]));
        }
    }
    if well_formed_spec(plan.candidate) && appended_disjoint(state, plan) {
        assert forall|i: int, j: int| 0 <= i < selected.len() && 0 <= j < selected.len() && i != j
            implies selected[i] != selected[j] by {
            if i < n && j < n {
                assert(state.selected[i] != state.selected[j]);
            } else if i < n {
                assert(!claim_matches_spec(plan.candidate.claims[start + j - n], state.selected[i]));
            } else if j < n {
                assert(!claim_matches_spec(plan.candidate.claims[start + i - n], state.selected[j]));
            } else {
                let keys = plan.candidate.claims.map_values(|c: ClaimView| (c.id, c.revision));
                assert(keys[start + i - n] != keys[start + j - n]);
            }
        }
    }
    if selected.no_duplicates() {
        assert forall|i: int, j: int| start <= i < plan.candidate.claims.len() && 0 <= j < n
            implies !claim_matches_spec(plan.candidate.claims[i], state.selected[j]) by {
            assert(selected[n + i - start] != selected[j]);
        }
    }
}
proof fn snapshot_lookups(kb: KBView, snapshot: Seq<char>)
    ensures
        forall|id: Seq<char>| #[trigger] enumeration_present(kb_at_spec(kb, snapshot), id) == enumeration_present(kb, id),
        forall|id: Seq<char>| #[trigger] observation_present(kb_at_spec(kb, snapshot), id) == observation_present(kb, id),
        forall|id: Seq<char>| #[trigger] attestation_present(kb_at_spec(kb, snapshot), id) == attestation_present(kb, id),
        forall|reference: ClaimRefView| #[trigger] claim_present(kb_at_spec(kb, snapshot), reference) == claim_present(kb, reference),
{
    let next = kb_at_spec(kb, snapshot);
    assert(next.enumerations == kb.enumerations);
    assert(next.observations == kb.observations);
    assert(next.attestations == kb.attestations);
    assert(next.claims == kb.claims);
}
proof fn formula_snapshot(formula: FormulaView, kb: KBView, snapshot: Seq<char>)
    ensures formula_wf_spec(formula, kb) == formula_wf_spec(formula, kb_at_spec(kb, snapshot)),
    decreases formula,
{
    reveal_with_fuel(formula_wf_spec, 2);
    snapshot_lookups(kb, snapshot);
    match formula {
        FormulaView::Conjunction { left, right } => {
            formula_snapshot(*left, kb, snapshot);
            formula_snapshot(*right, kb, snapshot);
        },
        FormulaView::All { enumeration, .. } | FormulaView::Absent { enumeration }
            | FormulaView::Inventory { enumeration } => {
            assert(enumeration_present(kb, enumeration) == enumeration_present(kb_at_spec(kb, snapshot), enumeration));
        },
        _ => {},
    }
}
proof fn witness_snapshot(witness: WitnessView, kb: KBView, snapshot: Seq<char>)
    ensures witness_wf_spec(witness, kb) == witness_wf_spec(witness, kb_at_spec(kb, snapshot)),
    decreases witness,
{
    match witness {
        WitnessView::And { left, right } => {
            witness_snapshot(*left, kb, snapshot);
            witness_snapshot(*right, kb, snapshot);
        },
        WitnessView::Specialize { premise, .. } | WitnessView::All { premise, .. }
            | WitnessView::Absent { premise, .. } => witness_snapshot(*premise, kb, snapshot),
        _ => {},
    }
}
proof fn attestation_snapshot(row: AttestationView, kb: KBView, snapshot: Seq<char>)
    ensures attestation_wf_spec(row, kb) == attestation_wf_spec(row, kb_at_spec(kb, snapshot)),
{
    let next = kb_at_spec(kb, snapshot);
    snapshot_lookups(kb, snapshot);
    assert(observation_present(next, row.observation) == observation_present(kb, row.observation));
    assert(next.claims == kb.claims);
    assert(next.enumerations == kb.enumerations);
    match row.statement.formula {
        FormulaView::Inventory { enumeration } => {
            assert forall|i: int| 0 <= i < kb.enumerations.len() implies
                (kb.enumerations[i].id == enumeration && row.statement.polarity == Polarity::Positive
                    && row.observation == kb.enumerations[i].observation && row.statement.scope == kb.enumerations[i].scope)
                == (next.enumerations[i].id == enumeration && row.statement.polarity == Polarity::Positive
                    && row.observation == next.enumerations[i].observation && row.statement.scope == next.enumerations[i].scope) by {}
        },
        _ => {},
    }
}
proof fn enumeration_snapshot(row: EnumerationView, kb: KBView, snapshot: Seq<char>)
    ensures enumeration_wf_spec(row, kb) == enumeration_wf_spec(row, kb_at_spec(kb, snapshot)),
{
    let next = kb_at_spec(kb, snapshot);
    assert(next.observations == kb.observations);
    assert forall|observation: ObservationView|
        #[trigger] first_observation_match(kb.observations, row.observation, Some(observation))
        == first_observation_match(next.observations, row.observation, Some(observation)) by {}
    reveal(enumeration_wf_spec);
}
pub proof fn wellformed_snapshot(kb: KBView, snapshot: Seq<char>)
    ensures well_formed_spec(kb) == well_formed_spec(kb_at_spec(kb, snapshot)),
{
    let next = kb_at_spec(kb, snapshot);
    snapshot_lookups(kb, snapshot);
    assert forall|formula: FormulaView| formula_wf_spec(formula, kb)
        == formula_wf_spec(formula, next) by { formula_snapshot(formula, kb, snapshot); }
    assert forall|witness: WitnessView| witness_wf_spec(witness, kb)
        == witness_wf_spec(witness, next) by { witness_snapshot(witness, kb, snapshot); }
    assert forall|from: Vertex, to: Vertex| edge(kb, from, to) == edge(next, from, to) by {}
    assert forall|set: Set<Vertex>| closed(kb, set) == closed(next, set) by {}
    assert forall|from: Vertex, to: Vertex| reachable(kb, from, to) == reachable(next, from, to) by {}
    assert forall|from: Vertex| no_cycle_from(kb, from) == no_cycle_from(next, from) by {}
    assert(acyclic(kb) == acyclic(next));
    assert forall|i: int| 0 <= i < kb.claims.len() implies
        statement_wf_spec(kb.claims[i].statement, kb) == statement_wf_spec(kb.claims[i].statement, next) by {}
    assert forall|i: int| 0 <= i < kb.attestations.len() implies
        attestation_wf_spec(kb.attestations[i], kb) == attestation_wf_spec(kb.attestations[i], next) by {
        attestation_snapshot(kb.attestations[i], kb, snapshot);
    }
    assert forall|i: int| 0 <= i < kb.enumerations.len() implies
        enumeration_wf_spec(kb.enumerations[i], kb) == enumeration_wf_spec(kb.enumerations[i], next) by {
        enumeration_snapshot(kb.enumerations[i], kb, snapshot);
    }
    assert forall|i: int| 0 <= i < kb.derivations.len() implies
        derivation_wf_spec(kb.derivations[i], kb) == derivation_wf_spec(kb.derivations[i], next) by {}
    assert forall|i: int| 0 <= i < kb.corrections.len() implies
        correction_wf_spec(kb.corrections[i], kb) == correction_wf_spec(kb.corrections[i], next) by {}
    assert forall|dependency: DependencyView| dependency_wf_spec(dependency, kb)
        == dependency_wf_spec(dependency, next) by {}
    assert forall|i: int| 0 <= i < kb.receipts.len() implies
        receipt_wf_spec(kb.receipts[i], kb) == receipt_wf_spec(kb.receipts[i], next) by {}
}
pub proof fn selection_projection(state: PortfolioStateView, plan: PortfolioSelectionPlanView)
    requires portfolio_state_valid_spec(state), plan.contexts.len() > 0,
        selection_contexts_valid_spec(state.contexts, plan.contexts, plan.new_snapshot),
    ensures portfolio_state_valid_spec(selected_portfolio_spec(state, plan))
        == (plan.selected.no_duplicates() && selected_present(state.kb, plan.selected)
            && forall|i: int| 0 <= i < plan.contexts.len() ==> context_wf_spec(plan.contexts[i].context)),
{
    wellformed_snapshot(state.kb, plan.new_snapshot);
    snapshot_lookups(state.kb, plan.new_snapshot);
    let next = selected_portfolio_spec(state, plan);
    assert forall|i: int| 0 <= i < plan.contexts.len()
        implies plan.contexts[i].context.snapshot == plan.new_snapshot by {
        assert(selection_context_valid(state.contexts, plan.contexts[i], plan.new_snapshot));
    }
    assert forall|i: int, j: int| 0 <= j < i < plan.contexts.len()
        implies plan.contexts[j].id != plan.contexts[i].id by {
        assert(state.contexts[j].id != state.contexts[i].id);
    }
    assert forall|i: int| 0 <= i < plan.selected.len()
        implies claim_present(next.kb, plan.selected[i]) == claim_present(state.kb, plan.selected[i]) by {}
    assert(selected_present(next.kb, plan.selected) == selected_present(state.kb, plan.selected));
    assert(context_family_valid_spec(next.kb, plan.contexts)
        == (forall|i: int| 0 <= i < plan.contexts.len() ==> context_wf_spec(plan.contexts[i].context)));
}
} // verus!
