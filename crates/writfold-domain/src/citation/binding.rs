#[cfg(verus_keep_ghost)]
use super::RepresentationRefView;
use super::{RepresentationRef, Selector, bytes_equal};
use crate::{CitationId, ContextId, EvidenceRoleId, SnapshotId};
use vstd::prelude::*;

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Binding<'a> {
    id: &'a CitationId,
    release: &'a SnapshotId,
    context: &'a ContextId,
    role: &'a EvidenceRoleId,
    representation: &'a RepresentationRef,
    selector: Selector,
    expected: &'a [u8],
}
#[cfg(verus_keep_ghost)]
pub ghost struct BindingView {
    pub id: Seq<char>, pub release: Seq<char>, pub context: Seq<char>,
    pub role: Seq<char>, pub representation: RepresentationRefView,
    pub selector: (nat, nat, super::CoordinateUnit), pub expected: Seq<u8>,
}
#[cfg(verus_keep_ghost)]
impl View for Binding<'_> {
    type V = BindingView;
    closed spec fn view(&self) -> Self::V {
        BindingView { id: self.id@, release: self.release@, context: self.context@,
            role: self.role@, representation: self.representation@,
            selector: self.selector@, expected: self.expected@ }
    }
}
impl<'a> Binding<'a> {
    #[allow(clippy::too_many_arguments, reason = "exact immutable citation material")]
    pub fn new(id: &'a CitationId, release: &'a SnapshotId, context: &'a ContextId,
        role: &'a EvidenceRoleId, representation: &'a RepresentationRef,
        selector: Selector, expected: &'a [u8]) -> (value: Self)
        ensures value@ == (BindingView { id: id@, release: release@, context: context@,
            role: role@, representation: representation@, selector: selector@, expected: expected@ }),
    { Self { id, release, context, role, representation, selector, expected } }
    pub fn id(self) -> (value: &'a CitationId)
        ensures value@ == self@.id,
    { self.id }
    pub fn release(self) -> (value: &'a SnapshotId)
        ensures value@ == self@.release,
    { self.release }
    pub fn context(self) -> (value: &'a ContextId)
        ensures value@ == self@.context,
    { self.context }
    pub fn role(self) -> (value: &'a EvidenceRoleId)
        ensures value@ == self@.role,
    { self.role }
    pub fn representation(self) -> (value: &'a RepresentationRef)
        ensures value@ == self@.representation,
    { self.representation }
    pub fn selector(self) -> (value: Selector)
        ensures value@ == self@.selector,
    { self.selector }
    pub fn expected(self) -> (value: &'a [u8])
        ensures value@ == self@.expected,
    { self.expected }
    pub fn same(self, other: Binding<'_>) -> (equal: bool)
        ensures equal == (self@ == other@),
    {
        self.id == other.id && self.release == other.release
            && self.context == other.context && self.role == other.role
            && self.representation == other.representation && self.selector.same(other.selector)
            && bytes_equal(self.expected, other.expected)
    }
}

/// A caller-supplied current policy decision about exact material, not a credential.
/// Authenticating the caller and evaluating current policy remain host obligations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccessDecision<'a> { binding: Binding<'a>, permitted: bool }
impl<'a> AccessDecision<'a> {
    pub closed spec fn binding_value(self) -> BindingView { self.binding@ }
    pub closed spec fn permitted_value(self) -> bool { self.permitted }
    pub fn new(binding: Binding<'a>, permitted: bool) -> (value: Self)
        ensures value.binding_value() == binding@, value.permitted_value() == permitted,
    { Self { binding, permitted } }
    pub fn allows(self, binding: Binding<'_>) -> (allowed: bool)
        ensures allowed == (self.permitted_value() && self.binding_value() == binding@),
    { self.permitted && self.binding.same(binding) }
}
}
