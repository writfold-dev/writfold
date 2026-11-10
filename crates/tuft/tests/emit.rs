//! Pure `emit` behaviour: budget stops at a row boundary with an explicit
//! omission count and cursor, cursors bind to the view, and a budget too
//! small for one row says so instead of printing nothing silently. The
//! budget is counted over compact rows (`src/f0.rs: 0123…+6B`, 6 tokens).
use tuft::emit::{CursorError, Format, Run, emit};
use tuft::view::{Evidence, Row, SearchReport, project};
use writfold_domain::citation::Coverage;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn plan(budget: u64) -> Result<tuft::Plan> {
    let source = format!(
        "@h := find text \"x\" at \"{}\";\n@e := retain @h;\n@v := project @e [path, excerpt(max_chars: 4)];\nemit @v budget_tokens {budget};\n",
        "a".repeat(40)
    );
    tuft::compile(source.as_bytes()).map_err(|d| format!("{d:?}").into())
}

fn rows() -> Vec<Row> {
    (0..5u64)
        .map(|i| Row {
            path: format!("src/f{i}.rs"),
            range: (10, 20),
            line: 2,
            evidence: Some(Evidence {
                binding: format!("{i:064}"),
                bytes: b"0123456789".to_vec(),
            }),
        })
        .collect()
}

fn render(budget: u64, cursor: Option<&str>) -> Result<std::result::Result<String, CursorError>> {
    render_as(budget, cursor, Format::Verbose)
}

fn render_as(
    budget: u64,
    cursor: Option<&str>,
    format: Format,
) -> Result<std::result::Result<String, CursorError>> {
    let plan = plan(budget)?;
    let projected = project(rows(), &plan.view);
    let run = Run {
        plan: &plan,
        search: SearchReport {
            coverage: Coverage::Measured {
                examined: 5,
                total: 5,
            },
            stop: None,
            hits: 5,
        },
        retained: Some(5),
        projected: &projected,
    };
    Ok(emit(run, cursor, format).map(|e| e.printed()))
}

fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.split([' ', '\n'])
        .find_map(|w| w.strip_prefix(key)?.strip_prefix('='))
}

#[test]
fn budget_stops_at_a_row_boundary_and_the_cursor_resumes() -> Result {
    let first = render(12, None)??;
    assert!(first.contains("\nsrc/f0.rs 10..14:\"0123\"…+6B"), "{first}");
    assert_eq!(field(&first, "shown"), Some("2"));
    assert_eq!(field(&first, "omitted"), Some("3"));
    let cursor = field(&first, "cursor").ok_or("cursor")?;
    let second = render(12, Some(cursor))??;
    assert_eq!(field(&second, "from"), Some("2"));
    assert!(second.contains("\nsrc/f2.rs "), "{second}");
    assert_eq!(render(13, Some(cursor))?, Err(CursorError::Mismatch));
    assert_eq!(render(12, Some("zz"))?, Err(CursorError::Malformed));
    Ok(())
}

#[test]
fn a_budget_below_one_row_is_reported_not_silent() -> Result {
    let text = render(5, None)??;
    assert_eq!(field(&text, "shown"), Some("0"));
    assert_eq!(field(&text, "omitted"), Some("5"));
    assert!(text.contains("notice next row needs 6 tokens"), "{text}");
    let compact = render_as(5, None, Format::Compact)??;
    assert!(
        compact
            .starts_with("# coverage=complete files=5/5 hits=5 shown=0 omitted=5 needs=6 cursor="),
        "{compact}"
    );
    Ok(())
}
