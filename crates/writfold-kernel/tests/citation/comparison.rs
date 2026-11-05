use super::fixture::{Fixture, TestResult, allowed, selector};
use writfold_domain::{citation::*, *};
use writfold_kernel::citation::*;

fn unlimited() -> SearchBudget {
    SearchBudget::new(100, 100)
}

#[test]
fn identity_transport_allows_new_id_and_release_but_not_position_or_role() -> TestResult {
    let f = Fixture::new()?;
    let store = [Representation::new(&f.reference, b"a-a")];
    let original = f.binding(0, 1, b"a")?;
    assert_eq!(
        compare(
            &store,
            original,
            original,
            allowed(original),
            allowed(original),
            None,
            unlimited()
        ),
        CompareResult::SameBinding
    );
    let next_id: CitationId = "next".parse()?;
    let next_release: SnapshotId = "next".parse()?;
    let next = Binding::new(
        &next_id,
        &next_release,
        &f.context,
        &f.role,
        &f.reference,
        selector(0, 1)?,
        b"a",
    );
    assert_eq!(
        compare(
            &store,
            original,
            next,
            allowed(original),
            allowed(next),
            Some(CorrespondenceWitness::IdenticalPosition),
            unlimited()
        ),
        CompareResult::UniqueJustified {
            successor: next,
            witness: CorrespondenceWitness::IdenticalPosition
        }
    );
    let other_role: EvidenceRoleId = "contrary".parse()?;
    let role = Binding::new(
        &next_id,
        &next_release,
        &f.context,
        &other_role,
        &f.reference,
        selector(0, 1)?,
        b"a",
    );
    let position = Binding::new(
        &next_id,
        &next_release,
        &f.context,
        &f.role,
        &f.reference,
        selector(2, 3)?,
        b"a",
    );
    for target in [role, position] {
        assert!(!same_occurrence(original, target));
        assert!(matches!(
            compare(
                &store,
                original,
                target,
                allowed(original),
                allowed(target),
                Some(CorrespondenceWitness::IdenticalPosition),
                unlimited()
            ),
            CompareResult::Multiple { .. }
        ));
    }
    Ok(())
}

