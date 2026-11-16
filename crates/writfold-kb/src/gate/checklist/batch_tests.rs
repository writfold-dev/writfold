//! Rows of `gate cover --from`: line numbers, and every fault refused.
use super::*;

fn batch() -> Batch {
    Batch {
        gate: "review".into(),
        from: Rows::Stdin,
        by: "agent".into(),
        run: Some("run-1".into()),
        repo: None,
    }
}

fn why(input: &str) -> String {
    match parse(&batch(), input) {
        Ok(rows) => format!("not refused: {} rows", rows.len()),
        Err(e) => e.to_string(),
    }
}

#[test]
fn rows_parse_with_their_line_numbers() -> Result<()> {
    let input = "{\"item\":\"errors\",\"outcome\":\"supported\",\"refs\":[\"gate:unit\"],\"note\":\"n\"}\n\n{\"item\":\"perf\",\"outcome\":\"not-applicable\",\"reason\":\"none\"}\n";
    let rows = parse(&batch(), input)?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 1);
    assert_eq!(rows[0].1.references, vec!["gate:unit".to_owned()]);
    assert_eq!(rows[0].1.note.as_deref(), Some("n"));
    assert_eq!(rows[1].0, 3);
    assert_eq!(rows[1].1.outcome, Outcome::NotApplicable);
    assert_eq!(rows[1].1.reason.as_deref(), Some("none"));
    assert!(
        rows.iter()
            .all(|(_, c)| c.gate == "review" && c.by == "agent")
    );
    Ok(())
}

#[test]
fn malformed_rows_are_refused_naming_the_line() {
    let ok = "{\"item\":\"errors\",\"outcome\":\"supported\",\"refs\":[\"gate:unit\"]}\n";
    for (bad, want) in [
        ("not json", "-:2:2: this line is not JSON"),
        ("[1]", "-:2:1: a cover row is a JSON object"),
        (
            "{\"outcome\":\"supported\"}",
            "-:2:1: a cover row needs `item`",
        ),
        (
            "{\"item\":\"x\",\"outcome\":\"met\"}",
            "-:2:23: outcome \"met\" is not",
        ),
        (
            "{\"item\":\"x\",\"outcome\":\"supported\",\"refs\":\"gate:unit\"}",
            "`refs` is not a list",
        ),
        (
            "{\"item\":\"x\",\"outcome\":\"supported\",\"refs\":[1]}",
            "not a string",
        ),
        (
            "{\"item\":\"x\",\"outcome\":\"supported\",\"by\":\"me\"}",
            "unknown key `by`",
        ),
        (
            "{\"item\":\"x\",\"outcome\":\"unsupported\",\"reason\":3}",
            "`reason` must be a string",
        ),
        (
            "{\"item\":\"errors\",\"outcome\":\"unsupported\",\"reason\":\"r\"}",
            "-:2:9: item errors is already covered on line 1",
        ),
    ] {
        let got = why(&format!("{ok}{bad}\n"));
        assert!(got.starts_with("refused: --from -:2:"), "{bad}: {got}");
        assert!(got.contains(want), "{bad}: {got}");
    }
    assert!(why("").contains("holds no rows"));
    assert!(why("\n  \n").contains("holds no rows"));
}

#[test]
fn input_reads_stdin_or_a_file_and_bounds_it() -> Result<()> {
    let dir = std::env::temp_dir().join(format!("writfold-batch-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("rows.jsonl"), "file rows\n")?;
    let file = Rows::File("rows.jsonl".into());
    assert_eq!(input(&file, &dir, std::io::empty())?, "file rows\n");
    assert_eq!(
        input(&Rows::Stdin, &dir, &b"piped rows\n"[..])?,
        "piped rows\n"
    );
    let big = vec![b'x'; (INPUT_LIMIT + 1) as usize];
    let e = input(&Rows::Stdin, &dir, &big[..])
        .err()
        .map(|e| e.to_string());
    assert!(e.is_some_and(|e| e.contains("larger than")));
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}
