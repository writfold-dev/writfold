//! Exact citation model inputs, not authentication or source-acquisition capabilities.
//! Metadata ownership is explicit; query bindings and representation bytes are borrowed.
//! Only pinned Git blob byte coordinates are supported by the kernel calculus.
//! `Selector::new` rejects reversed bounds at typed-input admission. This is not
//! raw-selector response parity with the mathematical model; admitted selectors
//! still undergo access-first profile, unit, retained-length and exact-byte checks.
//! Digests label content, never authenticate it or replace the expected preimage.
//!
//! Offsets cannot be substituted for byte counts:
//! ```compile_fail
//! use writfold_domain::{ByteCount, ByteOffset};
//! fn select(_: ByteOffset) {}
//! select(ByteCount::new(4).unwrap());
//! ```
mod binding;
mod generation;
mod handles;
mod presentation;
mod selector;
use crate::{ContentId, RealmId, SourceId, SourceRevision, records::record};
pub use binding::*;
pub use generation::CitationTableGeneration;
pub use handles::*;
pub use presentation::*;
pub use selector::*;
use vstd::prelude::*;

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepresentationProfile { GitBlob, UnsupportedNativeUtf16 }
}
crate::records::scalar_views!(RepresentationProfile);

record!(
    RepresentationRef,
    RepresentationRefView {
        realm: RealmId,
        repository: SourceId,
        revision: SourceRevision,
        path: String,
        content: ContentId,
        profile: RepresentationProfile,
    }
);

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Representation<'a> {
    reference: &'a RepresentationRef,
    bytes: &'a [u8],
}
impl<'a> Representation<'a> {
    pub closed spec fn reference_value(self) -> RepresentationRefView { self.reference@ }
    pub closed spec fn bytes_value(self) -> Seq<u8> { self.bytes@ }
    pub fn new(reference: &'a RepresentationRef, bytes: &'a [u8]) -> (value: Self)
        ensures value.reference_value() == reference@, value.bytes_value() == bytes@,
    { Self { reference, bytes } }
    pub fn reference(self) -> (value: &'a RepresentationRef)
        ensures value@ == self.reference_value(),
    { self.reference }
    pub fn bytes(self) -> (value: &'a [u8])
        ensures value@ == self.bytes_value(),
    { self.bytes }
}

/// Exact byte equality: neither digest equality nor text normalization substitutes for it.
pub fn bytes_equal(left: &[u8], right: &[u8]) -> (equal: bool)
    ensures equal == (left@ == right@),
{
    if left.len() != right.len() { return false; }
    let mut index = 0;
    while index < left.len()
        invariant index <= left.len(), left.len() == right.len(),
            forall|i: int| 0 <= i < index ==> left@[i] == right@[i],
        decreases left.len() - index,
    {
        if left[index] != right[index] { return false; }
        index += 1;
    }
    proof { assert(left@ =~= right@); }
    true
}
}
