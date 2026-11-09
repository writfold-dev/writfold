use super::{ConformanceFixture, Outcome, declared, fail, not_cancelled, request, require};
use crate::backend::{
    Capability, LookupCursor, LookupField, LookupPage, LookupRequest, MatchMode, Order, Refusal,
    StorageBackend,
};
use writfold_domain::citation::Coverage;

fn find<B: StorageBackend>(
    backend: &B,
    check: &'static str,
    pin: &B::Pin,
    lookup: &LookupRequest,
    cursor: Option<&LookupCursor<B::Pin>>,
) -> Result<LookupPage<B::Pin>, super::ConformanceFailure> {
    backend
        .lookup(pin, lookup, cursor, &not_cancelled())
        .map_err(|error| fail(check, error.to_string()))
}

fn keys<P>(page: &LookupPage<P>) -> Vec<&str> {
    page.hits().iter().map(|hit| hit.key().as_str()).collect()
}

fn complete<P>(page: &LookupPage<P>, total: u128) -> bool {
    page.coverage().complete()
        && page.coverage()
            == Coverage::Measured {
                examined: total as usize,
                total,
            }
        && page.next().is_none()
}

fn lexical_request(
    check: &'static str,
    field: LookupField,
    mode: MatchMode,
    needle: &str,
    limit: usize,
) -> Result<LookupRequest, super::ConformanceFailure> {
    request(
        check,
        LookupRequest::new(field, mode, Order::Key, needle.to_owned(), limit),
    )
}

/// Exact match returns every hit in key order with complete measured coverage.
pub(super) fn exact<B: StorageBackend>(backend: &mut B, f: &ConformanceFixture<B::Pin>) -> Outcome {
    const CHECK: &str = "exact-lookup";
    let lookup = lexical_request(CHECK, LookupField::Kind, MatchMode::Exact, "service", 10)?;
    let page = find(backend, CHECK, &f.before, &lookup, None)?;
    require(
        CHECK,
        page.pin() == &f.before,
        "page pin differs from the request",
    )?;
    require(
        CHECK,
        keys(&page) == ["svc:a", "svc:b"],
        "wrong hits or order",
    )?;
    require(
        CHECK,
        complete(&page, 4),
        "coverage not complete over 4 records",
    )?;
    let page = find(backend, CHECK, &f.after, &lookup, None)?;
    require(
        CHECK,
        keys(&page) == ["svc:a", "svc:c"],
        "after pin hits wrong",
    )?;
    let lookup = lexical_request(CHECK, LookupField::Key, MatchMode::Exact, "doc:b", 10)?;
    let page = find(backend, CHECK, &f.before, &lookup, None)?;
    require(CHECK, keys(&page) == ["doc:b"], "exact key lookup wrong")
}

/// Zero hits are reported only with complete coverage; absence is measured.
pub(super) fn zero_results<B: StorageBackend>(
    backend: &mut B,
    f: &ConformanceFixture<B::Pin>,
) -> Outcome {
    const CHECK: &str = "zero-results-complete";
    let lookup = lexical_request(CHECK, LookupField::Kind, MatchMode::Exact, "absent", 10)?;
    let page = find(backend, CHECK, &f.before, &lookup, None)?;
    require(CHECK, page.hits().is_empty(), "hits for an absent kind")?;
    require(
        CHECK,
        complete(&page, 4),
        "zero results without complete coverage",
    )
}

/// A bounded page reports incomplete coverage and a cursor; following cursors
/// reproduces the unpaged result exactly and ends with complete coverage.
pub(super) fn paging<B: StorageBackend>(
    backend: &mut B,
    f: &ConformanceFixture<B::Pin>,
) -> Outcome {
    const CHECK: &str = "paging-and-coverage";
    let lookup = lexical_request(CHECK, LookupField::Content, MatchMode::Substring, "e", 1)?;
    let whole = lexical_request(CHECK, LookupField::Content, MatchMode::Substring, "e", 100)?;
    let expected: Vec<String> = keys(&find(backend, CHECK, &f.before, &whole, None)?)
        .into_iter()
        .map(str::to_owned)
        .collect();
    require(CHECK, expected.len() == 4, "unpaged substring lookup wrong")?;
    let mut collected = Vec::new();
    let mut cursor = None;
    for _ in 0..=expected.len() {
        let page = find(backend, CHECK, &f.before, &lookup, cursor.as_ref())?;
        require(CHECK, page.hits().len() <= 1, "page exceeds its limit")?;
        collected.extend(keys(&page).into_iter().map(str::to_owned));
        cursor = page.next().cloned();
        if cursor.is_none() {
            require(
                CHECK,
                complete(&page, 4),
                "final page coverage not complete",
            )?;
            return require(
                CHECK,
                collected == expected,
                "paged hits differ from unpaged",
            );
        }
        require(
            CHECK,
            !page.coverage().complete(),
            "cursor issued with complete coverage",
        )?;
    }
    Err(fail(CHECK, "paging did not terminate"))
}

