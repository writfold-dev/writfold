//! Rejected and replayed operations retain the exact borrowed inputs.
use std::borrow::Cow;
use writfold_domain::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowResult<'a, S: Clone, J: Clone> {
    state: Cow<'a, S>,
    journal: Cow<'a, [J]>,
    outcome: CommitOutcome,
}
impl<'a, S: Clone, J: Clone> WorkflowResult<'a, S, J> {
    pub(crate) fn unchanged(state: &'a S, journal: &'a [J], outcome: CommitOutcome) -> Self {
        Self {
            state: Cow::Borrowed(state),
            journal: Cow::Borrowed(journal),
            outcome,
        }
    }
    pub(crate) fn applied(state: S, journal: Vec<J>) -> Self {
        Self {
            state: Cow::Owned(state),
            journal: Cow::Owned(journal),
            outcome: CommitOutcome::Applied,
        }
    }
    pub fn state(&self) -> &S {
        self.state.as_ref()
    }
    pub fn journal(&self) -> &[J] {
        self.journal.as_ref()
    }
    pub fn outcome(&self) -> &CommitOutcome {
        &self.outcome
    }
    pub fn into_parts(self) -> (Cow<'a, S>, Cow<'a, [J]>, CommitOutcome) {
        (self.state, self.journal, self.outcome)
    }
}
impl WorkflowResult<'_, KB, JournalEntry> {
    pub fn kb(&self) -> &KB {
        self.state()
    }
}
pub type Application<'a> = WorkflowResult<'a, KB, JournalEntry>;
pub type SelectionResult<'a> = WorkflowResult<'a, SelectionState, SelectionJournalEntry>;
pub type PortfolioResult<'a> = WorkflowResult<'a, PortfolioState, PortfolioJournalEntry>;
pub type PortfolioSelectionResult<'a> =
    WorkflowResult<'a, PortfolioState, PortfolioSelectionJournalEntry>;

pub(crate) fn replay_outcome<P: PartialEq>(recorded: &P, proposed: &P) -> CommitOutcome {
    if recorded == proposed {
        CommitOutcome::AlreadyApplied
    } else {
        CommitOutcome::InputConflict
    }
}
