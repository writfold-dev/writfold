use writfold_domain::*;

/// A candidate snapshot changes only the frame, never sources, policy or time.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Frame<'a> {
    pub base: &'a Context,
    pub snapshot: &'a SnapshotId,
}

impl<'a> Frame<'a> {
    pub fn new(base: &'a Context) -> Self {
        Self {
            base,
            snapshot: base.snapshot(),
        }
    }
    pub fn matches(self, kb: &KB) -> bool {
        kb.snapshot() == self.snapshot && crate::wellformed::context_wf(self.base)
    }
    pub fn now(self) -> &'a Integer {
        self.base.now()
    }
    pub fn policy(self) -> &'a Policy {
        self.base.policy()
    }
    pub fn target(self) -> &'a Scope {
        self.base.target()
    }
    pub fn same_context(self, other: &Context) -> bool {
        self.snapshot == other.snapshot()
            && self.base.semantic_version() == other.semantic_version()
            && self.base.ruleset() == other.ruleset()
            && self.base.checker() == other.checker()
            && self.base.policy() == other.policy()
            && self.base.target() == other.target()
            && self.base.intended_use() == other.intended_use()
            && self.base.sources() == other.sources()
    }
    pub fn normalized(self) -> Context {
        Context::new(
            self.snapshot.clone(),
            self.base.semantic_version().clone(),
            self.base.ruleset().clone(),
            self.base.checker().clone(),
            self.base.policy().clone(),
            Integer::from(0),
            self.base.target().clone(),
            self.base.intended_use().clone(),
            self.base.sources().clone(),
        )
    }
}
