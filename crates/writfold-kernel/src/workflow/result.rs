//! Rejected and replayed operations retain the exact borrowed inputs.
use std::borrow::Cow;
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::cmp::PartialEqSpec;
use writfold_domain::*;

verus! {
#[derive(Debug)]
#[verifier::reject_recursive_types(S)]
#[verifier::reject_recursive_types(J)]
pub struct WorkflowResult<'a, S: Clone, J: Clone> {
    state: Cow<'a, S>,
    journal: Cow<'a, [J]>,
    outcome: CommitOutcome,
}
impl<S: Clone, J: Clone> Clone for WorkflowResult<'_, S, J> {
    fn clone(&self) -> Self {
        let state = match &self.state {
            Cow::Borrowed(value) => Cow::Borrowed(*value),
            Cow::Owned(value) => Cow::Owned(value.clone()),
        };
        let journal = match &self.journal {
            Cow::Borrowed(value) => Cow::Borrowed(*value),
            Cow::Owned(value) => Cow::Owned(value.clone()),
        };
        Self { state, journal, outcome: self.outcome }
    }
}
#[cfg(verus_keep_ghost)]
impl<S: Clone + PartialEq, J: Clone + PartialEq> vstd::std_specs::cmp::PartialEqSpecImpl for WorkflowResult<'_, S, J> {
    open spec fn obeys_eq_spec() -> bool {
        S::obeys_eq_spec() && J::obeys_eq_spec()
    }
    open spec fn eq_spec(&self, other: &Self) -> bool {
        let left = self.storage();
        let right = other.storage();
        left.state.eq_spec(&right.state) && left.outcome == right.outcome
            && left.journal.len() == right.journal.len()
            && forall|i: int| 0 <= i < left.journal.len()
                ==> left.journal[i].eq_spec(&right.journal[i])
    }
}
impl<S: Clone + PartialEq, J: Clone + PartialEq> PartialEq for WorkflowResult<'_, S, J> {
    fn eq(&self, other: &Self) -> bool {
        self.state() == other.state() && self.journal() == other.journal()
            && self.outcome == other.outcome
    }
}
impl<S: Clone + Eq, J: Clone + Eq> Eq for WorkflowResult<'_, S, J> {}
#[cfg(verus_keep_ghost)]
pub ghost struct WorkflowResultView<S, J> {
    pub state: S,
    pub journal: Seq<J>,
    pub outcome: CommitOutcome,
    pub state_borrowed: bool,
    pub journal_borrowed: bool,
}
#[cfg(verus_keep_ghost)]
impl<S: Clone + DeepView, J: Clone + DeepView> View for WorkflowResult<'_, S, J> {
    type V = WorkflowResultView<S::V, J::V>;
    open spec fn view(&self) -> Self::V {
        let parts = self.parts_spec();
        WorkflowResultView {
            state: match parts.0 { Cow::Borrowed(value) => value.deep_view(), Cow::Owned(value) => value.deep_view() },
            journal: match parts.1 { Cow::Borrowed(value) => value.deep_view(), Cow::Owned(value) => value.deep_view() },
            outcome: parts.2, state_borrowed: matches!(parts.0, Cow::Borrowed(_)),
            journal_borrowed: matches!(parts.1, Cow::Borrowed(_)) }
    }
}
#[cfg(verus_keep_ghost)]
impl<S: Clone + DeepView, J: Clone + DeepView> DeepView for WorkflowResult<'_, S, J> {
    type V = WorkflowResultView<S::V, J::V>;
    open spec fn deep_view(&self) -> Self::V { self@ }
}
impl<'a, S: Clone, J: Clone> WorkflowResult<'a, S, J> {
    pub closed spec fn parts_spec(&self) -> (Cow<'a, S>, Cow<'a, [J]>, CommitOutcome) {
        (self.state, self.journal, self.outcome)
    }
    pub open spec fn storage(&self) -> WorkflowResultView<S, J> {
        let parts = self.parts_spec();
        WorkflowResultView {
            state: match parts.0 { Cow::Borrowed(value) => *value, Cow::Owned(value) => value },
            journal: match parts.1 { Cow::Borrowed(value) => value@, Cow::Owned(value) => value@ },
            outcome: parts.2, state_borrowed: matches!(parts.0, Cow::Borrowed(_)),
            journal_borrowed: matches!(parts.1, Cow::Borrowed(_)),
        }
    }
    pub(crate) fn unchanged(state: &'a S, journal: &'a [J], outcome: CommitOutcome) -> (result: Self)
        ensures result.parts_spec() == (Cow::Borrowed(state), Cow::Borrowed(journal), outcome),
    {
        Self { state: Cow::Borrowed(state), journal: Cow::Borrowed(journal), outcome }
    }
    pub(crate) fn applied(state: S, journal: Vec<J>) -> (result: Self)
        ensures result.parts_spec() == (Cow::Owned(state), Cow::Owned(journal), CommitOutcome::Applied),
    {
        Self { state: Cow::Owned(state), journal: Cow::Owned(journal), outcome: CommitOutcome::Applied }
    }
    pub fn state(&self) -> (state: &S)
        ensures *state == self.storage().state,
    {
        match &self.state { Cow::Borrowed(value) => value, Cow::Owned(value) => value }
    }
    pub fn journal(&self) -> (journal: &[J])
        ensures journal@ == self.storage().journal,
    {
        match &self.journal { Cow::Borrowed(value) => value, Cow::Owned(value) => value.as_slice() }
    }
    pub fn outcome(&self) -> (outcome: &CommitOutcome)
        ensures *outcome == self.storage().outcome,
    { &self.outcome }
    pub fn into_parts(self) -> (parts: (Cow<'a, S>, Cow<'a, [J]>, CommitOutcome))
        ensures parts == self.parts_spec(),
    { (self.state, self.journal, self.outcome) }
}
impl WorkflowResult<'_, KB, JournalEntry> {
    pub fn kb(&self) -> (kb: &KB)
        ensures kb@ == self@.state,
    { self.state() }
}
pub type Application<'a> = WorkflowResult<'a, KB, JournalEntry>;
pub type SelectionResult<'a> = WorkflowResult<'a, SelectionState, SelectionJournalEntry>;
pub type PortfolioResult<'a> = WorkflowResult<'a, PortfolioState, PortfolioJournalEntry>;
pub type PortfolioSelectionResult<'a> = WorkflowResult<'a, PortfolioState, PortfolioSelectionJournalEntry>;

pub(crate) fn replay_outcome<P: PartialEq>(recorded: &P, proposed: &P) -> (outcome: CommitOutcome)
    requires P::obeys_eq_spec(),
    ensures outcome == (if recorded.eq_spec(proposed) {
        CommitOutcome::AlreadyApplied
    } else { CommitOutcome::InputConflict }),
{
    if recorded == proposed { CommitOutcome::AlreadyApplied } else { CommitOutcome::InputConflict }
}
}