#[test]
fn singleton_and_exhaustive_empty_text_are_never_lineage_or_deletion() -> TestResult {
    let f = Fixture::new()?;
    let other = RepresentationRef::new(
        f.reference.realm().clone(),
        f.reference.repository().clone(),
        "r2".parse()?,
        f.reference.path().clone(),
        "bytes2".parse()?,
        RepresentationProfile::GitBlob,
    );
    let original = f.binding(0, 1, b"a")?;
    let target = f.binding_at(&other, 0, 1, b"a")?;
    for (bytes, expected) in [
        (&b"a-b"[..], UnknownReason::UnsupportedCorrespondence),
        (&b"bbb"[..], UnknownReason::NoTextMatch),
    ] {
        let store = [
            Representation::new(&f.reference, b"a"),
            Representation::new(&other, bytes),
        ];
        match compare(
            &store,
            original,
            target,
            allowed(original),
            allowed(target),
            None,
            unlimited(),
        ) {
            CompareResult::Unknown {
                reason, coverage, ..
            } => {
                assert_eq!(reason, expected);
                assert!(coverage.complete());
            }
            other => return Err(format!("text justified a correspondence: {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn original_invalid_reason_and_unavailability_are_not_conflated() -> TestResult {
    let f = Fixture::new()?;
    let good = f.binding(0, 1, b"a")?;
    let bad = f.binding(0, 1, b"z")?;
    let store = [Representation::new(&f.reference, b"a")];
    assert_eq!(
        compare(
            &store,
            bad,
            good,
            allowed(bad),
            allowed(good),
            None,
            unlimited()
        ),
        CompareResult::Unknown {
            reason: UnknownReason::OriginalInvalid(InvalidReason::SelectionMismatch),
            candidates: vec![],
            coverage: Coverage::Unavailable
        }
    );
    assert_eq!(
        compare(
            &[],
            good,
            good,
            allowed(good),
            allowed(good),
            None,
            unlimited()
        ),
        CompareResult::Unknown {
            reason: UnknownReason::OriginalUnavailable,
            candidates: vec![],
            coverage: Coverage::Unavailable
        }
    );
    for (left, right) in [
        (AccessDecision::new(bad, false), allowed(good)),
        (allowed(bad), AccessDecision::new(good, false)),
    ] {
        assert_eq!(
            compare(&store, bad, good, left, right, None, unlimited()),
            CompareResult::Denied
        );
    }
    Ok(())
}

#[test]
fn search_budgets_measure_actual_prefix_without_allocating_possible_positions() -> TestResult {
    let cases = [
        (SearchBudget::new(0, 9), 0, 0),
        (SearchBudget::new(9, 0), 0, 0),
        (SearchBudget::new(2, 9), 2, 1),
        (SearchBudget::new(9, 1), 1, 1),
        (SearchBudget::new(9, 2), 3, 2),
        (SearchBudget::new(9, 9), 5, 3),
    ];
    for (budget, examined, count) in cases {
        let scan = diagnose(b"a-a-a", b"a", budget);
        assert_eq!(scan.coverage(), Coverage::Measured { examined, total: 5 });
        assert_eq!(scan.candidates().len(), count);
        assert_eq!(scan.coverage().complete(), examined == 5);
        for candidate in scan.candidates() {
            assert!(candidate.first().get() < examined as u64);
            assert_eq!(
                &b"a-a-a"[candidate.first().get() as usize..candidate.last().get() as usize],
                b"a"
            );
        }
    }
    let empty = diagnose(b"ab", b"", unlimited());
    assert_eq!(
        empty.candidates(),
        &[selector(0, 0)?, selector(1, 1)?, selector(2, 2)?]
    );
    assert_eq!(
        empty.coverage(),
        Coverage::Measured {
            examined: 3,
            total: 3
        }
    );
    let impossible = diagnose(b"a", b"longer", SearchBudget::new(0, 0));
    assert_eq!(
        impossible.coverage(),
        Coverage::Measured {
            examined: 0,
            total: 0
        }
    );
    assert!(impossible.coverage().complete());
    Ok(())
}

#[test]
fn partial_target_scan_and_missing_target_remain_distinct() -> TestResult {
    let f = Fixture::new()?;
    let other = RepresentationRef::new(
        f.reference.realm().clone(),
        f.reference.repository().clone(),
        "r2".parse()?,
        f.reference.path().clone(),
        "bytes2".parse()?,
        RepresentationProfile::GitBlob,
    );
    let original = f.binding(0, 1, b"a")?;
    let target = f.binding_at(&other, 0, 1, b"a")?;
    let first = Representation::new(&f.reference, b"a");
    assert_eq!(
        compare(
            &[first],
            original,
            target,
            allowed(original),
            allowed(target),
            None,
            unlimited()
        ),
        CompareResult::Unknown {
            reason: UnknownReason::TargetUnavailable,
            candidates: vec![],
            coverage: Coverage::Unavailable
        }
    );
    let store = [first, Representation::new(&other, b"bba")];
    assert_eq!(
        compare(
            &store,
            original,
            target,
            allowed(original),
            allowed(target),
            None,
            SearchBudget::new(2, 9)
        ),
        CompareResult::Unknown {
            reason: UnknownReason::IncompleteSearch,
            candidates: vec![],
            coverage: Coverage::Measured {
                examined: 2,
                total: 3
            }
        }
    );
    let foreign: ContextId = "foreign".parse()?;
    let foreign = Binding::new(
        target.id(),
        target.release(),
        &foreign,
        target.role(),
        target.representation(),
        target.selector(),
        target.expected(),
    );
    assert_eq!(
        compare(
            &store,
            original,
            foreign,
            allowed(original),
            allowed(foreign),
            None,
            unlimited()
        ),
        CompareResult::Unknown {
            reason: UnknownReason::ScopeMismatch,
            candidates: vec![],
            coverage: Coverage::Unavailable
        }
    );
    Ok(())
}
