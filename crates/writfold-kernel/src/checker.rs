//! The preserved finite-witness calculus; acquisition alone proves no claim.
mod context;
pub use assessment::usable_for_snapshot;
use context::Frame;
mod assessment;
#[cfg(verus_keep_ghost)]
mod assessment_model;
mod dependencies;
mod inventory;
pub use dependencies::dependencies_union;
mod evaluation;
#[cfg(verus_keep_ghost)]
mod evaluation_model;
mod footprint;
mod input;
mod lookup;
#[cfg(verus_keep_ghost)]
pub mod spec;

pub use assessment::{
    assess, assessment_gaps, policy_allows, reusable, usable_for, witness_freshness,
};
#[cfg(verus_keep_ghost)]
pub use context::same_context_spec;
use evaluation::{accepted_derivation, evaluate, has_accepted_claim, has_accepted_identity};
#[cfg(verus_keep_ghost)]
use evaluation_model::{accepted_rows, supported_frame_spec};
#[cfg(verus_keep_ghost)]
use footprint::{EvaluationView, checked_value};
use footprint::{corrections, push_unique};
use input::Input;
#[cfg(verus_keep_ghost)]
use lookup::{claim_lookup_value, claim_value};
#[cfg(verus_keep_ghost)]
pub(crate) use lookup::{first_matching, first_matching_absent, first_matching_at};
#[cfg(verus_keep_ghost)]
use spec::{
    accepted_spec, check_matches_snapshot_spec, check_matches_spec, check_spec,
    correction_dependency_spec,
};
use vstd::prelude::*;
use writfold_domain::*;

verus! {
pub fn context_matches(kb: &KB, context: &Context) -> (matches: bool)
    ensures matches == (kb@.snapshot == context@.snapshot
        && crate::wellformed::spec::context_wf_spec(context@)),
{
    Frame::new(context).matches(kb)
}

pub open spec fn source_binding_spec(source: SourceSelectionView, observation: ObservationView) -> bool {
    source.source == observation.source && source.revision == observation.revision
        && source.content == observation.content
}
pub open spec fn source_selected_spec(context: ContextView, observation: ObservationView) -> bool {
    exists|i: int| 0 <= i < context.sources.len() && source_binding_spec(context.sources[i], observation)
}
pub fn source_selected(context: &Context, observation: &Observation) -> (selected: bool)
    ensures selected == source_selected_spec(context@, observation@),
{
    let sources = context.sources();
    let mut index = 0;
    proof { assert(sources.deep_view().len() == sources.len()); }
    while index < sources.len()
        invariant
            sources.deep_view() == context@.sources, sources.len() == context@.sources.len(),
            index <= sources.len(),
            forall|i: int| 0 <= i < index ==> !source_binding_spec(context@.sources[i], observation@),
        decreases sources.len() - index,
    {
        let source = &sources[index];
        proof { assert(source@ == context@.sources[index as int]); }
        if source.source() == observation.source() && source.revision() == observation.revision()
            && source.content() == observation.content() {
            return true;
        }
        index += 1;
    }
    false
}

pub fn inventory_content(members: &[String]) -> (identity: ContentId)
    ensures identity@ == ContentId::inventory_spec(members.deep_view()), identity.valid(),
{
    ContentId::inventory(members)
}

pub open spec fn retracts_at(correction: CorrectionView, reference: ClaimRefView, now: int) -> bool {
    correction.kind == CorrectionKind::Retract && correction.target == reference && correction.recorded_at <= now
}
pub open spec fn retracted_spec(kb: KBView, context: ContextView, reference: ClaimRefView) -> bool {
    exists|i: int| 0 <= i < kb.corrections.len() && retracts_at(kb.corrections[i], reference, context.now)
}
pub fn retracted(kb: &KB, context: &Context, reference: &ClaimRef) -> (retracted: bool)
    ensures retracted == retracted_spec(kb@, context@, reference@),
{
    let corrections = kb.corrections();
    let mut index = 0;
    proof { assert(corrections.deep_view().len() == corrections.len()); }
    while index < corrections.len()
        invariant
            corrections.deep_view() == kb@.corrections, corrections.len() == kb@.corrections.len(),
            index <= corrections.len(),
            forall|i: int| 0 <= i < index ==> !retracts_at(kb@.corrections[i], reference@, context@.now),
        decreases corrections.len() - index,
    {
        let correction = &corrections[index];
        proof { assert(correction@ == kb@.corrections[index as int]); }
        if *correction.kind() == CorrectionKind::Retract && correction.target() == reference
            && correction.recorded_at() <= context.now() {
            return true;
        }
        index += 1;
    }
    false
}
}

