//! Borrowed footprint values preserve their original equality and owned projection.
use super::context::Frame;
#[cfg(verus_keep_ghost)]
use super::context::{FrameView, normalized_context};
use vstd::prelude::*;
use writfold_domain::{Integer, *};

verus! {

pub enum Input<'a> {
    Observation(&'a Observation),
    Claim(&'a Claim),
    Attestation(&'a Attestation),
    Enumeration(&'a Enumeration),
    Corrections { target: &'a ClaimRef, events: Vec<&'a Correction> },
    Context(Frame<'a>),
}

#[cfg(verus_keep_ghost)]
pub ghost enum InputView {
    Observation(ObservationView),
    Claim(ClaimView),
    Attestation(AttestationView),
    Enumeration(EnumerationView),
    Corrections { target: ClaimRefView, events: Seq<CorrectionView> },
    Context(FrameView),
}
#[cfg(verus_keep_ghost)]
impl View for Input<'_> {
    type V = InputView;
    open spec fn view(&self) -> InputView {
        match self {
            Self::Observation(value) => InputView::Observation(value@),
            Self::Claim(value) => InputView::Claim(value@),
            Self::Attestation(value) => InputView::Attestation(value@),
            Self::Enumeration(value) => InputView::Enumeration(value@),
            Self::Corrections { target, events } => InputView::Corrections {
                target: target@, events: events.deep_view(),
            },
            Self::Context(frame) => InputView::Context(frame@),
        }
    }
}
#[cfg(verus_keep_ghost)]
impl DeepView for Input<'_> {
    type V = InputView;
    open spec fn deep_view(&self) -> InputView { self@ }
}
#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Input<'_> {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { self@ == other@ }
}
impl PartialEq for Input<'_> {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (self@ == other@),
    {
        proof { broadcast use writfold_domain::equality::vector_equality; }
        match (self, other) {
            (Self::Observation(left), Self::Observation(right)) => *left == *right,
            (Self::Claim(left), Self::Claim(right)) => *left == *right,
            (Self::Attestation(left), Self::Attestation(right)) => *left == *right,
            (Self::Enumeration(left), Self::Enumeration(right)) => *left == *right,
            (Self::Corrections { target, events }, Self::Corrections { target: other_target, events: other_events }) =>
                *target == *other_target && events == other_events,
            (Self::Context(left), Self::Context(right)) => left == right,
            _ => false,
        }
    }
}

pub open spec fn dependency_value(input: InputView) -> DependencyView {
    match input {
        InputView::Observation(value) => DependencyView::Observation(value),
        InputView::Claim(value) => DependencyView::Claim(value),
        InputView::Attestation(value) => DependencyView::Attestation(value),
        InputView::Enumeration(value) => DependencyView::Enumeration(value),
        InputView::Corrections { target, events } => DependencyView::Corrections { target, events },
        InputView::Context(frame) => DependencyView::Context(normalized_context(frame)),
    }
}

fn same_events(left: &[&Correction], right: &[Correction]) -> (equal: bool)
    ensures equal == (left.deep_view() == right.deep_view()),
{
    proof {
        assert(left.deep_view().len() == left.len());
        assert(right.deep_view().len() == right.len());
    }
    if left.len() != right.len() { return false; }
    let mut index = 0;
    while index < left.len()
        invariant left.len() == right.len(), index <= left.len(),
            left.deep_view().len() == left.len(), right.deep_view().len() == right.len(),
            forall|i: int| 0 <= i < index ==> left.deep_view()[i] == right.deep_view()[i],
        decreases left.len() - index,
    {
        if !left[index].eq(&right[index]) {
            proof { assert(left.deep_view()[index as int] != right.deep_view()[index as int]); }
            return false;
        }
        index += 1;
    }
    proof { assert(left.deep_view() =~= right.deep_view()); }
    true
}

impl Input<'_> {
    pub(super) fn into_owned(self) -> (dependency: Dependency)
        ensures dependency@ == dependency_value(self@),
    {
        match self {
            Self::Observation(value) => Dependency::Observation(value.clone()),
            Self::Claim(value) => Dependency::Claim(value.clone()),
            Self::Attestation(value) => Dependency::Attestation(value.clone()),
            Self::Enumeration(value) => Dependency::Enumeration(value.clone()),
            Self::Corrections { target, events } => {
                let mut copied: Vec<Correction> = Vec::with_capacity(events.len());
                let mut index = 0;
                proof { assert(events.deep_view().len() == events.len()); }
                while index < events.len()
                    invariant index <= events.len(), events.deep_view().len() == events.len(),
                        copied.deep_view() == events.deep_view().take(index as int),
                    decreases events.len() - index,
                {
                    let ghost prefix = events.deep_view().take(index as int);
                    let event = events[index].clone();
                    proof { assert(event@ == events.deep_view()[index as int]); }
                    copied.push(event);
                    proof {
                        assert(copied.deep_view() =~= prefix.push(event@));
                        assert(events.deep_view().take(index as int + 1) =~= prefix.push(event@));
                    }
                    index += 1;
                }
                proof { assert(events.deep_view().take(events.len() as int) == events.deep_view()); }
                Dependency::Corrections { target: target.clone(), events: copied }
            },
            Self::Context(frame) => Dependency::Context(frame.normalized()),
        }
    }

    pub(super) fn matches(&self, dependency: &Dependency) -> (matches: bool)
        ensures matches == (dependency_value(self@) == dependency@),
    {
        match (self, dependency) {
            (Self::Observation(left), Dependency::Observation(right)) => *left == right,
            (Self::Claim(left), Dependency::Claim(right)) => *left == right,
            (Self::Attestation(left), Dependency::Attestation(right)) => *left == right,
            (Self::Enumeration(left), Dependency::Enumeration(right)) => *left == right,
            (Self::Corrections { target, events }, Dependency::Corrections { target: other, events: values }) =>
                *target == other && same_events(events, values),
            (Self::Context(left), Dependency::Context(right)) =>
                right.now() == &Integer::from(0) && left.same_context(right),
            _ => false,
        }
    }
}

} // verus!
