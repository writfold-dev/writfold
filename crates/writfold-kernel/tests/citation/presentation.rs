use super::fixture::{Fixture, TestResult};
use writfold_domain::{citation::*, *};
use writfold_kernel::citation::*;

#[test]
fn contrary_notices_remain_omitted_even_on_final_page_and_coverage_is_separate() -> TestResult {
    let f = Fixture::new()?;
    let identity: ContentId = "query".parse()?;
    let order: ContentId = "order".parse()?;
    let notices = [
        Notice::new(NoticeKind::Evidence, "supports"),
        Notice::new(NoticeKind::Contrary, "refutes"),
    ];
    for coverage in [
        Coverage::Unavailable,
        Coverage::Measured {
            examined: 2,
            total: 9,
        },
        Coverage::Measured {
            examined: 9,
            total: 9,
        },
    ] {
        let query = NoticeQuery::new(f.scope(0), &identity, &order, 1, &notices, coverage);
        let first = present_notices(&query, None).map_err(|error| format!("{error:?}"))?;
        assert_eq!(first.notices(), &notices[..1]);
        assert_eq!(first.notices().as_ptr(), notices.as_ptr());
        assert_eq!(first.offset(), 0);
        assert_eq!(first.omitted_count(), 1);
        assert!(!first.presentation_complete());
        assert_eq!(first.coverage(), coverage);
        let second = present_notices(&query, first.next()).map_err(|error| format!("{error:?}"))?;
        assert_eq!(second.notices(), &notices[1..]);
        assert_eq!(second.notices()[0].kind(), NoticeKind::Contrary);
        assert_eq!(second.offset(), 1);
        assert_eq!(second.omitted_count(), 1);
        assert!(!second.presentation_complete());
        assert!(second.next().is_none());
        assert_eq!(second.coverage(), coverage);
    }
    Ok(())
}

#[test]
fn cursor_rejects_substituted_closure_even_under_the_same_query_name() -> TestResult {
    let f = Fixture::new()?;
    let identity: ContentId = "query".parse()?;
    let other: ContentId = "other".parse()?;
    let order: ContentId = "order".parse()?;
    let notices = [
        Notice::new(NoticeKind::Evidence, "a"),
        Notice::new(NoticeKind::Contrary, "b"),
    ];
    let substituted = [
        Notice::new(NoticeKind::Evidence, "a"),
        Notice::new(NoticeKind::Evidence, "b"),
    ];
    let changed_text = [
        Notice::new(NoticeKind::Evidence, "a"),
        Notice::new(NoticeKind::Contrary, "c"),
    ];
    let query = NoticeQuery::new(
        f.scope(0),
        &identity,
        &order,
        1,
        &notices,
        Coverage::Unavailable,
    );
    let cursor = present_notices(&query, None)
        .map_err(|error| format!("{error:?}"))?
        .next();
    for field in [
        "scope", "identity", "ordering", "budget", "role", "text", "count", "coverage",
    ] {
        let other_query = NoticeQuery::new(
            f.scope(u64::from(field == "scope")),
            if field == "identity" {
                &other
            } else {
                &identity
            },
            if field == "ordering" { &other } else { &order },
            if field == "budget" { 2 } else { 1 },
            match field {
                "role" => &substituted,
                "text" => &changed_text,
                "count" => &notices[..1],
                _ => &notices,
            },
            if field == "coverage" {
                Coverage::Measured {
                    examined: 0,
                    total: 0,
                }
            } else {
                Coverage::Unavailable
            },
        );
        assert!(matches!(
            present_notices(&other_query, cursor),
            Err(PresentationError::CursorMismatch)
        ));
    }
    Ok(())
}

#[test]
fn empty_complete_page_zero_budget_and_invalid_offset_are_distinct() -> TestResult {
    let f = Fixture::new()?;
    let identity: ContentId = "query".parse()?;
    let query = NoticeQuery::new(
        f.scope(0),
        &identity,
        &identity,
        1,
        &[],
        Coverage::Unavailable,
    );
    let page = present_notices(&query, None).map_err(|error| format!("{error:?}"))?;
    assert!(page.presentation_complete());
    assert!(page.notices().is_empty());
    assert_eq!(page.omitted_count(), 0);
    assert!(!page.coverage().complete());
    assert!(page.next().is_none());
    assert!(matches!(
        present_notices(&query, Some(NoticeCursor::new(&query, 1))),
        Err(PresentationError::InvalidOffset)
    ));
    let zero = NoticeQuery::new(
        f.scope(0),
        &identity,
        &identity,
        0,
        &[],
        Coverage::Unavailable,
    );
    assert!(matches!(
        present_notices(&zero, None),
        Err(PresentationError::ZeroBudget)
    ));
    Ok(())
}
