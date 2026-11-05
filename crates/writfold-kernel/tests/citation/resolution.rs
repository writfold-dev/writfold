use super::fixture::{Fixture, TestResult, allowed};
use writfold_domain::{citation::*, *};
use writfold_kernel::citation::*;

#[test]
fn exact_empty_boundary_and_out_of_bounds_are_distinct() -> TestResult {
    let f = Fixture::new()?;
    let bytes = b"abc";
    let store = [Representation::new(&f.reference, bytes)];
    for (first, last, expected) in [
        (0, 3, &bytes[..]),
        (0, 0, &b""[..]),
        (3, 3, &b""[..]),
        (1, 2, &b"b"[..]),
    ] {
        let binding = f.binding(first, last, expected)?;
        match resolve(&store, binding, allowed(binding)) {
            ResolveResult::Exact {
                bytes: selected,
                provenance,
            } => {
                assert_eq!(selected, expected);
                assert_eq!(provenance, &f.reference);
                assert_eq!(selected.as_ptr(), bytes[first as usize..].as_ptr());
            }
            other => return Err(format!("unexpected resolution: {other:?}").into()),
        }
    }
    for (first, last) in [(0, 4), (4, 4), (u64::MAX, u64::MAX)] {
        let binding = f.binding(first, last, b"")?;
        assert_eq!(
            resolve(&store, binding, allowed(binding)),
            ResolveResult::Invalid(InvalidReason::InvalidBounds)
        );
    }
    let wrong = f.binding(0, 1, b"z")?;
    assert_eq!(
        resolve(&store, wrong, allowed(wrong)),
        ResolveResult::Invalid(InvalidReason::SelectionMismatch)
    );
    Ok(())
}

#[test]
fn denial_precedes_existence_profile_bounds_and_selection_errors() -> TestResult {
    let f = Fixture::new()?;
    let unsupported = RepresentationRef::new(
        f.reference.realm().clone(),
        f.reference.repository().clone(),
        f.reference.revision().clone(),
        f.reference.path().clone(),
        f.reference.content().clone(),
        RepresentationProfile::UnsupportedNativeUtf16,
    );
    for reference in [&f.reference, &unsupported] {
        let binding = f.binding_at(reference, u64::MAX, u64::MAX, b"x")?;
        for store in [vec![], vec![Representation::new(reference, b"abc")]] {
            assert_eq!(
                resolve(&store, binding, AccessDecision::new(binding, false)),
                ResolveResult::Denied
            );
        }
    }
    let binding = f.binding(0, 1, b"a")?;
    let other = f.binding(1, 2, b"b")?;
    assert_eq!(resolve(&[], binding, allowed(other)), ResolveResult::Denied);
    let unsupported = f.binding_at(&unsupported, 0, 0, b"")?;
    assert_eq!(
        resolve(&[], unsupported, allowed(unsupported)),
        ResolveResult::Invalid(InvalidReason::UnsupportedProfile)
    );
    let utf16 = Binding::new(
        binding.id(),
        binding.release(),
        binding.context(),
        binding.role(),
        binding.representation(),
        Selector::new(
            ByteOffset::new(0)?,
            ByteOffset::new(1)?,
            CoordinateUnit::Utf16,
        )
        .map_err(|error| format!("{error:?}"))?,
        b"a",
    );
    assert_eq!(
        resolve(&[], utf16, allowed(utf16)),
        ResolveResult::Invalid(InvalidReason::MixedUnits)
    );
    Ok(())
}

#[test]
fn all_provenance_dimensions_participate_in_exact_lookup() -> TestResult {
    let f = Fixture::new()?;
    let store = [Representation::new(&f.reference, b"abc")];
    let variants = [
        RepresentationRef::new(
            "other".parse()?,
            f.reference.repository().clone(),
            f.reference.revision().clone(),
            f.reference.path().clone(),
            f.reference.content().clone(),
            RepresentationProfile::GitBlob,
        ),
        RepresentationRef::new(
            f.reference.realm().clone(),
            "other".parse()?,
            f.reference.revision().clone(),
            f.reference.path().clone(),
            f.reference.content().clone(),
            RepresentationProfile::GitBlob,
        ),
        RepresentationRef::new(
            f.reference.realm().clone(),
            f.reference.repository().clone(),
            "other".parse()?,
            f.reference.path().clone(),
            f.reference.content().clone(),
            RepresentationProfile::GitBlob,
        ),
        RepresentationRef::new(
            f.reference.realm().clone(),
            f.reference.repository().clone(),
            f.reference.revision().clone(),
            "other".into(),
            f.reference.content().clone(),
            RepresentationProfile::GitBlob,
        ),
        RepresentationRef::new(
            f.reference.realm().clone(),
            f.reference.repository().clone(),
            f.reference.revision().clone(),
            f.reference.path().clone(),
            "other".parse()?,
            RepresentationProfile::GitBlob,
        ),
    ];
    for reference in &variants {
        let binding = f.binding_at(reference, 0, 1, b"a")?;
        assert_eq!(
            resolve(&store, binding, allowed(binding)),
            ResolveResult::Unavailable
        );
    }
    Ok(())
}

#[test]
fn retention_never_rebinds_and_hostile_duplicate_lookup_is_first_match() -> TestResult {
    let f = Fixture::new()?;
    let first = Representation::new(&f.reference, b"abc");
    let replacement = Representation::new(&f.reference, b"xyz");
    let mut retained = Vec::new();
    assert_eq!(retain(&mut retained, first), Ok(()));
    assert_eq!(
        retain(&mut retained, replacement),
        Err(RetainError::AlreadyRetained)
    );
    assert_eq!(
        retain(&mut retained, first),
        Err(RetainError::AlreadyRetained)
    );
    assert_eq!(retained, vec![first]);
    assert_eq!(
        lookup_representation(&[first, replacement], &f.reference),
        Some(first)
    );
    Ok(())
}
