#[cfg(verus_keep_ghost)]
use super::AliasScopeView;
use super::{AliasScope, Coverage};
use crate::ContentId;
use vstd::prelude::*;

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoticeKind { Evidence, Contrary, Stale, Ambiguous, Omission }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Notice<'a> { kind: NoticeKind, text: &'a str }
impl<'a> Notice<'a> {
    pub closed spec fn material(self) -> (NoticeKind, Seq<char>) { (self.kind, self.text@) }
    pub fn new(kind: NoticeKind, text: &'a str) -> (value: Self)
        ensures value.material() == (kind, text@),
    { Self { kind, text } }
    pub fn kind(self) -> (value: NoticeKind)
        ensures value == self.material().0,
    { self.kind }
    pub fn text(self) -> (value: &'a str)
        ensures value@ == self.material().1,
    { self.text }
}

/// Immutable query closure. Cursor bindings include actual notices and coverage,
/// not just a caller-selected query name. The budget counts notices, not scan work.
#[derive(Debug)]
pub struct NoticeQuery<'a> {
    scope: AliasScope<'a>, identity: &'a ContentId, ordering_profile: &'a ContentId,
    budget: usize, notices: &'a [Notice<'a>], coverage: Coverage,
}
#[cfg(verus_keep_ghost)]
pub ghost struct NoticeQueryView {
    pub scope: AliasScopeView, pub identity: Seq<char>, pub ordering_profile: Seq<char>,
    pub budget: nat, pub notices: Seq<(NoticeKind, Seq<char>)>, pub coverage: Coverage,
}
#[cfg(verus_keep_ghost)]
impl View for NoticeQuery<'_> {
    type V = NoticeQueryView;
    closed spec fn view(&self) -> Self::V {
        NoticeQueryView { scope: self.scope@, identity: self.identity@,
            ordering_profile: self.ordering_profile@, budget: self.budget as nat,
            notices: self.notices@.map_values(|notice: Notice| notice.material()), coverage: self.coverage }
    }
}
impl<'a> NoticeQuery<'a> {
    pub fn new(scope: AliasScope<'a>, identity: &'a ContentId, ordering_profile: &'a ContentId,
        budget: usize, notices: &'a [Notice<'a>], coverage: Coverage) -> (value: Self)
        ensures value@ == (NoticeQueryView { scope: scope@, identity: identity@,
            ordering_profile: ordering_profile@, budget: budget as nat,
            notices: notices@.map_values(|notice: Notice| notice.material()), coverage }),
    { Self { scope, identity, ordering_profile, budget, notices, coverage } }
    pub fn scope(&self) -> (value: AliasScope<'a>)
        ensures value@ == self@.scope,
    { self.scope }
    pub fn identity(&self) -> (value: &'a ContentId)
        ensures value@ == self@.identity,
    { self.identity }
    pub fn ordering_profile(&self) -> (value: &'a ContentId)
        ensures value@ == self@.ordering_profile,
    { self.ordering_profile }
    pub fn budget(&self) -> (value: usize)
        ensures value == self@.budget,
    { self.budget }
    pub fn notices(&self) -> (value: &'a [Notice<'a>])
        ensures value@.map_values(|notice: Notice| notice.material()) == self@.notices,
    { self.notices }
    pub fn coverage(&self) -> (value: Coverage)
        ensures value == self@.coverage,
    { self.coverage }
    pub fn same(&self, other: &NoticeQuery<'_>) -> (equal: bool)
        ensures equal == (self@ == other@),
    {
        proof {
            assert(self@.notices.len() == self.notices.len());
            assert(other@.notices.len() == other.notices.len());
        }
        if !self.scope.same(other.scope) || self.identity != other.identity
            || self.ordering_profile != other.ordering_profile || self.budget != other.budget
            || self.coverage != other.coverage || self.notices.len() != other.notices.len() {
            return false;
        }
        let mut index = 0;
        while index < self.notices.len()
            invariant index <= self.notices.len(), self.notices.len() == other.notices.len(),
                self@.scope == other@.scope, self@.identity == other@.identity,
                self@.ordering_profile == other@.ordering_profile,
                self@.budget == other@.budget, self@.coverage == other@.coverage,
                forall|i: int| 0 <= i < index ==> self@.notices[i] == other@.notices[i],
            decreases self.notices.len() - index,
        {
            let left = self.notices[index];
            let right = other.notices[index];
            if left.kind != right.kind || left.text != right.text {
                proof {
                    assert(self@.notices[index as int] == left.material());
                    assert(other@.notices[index as int] == right.material());
                    assert(self@.notices[index as int] != other@.notices[index as int]);
                }
                return false;
            }
            index += 1;
        }
        proof { assert(self@.notices =~= other@.notices); }
        true
    }
}

/// Borrows a stable immutable query; neither notice lists nor byte preimages are cloned.
#[derive(Clone, Copy, Debug)]
pub struct NoticeCursor<'a> { query: &'a NoticeQuery<'a>, offset: usize }
impl<'a> NoticeCursor<'a> {
    pub closed spec fn query_value(self) -> NoticeQueryView { self.query@ }
    pub closed spec fn offset_value(self) -> nat { self.offset as nat }
    pub fn new(query: &'a NoticeQuery<'a>, offset: usize) -> (value: Self)
        ensures value.query_value() == query@, value.offset_value() == offset,
    { Self { query, offset } }
    pub fn query(self) -> (value: &'a NoticeQuery<'a>)
        ensures value@ == self.query_value(),
    { self.query }
    pub fn offset(self) -> (value: usize)
        ensures value == self.offset_value(),
    { self.offset }
}
}
crate::records::scalar_views!(NoticeKind);