/// A cursor resumes only the exact pin and request it was issued for.
pub(super) fn cursor_binding<B: StorageBackend>(
    backend: &mut B,
    f: &ConformanceFixture<B::Pin>,
) -> Outcome {
    const CHECK: &str = "cursor-binding";
    let lookup = lexical_request(CHECK, LookupField::Kind, MatchMode::Exact, "document", 1)?;
    let page = find(backend, CHECK, &f.before, &lookup, None)?;
    let cursor = page
        .next()
        .ok_or_else(|| fail(CHECK, "no cursor for a bounded page"))?;
    let cancel = not_cancelled();
    let other_pin = backend.lookup(&f.after, &lookup, Some(cursor), &cancel);
    require(
        CHECK,
        other_pin == Err(Refusal::CursorMismatch),
        "cursor accepted at another pin",
    )?;
    let other = lexical_request(CHECK, LookupField::Kind, MatchMode::Exact, "service", 1)?;
    let other_request = backend.lookup(&f.before, &other, Some(cursor), &cancel);
    require(
        CHECK,
        other_request == Err(Refusal::CursorMismatch),
        "cursor accepted for another request",
    )
}

/// Prefix and substring are byte-wise and case-sensitive, including Unicode.
pub(super) fn lexical<B: StorageBackend>(
    backend: &mut B,
    f: &ConformanceFixture<B::Pin>,
) -> Outcome {
    const CHECK: &str = "lexical-lookup";
    let cases: [(LookupField, MatchMode, &str, &[&str]); 6] = [
        (
            LookupField::Key,
            MatchMode::Prefix,
            "svc:",
            &["svc:a", "svc:b"],
        ),
        (
            LookupField::Content,
            MatchMode::Substring,
            "service",
            &["doc:b", "svc:a"],
        ),
        (LookupField::Content, MatchMode::Substring, "SERVICE", &[]),
        (
            LookupField::Content,
            MatchMode::Substring,
            "naïve",
            &["doc:a"],
        ),
        (
            LookupField::Content,
            MatchMode::Exact,
            "service A",
            &["svc:a"],
        ),
        (
            LookupField::Content,
            MatchMode::Substring,
            "DROP TABLE",
            &["svc:b"],
        ),
    ];
    for (field, mode, needle, expected) in cases {
        let lookup = lexical_request(CHECK, field, mode, needle, 10)?;
        let page = find(backend, CHECK, &f.before, &lookup, None)?;
        require(CHECK, keys(&page) == expected, needle)?;
        require(CHECK, complete(&page, 4), "lexical coverage not complete")?;
    }
    Ok(())
}

/// Undeclared fuzzy, semantic and ranked lookups refuse by type, never emulated.
pub(super) fn unsupported<B: StorageBackend>(
    backend: &mut B,
    f: &ConformanceFixture<B::Pin>,
) -> Outcome {
    const CHECK: &str = "unsupported-refused";
    let cases = [
        (MatchMode::Fuzzy, Order::Key, Capability::Fuzzy),
        (MatchMode::Semantic, Order::Key, Capability::Semantic),
        (MatchMode::Substring, Order::Ranked, Capability::Ranking),
    ];
    for (mode, order, capability) in cases {
        if declared(backend, capability) {
            continue;
        }
        let lookup = request(
            CHECK,
            LookupRequest::new(LookupField::Content, mode, order, "service".to_owned(), 10),
        )?;
        let result = backend.lookup(&f.before, &lookup, None, &not_cancelled());
        require(
            CHECK,
            result == Err(Refusal::Unsupported(capability)),
            "not refused as Unsupported",
        )?;
    }
    Ok(())
}
