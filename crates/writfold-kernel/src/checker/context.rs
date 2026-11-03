use vstd::prelude::*;
use writfold_domain::{Integer, *};

verus! {

/// A candidate snapshot changes only the frame, never sources, policy or time.
#[derive(Clone, Copy)]
pub struct Frame<'a> {
    pub base: &'a Context,
    pub snapshot: &'a SnapshotId,
}

#[cfg(verus_keep_ghost)]
pub ghost struct FrameView {
    pub base: ContextView,
    pub snapshot: Seq<char>,
}
#[cfg(verus_keep_ghost)]
impl View for Frame<'_> {
    type V = FrameView;
    open spec fn view(&self) -> FrameView {
        FrameView { base: self.base@, snapshot: self.snapshot@ }
    }
}
#[cfg(verus_keep_ghost)]
impl DeepView for Frame<'_> {
    type V = FrameView;
    open spec fn deep_view(&self) -> FrameView { self@ }
}
#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Frame<'_> {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { self@ == other@ }
}
impl PartialEq for Frame<'_> {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (self@ == other@),
    { self.base == other.base && self.snapshot == other.snapshot }
}

pub open spec fn effective_context(frame: FrameView) -> ContextView {
    ContextView { snapshot: frame.snapshot, ..frame.base }
}
pub open spec fn normalized_context(frame: FrameView) -> ContextView {
    ContextView { snapshot: frame.snapshot, now: 0, ..frame.base }
}
pub open spec fn same_context_spec(left: ContextView, right: ContextView) -> bool {
    left.snapshot == right.snapshot && left.semantic_version == right.semantic_version
        && left.ruleset == right.ruleset && left.checker == right.checker
        && left.policy == right.policy && left.target == right.target
        && left.intended_use == right.intended_use && left.sources == right.sources
}

impl<'a> Frame<'a> {
    pub fn new(base: &'a Context) -> (frame: Self)
        ensures frame@.base == base@, frame@.snapshot == base@.snapshot,
    {
        Self {
            base,
            snapshot: base.snapshot(),
        }
    }
    pub fn matches(self, kb: &KB) -> (matches: bool)
        ensures matches == (kb@.snapshot == self@.snapshot
            && crate::wellformed::spec::context_wf_spec(self@.base)),
    {
        kb.snapshot() == self.snapshot && crate::wellformed::context_wf(self.base)
    }
    pub fn now(self) -> (now: &'a Integer)
        ensures now@ == self@.base.now,
    {
        self.base.now()
    }
    pub fn policy(self) -> (policy: &'a Policy)
        ensures policy@ == self@.base.policy,
    {
        self.base.policy()
    }
    pub fn target(self) -> (target: &'a Scope)
        ensures target@ == self@.base.target,
    {
        self.base.target()
    }
    pub fn same_context(self, other: &Context) -> (same: bool)
        ensures same == same_context_spec(effective_context(self@), other@),
    {
        proof { broadcast use writfold_domain::equality::vector_equality; }
        self.snapshot == other.snapshot()
            && self.base.semantic_version().as_str().eq(other.semantic_version().as_str())
            && self.base.ruleset() == other.ruleset()
            && self.base.checker() == other.checker()
            && self.base.policy() == other.policy()
            && self.base.target() == other.target()
            && self.base.intended_use().as_str().eq(other.intended_use().as_str())
            && self.base.sources() == other.sources()
    }
    pub fn normalized(self) -> (normalized: Context)
        ensures normalized@ == normalized_context(self@),
    {
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

} // verus!
