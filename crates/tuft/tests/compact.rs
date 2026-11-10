//! Golden compact view: `path:line: text` rows that print raw unless raw
//! would be ambiguous, and one trailer line carrying coverage, counts, the
//! cursor and a short program digest.
use tuft::emit::{Format, Run, emit};
use tuft::view::{Evidence, Row, SearchReport, Stop, project};
use writfold_domain::citation::Coverage;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn program(retain: &str, budget: u64) -> String {
    format!(
        "@h := find text \"x\" at \"{}\";\n@e := retain @h{retain};\n@v := project @e [path, line, excerpt(max_chars: 12)];\nemit @v budget_tokens {budget};\n",
        "a".repeat(40)
    )
}

fn row(path: &str, bytes: &[u8]) -> Row {
    Row {
        path: path.to_owned(),
        range: (0, bytes.len() as u64),
        line: 1,
        evidence: Some(Evidence {
            binding: "0".repeat(64),
            bytes: bytes.to_vec(),
        }),
    }
}

fn render(source: &str, rows: Vec<Row>, stop: Option<Stop>) -> Result<String> {
    let plan = tuft::compile(source.as_bytes()).map_err(|d| format!("{d:?}"))?;
    let projected = project(rows, &plan.view);
    let run = Run {
        plan: &plan,
        search: SearchReport {
            coverage: Coverage::Measured {
                examined: 3,
                total: 4,
            },
            stop,
            hits: 6,
        },
        retained: Some(6),
        projected: &projected,
    };
    Ok(emit(run, None, Format::Compact)?.printed())
}

#[test]
fn line_rows_print_raw_unless_ambiguous() -> Result {
    let source = program(" around lines 0", 1_000);
    let rows = vec![
        row("a.rs", b"\tlet x = 1;\n"),
        row("b.rs", b"\"quoted\"\n"),
        row("c.rs", b"last"),
        row("d.rs", b"0123456789abcdef\n"),
        row("e:f.rs", b"x\n"),
        row("g.rs", b"a\rb\n"),
    ];
    let digest = tuft::digest_hex(source.as_bytes());
    let expected = format!(
        "a.rs:1: \tlet x = 1;\n\
b.rs:1: \"\\\"quoted\\\"\\n\"\n\
c.rs:1: \"last\"\n\
d.rs:1: 0123456789ab…+5B\n\
\"e:f.rs\":1: x\n\
g.rs:1: \"a\\rb\\n\"\n\
# coverage=partial files=3/4 hits=6 stopped=hit-limit shown=6 omitted=0 cursor=none program={}",
        &digest[..12]
    );
    assert_eq!(render(&source, rows, Some(Stop::HitLimit))?, expected);
    Ok(())
}

#[test]
fn match_rows_keep_every_byte_and_a_cut_page_names_its_cursor() -> Result {
    let source = program("", 4);
    let rows = vec![row("a.rs", b"x"), row("b.rs", b"x\n")];
    let out = render(&source, rows.clone(), None)?;
    let (shown, trailer) = out.split_once('\n').ok_or("no row")?;
    assert_eq!(shown, "a.rs:1: x");
    assert!(
        trailer.starts_with("# coverage=partial files=3/4 hits=6 shown=1 omitted=1 cursor="),
        "{trailer}"
    );
    let all = render(&program("", 100), rows, None)?;
    assert!(all.contains("\nb.rs:1: \"x\\n\"\n"), "{all}");
    Ok(())
}
