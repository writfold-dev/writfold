use vstd::prelude::*;
use writfold_domain::{AliasId, EpochSeconds, citation::*};

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AliasIssueError { Denied, Foreign, Expired, InvalidLifetime, AlreadyIssued }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AliasResult<'a> { Durable(Binding<'a>), Expired, Foreign, Unavailable, Denied }

#[derive(Clone, Copy, Debug)]
pub struct AliasEntry<'a> {
    handle: &'a AliasId, binding: Binding<'a>, issued_at: EpochSeconds, expires_at: EpochSeconds,
}
impl<'a> AliasEntry<'a> {
    pub closed spec fn handle_value(self) -> Seq<char> { self.handle@ }
    pub closed spec fn binding_value(self) -> BindingView { self.binding@ }
    pub closed spec fn issue_value(self) -> nat { self.issued_at@ }
    pub closed spec fn expiry_value(self) -> nat { self.expires_at@ }
    pub fn handle(self) -> (value: &'a AliasId)
        ensures value@ == self.handle_value(),
    { self.handle }
    pub fn binding(self) -> (value: Binding<'a>)
        ensures value@ == self.binding_value(),
    { self.binding }
    pub fn issued_at(self) -> (value: EpochSeconds)
        ensures value@ == self.issue_value(),
    { self.issued_at }
    pub fn expires_at(self) -> (value: EpochSeconds)
        ensures value@ == self.expiry_value(),
    { self.expires_at }
}

/// Owns only entry metadata. Durable binding material remains immutably borrowed.
/// A retired table cannot be reopened or rebound; construct a fresh scoped table.
#[derive(Debug)]
pub struct AliasTable<'a> { scope: AliasScope<'a>, entries: Vec<AliasEntry<'a>>, retired: bool }
impl<'a> AliasTable<'a> {
    pub closed spec fn scope_value(&self) -> AliasScopeView { self.scope@ }
    pub closed spec fn entries_value(&self) -> Seq<AliasEntry<'a>> { self.entries@ }
    pub closed spec fn retired_value(&self) -> bool { self.retired }
    pub fn new(scope: AliasScope<'a>) -> (value: Self)
        ensures value.scope_value() == scope@, value.entries_value().len() == 0,
            !value.retired_value(),
    { Self { scope, entries: Vec::new(), retired: false } }
    pub fn scope(&self) -> (value: AliasScope<'a>)
        ensures value@ == self.scope_value(),
    { self.scope }
    pub fn entries(&self) -> (value: &[AliasEntry<'a>])
        ensures value@ == self.entries_value(),
    { self.entries.as_slice() }
    pub fn retire(&mut self)
        ensures final(self).retired_value(), final(self).scope_value() == old(self).scope_value(),
            final(self).entries_value() == old(self).entries_value(),
    { self.retired = true; }
    pub fn lookup(&self, handle: &AliasId) -> (entry: Option<AliasEntry<'a>>)
        ensures match entry {
            None => forall|i: int| 0 <= i < self.entries_value().len()
                ==> self.entries_value()[i].handle_value() != handle@,
            Some(found) => exists|i: int| 0 <= i < self.entries_value().len()
                && found == self.entries_value()[i] && found.handle_value() == handle@
                && forall|j: int| 0 <= j < i ==> self.entries_value()[j].handle_value() != handle@,
        },
    {
        let mut index = 0;
        while index < self.entries.len()
            invariant index <= self.entries.len(),
                forall|i: int| 0 <= i < index ==> self.entries@[i].handle_value() != handle@,
            decreases self.entries.len() - index,
        {
            let entry = self.entries[index];
            if entry.handle == handle {
                proof {
                    assert(self.entries_value()[index as int] == entry);
                    assert(entry.handle_value() == handle@);
                    assert(forall|j: int| 0 <= j < index
                        ==> self.entries_value()[j].handle_value() != handle@);
                }
                return Some(entry);
            }
            index += 1;
        }
        None
    }
    pub fn issue(&mut self, handle: &'a AliasId, binding: Binding<'a>,
        issued_at: EpochSeconds, expires_at: EpochSeconds, access: AccessDecision<'_>)
        -> (result: Result<(), AliasIssueError>)
        ensures final(self).scope_value() == old(self).scope_value(),
            final(self).retired_value() == old(self).retired_value(),
            result.is_err() ==> final(self).entries_value() == old(self).entries_value(),
            result.is_ok() ==> final(self).entries_value().len() == old(self).entries_value().len() + 1,
            result.is_ok() ==> final(self).entries_value().drop_last() == old(self).entries_value(),
            result.is_ok() ==> final(self).entries_value().last().binding_value() == binding@,
            result.is_ok() ==> final(self).entries_value().last().handle_value() == handle@,
            result.is_ok() ==> issued_at@ <= expires_at@
                && old(self).scope_value().realm == binding@.representation.realm
                && old(self).scope_value().release == binding@.release
                && old(self).scope_value().context == binding@.context
                && access.permitted_value() && access.binding_value() == binding@
                && forall|i: int| 0 <= i < old(self).entries_value().len()
                    ==> old(self).entries_value()[i].handle_value() != handle@,
    {
        if !access.allows(binding) { return Err(AliasIssueError::Denied); }
        if self.retired { return Err(AliasIssueError::Expired); }
        if !self.scope.fits(binding) { return Err(AliasIssueError::Foreign); }
        if issued_at.get() > expires_at.get() { return Err(AliasIssueError::InvalidLifetime); }
        if self.lookup(handle).is_some() { return Err(AliasIssueError::AlreadyIssued); }
        self.entries.push(AliasEntry { handle, binding, issued_at, expires_at });
        Ok(())
    }
    /// The host obtains the exact retained entry, recomputes current access, and
    /// supplies a decision tied to that binding. A decision for another role fails.
    pub fn expand(&self, current: AliasScope<'_>, handle: &AliasId, now: EpochSeconds,
        access: AccessDecision<'_>) -> (result: AliasResult<'a>)
        ensures match result {
            AliasResult::Durable(binding) => !self.retired_value()
                && self.scope_value().realm == current@.realm
                && self.scope_value().principal == current@.principal
                && self.scope_value().instance == current@.instance
                && self.scope_value().incarnation == current@.incarnation
                && self.scope_value().table_generation == current@.table_generation
                && self.scope_value().release == current@.release
                && self.scope_value().context == current@.context
                && binding@.representation.realm == current@.realm
                && binding@.release == current@.release && binding@.context == current@.context
                && access.permitted_value() && access.binding_value() == binding@
                && exists|i: int| 0 <= i < self.entries_value().len()
                    && self.entries_value()[i].handle_value() == handle@
                    && self.entries_value()[i].binding_value() == binding@
                    && self.entries_value()[i].issue_value() <= now@ <= self.entries_value()[i].expiry_value(),
            _ => true,
        },
    {
        if self.retired { return AliasResult::Expired; }
        if !self.scope.same_audience(current) { return AliasResult::Foreign; }
        if self.scope.table_generation().get() != current.table_generation().get() { return AliasResult::Expired; }
        let Some(entry) = self.lookup(handle) else { return AliasResult::Unavailable; };
        if !self.scope.fits(entry.binding) { return AliasResult::Foreign; }
        if now.get() < entry.issued_at.get() || now.get() > entry.expires_at.get() {
            return AliasResult::Expired;
        }
        if !access.allows(entry.binding) { return AliasResult::Denied; }
        AliasResult::Durable(entry.binding)
    }
}
}