verus! {
pub fn correction_dependency(kb: &KB, context: &Context, reference: &ClaimRef) -> (dependency: Dependency)
    ensures dependency@ == correction_dependency_spec(kb@, context@, reference@),
{
    corrections(kb, Frame::new(context), reference).into_owned()
}

pub fn check(kb: &KB, context: &Context, witness: &Witness) -> (checked: Option<Checked>)
    ensures checked.deep_view() == check_spec(kb@, context@, witness@),
{
    let mut result = evaluate(kb, Frame::new(context), witness)?;
    push_unique(&mut result.dependencies, Input::Context(Frame::new(context)));
    Some(result.into_owned())
}

pub fn accepted<'a>(
    kb: &'a KB,
    context: &Context,
    reference: &ClaimRef,
) -> (accepted: Vec<(&'a Witness, Checked)>)
    ensures accepted.deep_view() == accepted_spec(kb@, context@, reference@),
{
    proof { broadcast use claim_lookup_value; }
    let Some(claim) = find_claim(kb, reference) else {
        return Vec::new();
    };
    if retracted(kb, context, reference) {
        return Vec::new();
    }
    let frame = Frame::new(context);
    let rows = kb.derivations();
    let mut accepted = Vec::new();
    let mut index = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while index < rows.len()
        invariant
            frame@ == spec::frame(context@),
            rows.deep_view() == kb@.derivations, rows.len() == kb@.derivations.len(),
            index <= rows.len(), claim_value(kb@, reference@) == Some(claim@),
            !retracted_spec(kb@, context@, reference@),
            accepted.deep_view() == accepted_rows(kb@, frame@, claim@, kb@.derivations.take(index as int))
                .map_values(|item: (WitnessView, EvaluationView)| (item.0, checked_value(item.1))),
        decreases rows.len() - index,
    {
        proof {
            evaluation_model::accepted_rows_step(kb@, frame@, claim@, kb@.derivations, index as int);
        }
        let row = &rows[index];
        proof { assert(row@ == kb@.derivations[index as int]); }
        let ghost prior = accepted_rows(kb@, frame@, claim@, kb@.derivations.take(index as int));
        if let Some(result) = accepted_derivation(kb, frame, claim, row) {
            let ghost evaluated = result@;
            let checked = result.into_owned();
            let item = (row.witness(), checked);
            proof { assert(item.deep_view() == (row@.witness, checked_value(evaluated))); }
            accepted.push(item);
            proof {
                footprint::map_push(prior, (row@.witness, evaluated),
                    |value: (WitnessView, EvaluationView)| (value.0, checked_value(value.1)));
                assert(accepted.deep_view() =~= prior
                    .map_values(|value: (WitnessView, EvaluationView)| (value.0, checked_value(value.1)))
                    .push((row@.witness, checked_value(evaluated))));
            }
        }
        index += 1;
    }
    proof { assert(kb@.derivations.take(rows.len() as int) == kb@.derivations); }
    accepted
}

pub fn supported(kb: &KB, context: &Context, reference: &ClaimRef, polarity: Polarity) -> (supported: bool)
    ensures supported == supported_frame_spec(kb@, spec::frame(context@), reference@, polarity),
{
    supported_frame(kb, Frame::new(context), reference, polarity)
}

fn supported_frame(kb: &KB, context: Frame<'_>, reference: &ClaimRef, polarity: Polarity) -> (supported: bool)
    ensures supported == supported_frame_spec(kb@, context@, reference@, polarity),
{
    proof {
        broadcast use claim_lookup_value;
        reveal_with_fuel(<Formula as View>::view, 2);
    }
    let Some(claim) = find_claim(kb, reference) else {
        return false;
    };
    if polarity == *claim.statement().polarity() {
        return has_accepted_claim(kb, context, claim);
    }
    if !matches!(claim.statement().formula(), Formula::Atomic(_)) {
        return false;
    }
    let rows = kb.claims();
    let mut index = 0;
    proof { assert(rows.deep_view().len() == rows.len()); }
    while index < rows.len()
        invariant
            claim_value(kb@, reference@) == Some(claim@), claim@.statement.formula is Atomic,
            polarity != claim@.statement.polarity, index <= rows.len(),
            rows.deep_view() == kb@.claims, rows.len() == kb@.claims.len(),
            forall|i: int| 0 <= i < index ==> !(kb@.claims[i].statement.formula == claim@.statement.formula
                && kb@.claims[i].statement.scope == claim@.statement.scope && kb@.claims[i].statement.polarity == polarity
                && evaluation_model::has_accepted_reference_spec(kb@, context@,
                    ClaimRefView { id: kb@.claims[i].id, rev: kb@.claims[i].revision })),
        decreases rows.len() - index,
    {
        let row = &rows[index];
        proof { assert(row@ == kb@.claims[index as int]); }
        if row.statement().formula() == claim.statement().formula()
            && row.statement().scope() == claim.statement().scope() && *row.statement().polarity() == polarity
            && has_accepted_identity(kb, context, row.id(), row.revision()) {
            return true;
        }
        index += 1;
    }
    false
}

pub fn same_context(left: &Context, right: &Context) -> (same: bool)
    ensures same == same_context_spec(left@, right@),
{
    Frame::new(left).same_context(right)
}

pub fn check_matches(kb: &KB, context: &Context, witness: &Witness, statement: &Statement) -> (matches: bool)
    ensures matches == check_matches_spec(kb@, context@, witness@, statement@),
{
    match evaluate(kb, Frame::new(context), witness) {
        Some(value) => value.matches_statement(statement),
        None => false,
    }
}

pub fn check_matches_snapshot(
    kb: &KB,
    context: &Context,
    snapshot: &SnapshotId,
    witness: &Witness,
    statement: &Statement,
) -> (matches: bool)
    ensures matches == check_matches_snapshot_spec(kb@, context@, snapshot@, witness@, statement@),
{
    match evaluate(kb, Frame { base: context, snapshot }, witness) {
        Some(value) => value.matches_statement(statement),
        None => false,
    }
}
} // verus!
