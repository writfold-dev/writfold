use vstd::prelude::*;
use writfold_domain::citation::*;

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidReason { UnsupportedProfile, MixedUnits, InvalidBounds, SelectionMismatch }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolveResult<'a> {
    Exact { bytes: &'a [u8], provenance: &'a RepresentationRef },
    Invalid(InvalidReason), Unavailable, Denied,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetainError { AlreadyRetained }

pub open spec fn first_representation<'a>(store: Seq<Representation<'a>>,
    reference: RepresentationRefView, found: Option<Representation<'a>>) -> bool {
    match found {
        None => forall|i: int| 0 <= i < store.len() ==> store[i].reference_value() != reference,
        Some(object) => exists|i: int| 0 <= i < store.len() && object == store[i]
            && store[i].reference_value() == reference
            && forall|j: int| 0 <= j < i ==> store[j].reference_value() != reference,
    }
}

/// Lookup uses the full immutable provenance key, not content identity alone.
pub fn lookup_representation<'a>(store: &[Representation<'a>], reference: &RepresentationRef)
    -> (found: Option<Representation<'a>>)
    ensures first_representation(store@, reference@, found),
{
    let mut index = 0;
    while index < store.len()
        invariant index <= store.len(),
            forall|i: int| 0 <= i < index ==> store@[i].reference_value() != reference@,
        decreases store.len() - index,
    {
        let object = store[index];
        if object.reference() == reference { return Some(object); }
        index += 1;
    }
    None
}

/// Retention appends borrowed immutable ownership; it never overwrites a key.
pub fn retain<'a>(store: &mut Vec<Representation<'a>>, object: Representation<'a>)
    -> (result: Result<(), RetainError>)
    ensures result.is_ok() ==> final(store)@ == old(store)@.push(object),
        result.is_err() ==> final(store)@ == old(store)@,
        result.is_ok() ==> forall|i: int| 0 <= i < old(store).len()
            ==> old(store)@[i].reference_value() != object.reference_value(),
{
    if lookup_representation(store.as_slice(), object.reference()).is_some() {
        return Err(RetainError::AlreadyRetained);
    }
    store.push(object);
    Ok(())
}

/// Current access precedes unsupported selectors, existence, and byte diagnostics.
pub fn resolve<'a>(store: &[Representation<'a>], binding: Binding<'_>, access: AccessDecision<'_>)
    -> (result: ResolveResult<'a>)
    ensures
        !access.permitted_value() || access.binding_value() != binding@ ==> result is Denied,
        match result {
            ResolveResult::Exact { bytes, provenance } =>
                access.permitted_value() && access.binding_value() == binding@
                && provenance@ == binding@.representation && bytes@ == binding@.expected
                && binding@.representation.profile == RepresentationProfile::GitBlob
                && binding@.selector.2 == CoordinateUnit::Byte
                && exists|i: int| 0 <= i < store.len()
                    && store@[i].reference_value() == provenance@
                    && binding@.selector.0 <= binding@.selector.1 <= store@[i].bytes_value().len()
                    && bytes@ == store@[i].bytes_value().subrange(
                        binding@.selector.0 as int, binding@.selector.1 as int),
            _ => true,
        },
{
    if !access.allows(binding) { return ResolveResult::Denied; }
    if *binding.representation().profile() != RepresentationProfile::GitBlob {
        return ResolveResult::Invalid(InvalidReason::UnsupportedProfile);
    }
    let selector = binding.selector();
    if selector.unit() != CoordinateUnit::Byte {
        return ResolveResult::Invalid(InvalidReason::MixedUnits);
    }
    let Some(object) = lookup_representation(store, binding.representation()) else {
        return ResolveResult::Unavailable;
    };
    let bytes = object.bytes();
    let first = selector.first().get();
    let last = selector.last().get();
    if first > last || last as u128 > bytes.len() as u128 {
        return ResolveResult::Invalid(InvalidReason::InvalidBounds);
    }
    let selected = &bytes[first as usize..last as usize];
    if !bytes_equal(selected, binding.expected()) {
        return ResolveResult::Invalid(InvalidReason::SelectionMismatch);
    }
    ResolveResult::Exact { bytes: selected, provenance: object.reference() }
}
}
