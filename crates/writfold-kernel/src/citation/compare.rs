use super::{InvalidReason, ResolveResult, diagnose, lookup_representation, resolve};
use vstd::prelude::*;
use writfold_domain::citation::*;

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorrespondenceWitness { IdenticalPosition }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnknownReason {
    OriginalUnavailable, OriginalInvalid(InvalidReason), TargetUnavailable,
    UnsupportedCorrespondence, IncompleteSearch, NoTextMatch, ScopeMismatch,
}
#[derive(Debug, Eq, PartialEq)]
pub enum CompareResult<'a> {
    SameBinding,
    UniqueJustified { successor: Binding<'a>, witness: CorrespondenceWitness },
    Multiple { candidates: Vec<Selector>, coverage: Coverage },
    Unknown { reason: UnknownReason, candidates: Vec<Selector>, coverage: Coverage },
    Denied,
}

pub fn same_occurrence(original: Binding<'_>, target: Binding<'_>) -> (same: bool)
    ensures same == (original@.representation == target@.representation
        && original@.selector == target@.selector && original@.expected == target@.expected
        && original@.context == target@.context && original@.role == target@.role),
{
    original.representation() == target.representation()
        && original.selector().same(target.selector())
        && bytes_equal(original.expected(), target.expected())
        && original.context() == target.context() && original.role() == target.role()
}

fn unavailable<'a>(reason: UnknownReason) -> (result: CompareResult<'a>)
    ensures match result {
        CompareResult::Unknown { reason: actual, candidates, coverage } =>
            actual == reason && candidates.len() == 0 && coverage == Coverage::Unavailable,
        _ => false,
    },
{
    CompareResult::Unknown { reason, candidates: Vec::new(), coverage: Coverage::Unavailable }
}

fn text_diagnosis<'a>(bytes: &[u8], needle: &[u8], budget: SearchBudget)
    -> (result: CompareResult<'a>)
    ensures result is Multiple || result is Unknown,
{
    let (candidates, coverage) = diagnose(bytes, needle, budget).into_parts();
    if candidates.len() >= 2 { return CompareResult::Multiple { candidates, coverage }; }
    let reason = if !coverage.complete() { UnknownReason::IncompleteSearch }
        else if candidates.is_empty() { UnknownReason::NoTextMatch }
        else { UnknownReason::UnsupportedCorrespondence };
    CompareResult::Unknown { reason, candidates, coverage }
}

/// The only supported lineage rule is identical immutable occurrence, context and
/// evidence role. Release and citation ID may differ. Text-only diagnostics never
/// produce UniqueJustified, SameBinding, or a claim of deletion.
#[allow(clippy::too_many_arguments, reason = "two exact bindings and current policy decisions")]
pub fn compare<'a>(store: &[Representation<'_>], original: Binding<'_>, target: Binding<'a>,
    original_access: AccessDecision<'_>, target_access: AccessDecision<'_>,
    witness: Option<CorrespondenceWitness>, budget: SearchBudget) -> (result: CompareResult<'a>)
    ensures
        (!original_access.permitted_value() || original_access.binding_value() != original@
            || !target_access.permitted_value() || target_access.binding_value() != target@)
            ==> result is Denied,
        match result {
            CompareResult::UniqueJustified { successor, witness: proof_kind } =>
                successor@ == target@ && proof_kind == CorrespondenceWitness::IdenticalPosition
                && witness == Some(CorrespondenceWitness::IdenticalPosition)
                && original@.representation == target@.representation
                && original@.selector == target@.selector && original@.expected == target@.expected
                && original@.context == target@.context && original@.role == target@.role
                && original_access.permitted_value() && original_access.binding_value() == original@
                && target_access.permitted_value() && target_access.binding_value() == target@,
            CompareResult::SameBinding => original@ == target@,
            _ => true,
        },
{
    if !original_access.allows(original) || !target_access.allows(target) {
        return CompareResult::Denied;
    }
    match resolve(store, original, original_access) {
        ResolveResult::Denied => return CompareResult::Denied,
        ResolveResult::Invalid(reason) => return unavailable(UnknownReason::OriginalInvalid(reason)),
        ResolveResult::Unavailable => return unavailable(UnknownReason::OriginalUnavailable),
        ResolveResult::Exact { .. } => {},
    }
    if original.representation().realm() != target.representation().realm()
        || original.representation().repository() != target.representation().repository()
        || original.context() != target.context() {
        return unavailable(UnknownReason::ScopeMismatch);
    }
    if *target.representation().profile() != RepresentationProfile::GitBlob
        || target.selector().unit() != CoordinateUnit::Byte {
        return unavailable(UnknownReason::UnsupportedCorrespondence);
    }
    let Some(object) = lookup_representation(store, target.representation()) else {
        return unavailable(UnknownReason::TargetUnavailable);
    };
    if original.same(target) { return CompareResult::SameBinding; }
    if matches!(witness, Some(CorrespondenceWitness::IdenticalPosition))
        && same_occurrence(original, target)
        && matches!(resolve(store, target, target_access), ResolveResult::Exact { .. }) {
        return CompareResult::UniqueJustified {
            successor: target, witness: CorrespondenceWitness::IdenticalPosition,
        };
    }
    text_diagnosis(object.bytes(), original.expected(), budget)
}
}
