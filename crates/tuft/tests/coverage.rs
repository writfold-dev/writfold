//! Index coverage never claims completeness, and cursors bind external inputs.
use tuft::emit::{Cursor, CursorError, Format, Run, emit};
use tuft::view::{IndexCoverage, Row, SearchReport, project};
use writfold_domain::citation::Coverage;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn plan(matcher: &str, budget: usize) -> Result<tuft::Plan> {
    let source = format!(
        "@h := find {matcher} \"f\" at \"{COMMIT}\"; @v := project @h [path]; emit @v budget_tokens {budget};"
    );
    tuft::compile(source.as_bytes()).map_err(|d| format!("{d:?}").into())
}

fn report() -> SearchReport {
    SearchReport {
        coverage: Coverage::Measured {
            examined: 2,
            total: 2,
        },
        stop: None,
        hits: 2,
        index: None,
        snapshot: None,
    }
}

fn index() -> IndexCoverage {
    IndexCoverage {
        commit: COMMIT.into(),
        sha256: "a".repeat(64),
    }
}

#[test]
fn index_provenance_is_partial_in_every_output_format() -> Result {
    let plan = plan("def", 100)?;
    let projected = project(Vec::new(), &plan.view);
    let mut search = report();
    search.hits = 0;
    search.index = Some(index());
    for format in [Format::Compact, Format::Verbose, Format::Json] {
        let emitted = emit(
            Run {
                plan: &plan,
                search: &search,
                retained: None,
                projected: &projected,
            },
            None,
            format,
        )?;
        assert!(emitted.trailer.contains("coverage=partial"));
        assert!(emitted.trailer.contains(&format!("index_commit={COMMIT}")));
        assert!(
            emitted
                .trailer
                .contains(&format!("index_sha256={}", "a".repeat(64)))
        );
        let text = emitted.printed();
        assert!(!text.contains("complete"));
        if format == Format::Json {
            assert!(text.contains("\"state\":\"partial\""));
            assert!(text.contains(&format!("\"index_commit\":\"{COMMIT}\"")));
            assert!(text.contains(&format!("\"index_sha256\":\"{}\"", "a".repeat(64))));
        }
    }
    Ok(())
}

fn cursor(search: &SearchReport, changed: &SearchReport) -> Result {
    let plan = plan("text", 1)?;
    let rows = ["a", "b"].map(|path| Row {
        path: path.into(),
        range: (0, 1),
        line: 1,
        evidence: None,
    });
    let projected = project(rows.into(), &plan.view);
    let run = Run {
        plan: &plan,
        search,
        retained: None,
        projected: &projected,
    };
    let first = emit(run, None, Format::Compact)?;
    let text = first
        .trailer
        .split_whitespace()
        .find_map(|part| part.strip_prefix("cursor="))
        .ok_or("missing cursor")?;
    let cursor = Cursor::parse(text)?;
    assert!(emit(run, Some(&cursor), Format::Compact).is_ok());
    let resumed = emit(
        Run {
            search: changed,
            ..run
        },
        Some(&cursor),
        Format::Compact,
    );
    assert_eq!(resumed, Err(CursorError::Tampered));
    Ok(())
}

#[test]
fn changed_working_bytes_refuse_even_when_projected_paths_are_unchanged() -> Result {
    let mut before = report();
    before.snapshot = Some("a".repeat(64));
    let mut after = before.clone();
    after.snapshot = Some("b".repeat(64));
    cursor(&before, &after)
}

#[test]
fn changed_index_refuses_even_when_projected_paths_are_unchanged() -> Result {
    let mut before = report();
    before.index = Some(index());
    let mut after = before.clone();
    after.index = Some(IndexCoverage {
        sha256: "b".repeat(64),
        ..index()
    });
    cursor(&before, &after)
}
