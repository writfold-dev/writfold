use super::{Binding, CitationTableGeneration};
use crate::{ActorId, ContextId, InstanceId, RealmId, SessionIncarnation, SnapshotId};
use vstd::prelude::*;

verus! {
/// Session identity and citation-table epoch are distinct from request correlation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AliasScope<'a> {
    realm: &'a RealmId,
    principal: &'a ActorId,
    instance: &'a InstanceId,
    incarnation: &'a SessionIncarnation,
    release: &'a SnapshotId,
    context: &'a ContextId,
    table_generation: CitationTableGeneration,
}
#[cfg(verus_keep_ghost)]
pub ghost struct AliasScopeView {
    pub realm: Seq<char>, pub principal: Seq<char>, pub instance: Seq<char>,
    pub incarnation: Seq<char>, pub release: Seq<char>, pub context: Seq<char>,
    pub table_generation: u64,
}
#[cfg(verus_keep_ghost)]
impl View for AliasScope<'_> {
    type V = AliasScopeView;
    closed spec fn view(&self) -> Self::V {
        AliasScopeView { realm: self.realm@, principal: self.principal@,
            instance: self.instance@, incarnation: self.incarnation@,
            release: self.release@, context: self.context@, table_generation: self.table_generation.value() }
    }
}
impl<'a> AliasScope<'a> {
    #[allow(clippy::too_many_arguments, reason = "complete invocation and policy scope")]
    pub fn new(realm: &'a RealmId, principal: &'a ActorId, instance: &'a InstanceId,
        incarnation: &'a SessionIncarnation, release: &'a SnapshotId, context: &'a ContextId,
        table_generation: CitationTableGeneration) -> (value: Self)
        ensures value@ == (AliasScopeView { realm: realm@, principal: principal@,
            instance: instance@, incarnation: incarnation@, release: release@,
            context: context@, table_generation: table_generation.value() }),
    { Self { realm, principal, instance, incarnation, release, context, table_generation } }
    pub fn realm(self) -> (value: &'a RealmId)
        ensures value@ == self@.realm,
    { self.realm }
    pub fn principal(self) -> (value: &'a ActorId)
        ensures value@ == self@.principal,
    { self.principal }
    pub fn instance(self) -> (value: &'a InstanceId)
        ensures value@ == self@.instance,
    { self.instance }
    pub fn incarnation(self) -> (value: &'a SessionIncarnation)
        ensures value@ == self@.incarnation,
    { self.incarnation }
    pub fn release(self) -> (value: &'a SnapshotId)
        ensures value@ == self@.release,
    { self.release }
    pub fn context(self) -> (value: &'a ContextId)
        ensures value@ == self@.context,
    { self.context }
    pub fn table_generation(self) -> (value: CitationTableGeneration)
        ensures value.value() == self@.table_generation,
    { self.table_generation }
    pub fn same_audience(self, other: AliasScope<'_>) -> (equal: bool)
        ensures equal == (self@.realm == other@.realm && self@.principal == other@.principal
            && self@.instance == other@.instance && self@.incarnation == other@.incarnation
            && self@.release == other@.release && self@.context == other@.context),
    {
        self.realm == other.realm && self.principal == other.principal
            && self.instance == other.instance && self.incarnation == other.incarnation
            && self.release == other.release && self.context == other.context
    }
    pub fn same(self, other: AliasScope<'_>) -> (equal: bool)
        ensures equal == (self@ == other@),
    { self.same_audience(other) && self.table_generation.get() == other.table_generation.get() }
    pub fn fits(self, binding: Binding<'_>) -> (fits: bool)
        ensures fits == (self@.realm == binding@.representation.realm
            && self@.release == binding@.release && self@.context == binding@.context),
    {
        self.realm == binding.representation().realm()
            && self.release == binding.release() && self.context == binding.context()
    }
}
}
