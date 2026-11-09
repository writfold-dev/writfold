//! `--json` output read through typed structures: excerpt fidelity against
//! the retained bytes, measured completeness, and the citation round trip
//! (retained bindings verify and a Weft report citing one passes `prepare`).
use super::tests::{Fixture, fixture, pipeline, tuft};
use crate::trial::retain::retained;
use crate::trial::{Result, Trial, prepare};
use serde::Deserialize;
use std::fs;
use tuft::emit::Format;

#[derive(Deserialize)]
struct Output {
    find: Find,
    rows: Vec<Row>,
}

#[derive(Deserialize)]
struct Find {
    coverage: Coverage,
    hits: usize,
}

#[derive(Deserialize)]
struct Coverage {
    state: String,
    examined_files: usize,
    total_files: usize,
}

#[derive(Deserialize)]
struct Row {
    path: String,
    binding: String,
    range: Option<(usize, usize)>,
    excerpt: Option<Excerpt>,
}

#[derive(Deserialize)]
struct Excerpt {
    range: (usize, usize),
    text: String,
    hidden_bytes: usize,
}

fn output(f: &Fixture, program: &str) -> Result<Output> {
    Ok(serde_json::from_str(&tuft(
        f,
        program,
        None,
        Format::Json,
    )?)?)
}

#[test]
fn every_excerpt_equals_the_retained_bytes_at_its_range() -> Result<()> {
    let f = fixture()?;
    let program = pipeline(&f, 100_000)
        .replace("retain @h;", "retain @h around lines 0;")
        .replace("excerpt(max_chars: 6)", "excerpt(max_chars: 12), range");
    let out = output(&f, &program)?;
    assert_eq!(out.rows.len(), 8);
    for row in &out.rows {
        let excerpt = row.excerpt.as_ref().ok_or("excerpt")?;
        let (start, end) = row.range.ok_or("range")?;
        let (lo, hi) = excerpt.range;
        let file = fs::read(f.repo.join(&row.path))?;
        let evidence = f
            .state
            .join("evidence")
            .join(format!("{}.bytes", row.binding));
        let retained = fs::read(evidence)?;
        assert_eq!(retained.as_slice(), &file[start..end], "{}", row.path);
        assert_eq!(excerpt.text.as_bytes(), &file[lo..hi], "{}", row.path);
        assert_eq!(excerpt.text.as_bytes(), &retained[lo - start..hi - start]);
        assert_eq!(hi + excerpt.hidden_bytes, end, "{}", row.path);
    }
    let notes = out.rows.first().ok_or("no rows")?;
    assert_eq!(notes.path, "docs/notes.md");
    let excerpt = notes.excerpt.as_ref().ok_or("excerpt")?;
    assert_eq!(
        (excerpt.text.as_str(), excerpt.hidden_bytes),
        ("let a = café", 2)
    );
    Ok(())
}

#[test]
fn zero_results_are_complete_in_json_too() -> Result<()> {
    let f = fixture()?;
    let program = format!(
        "@h := find text \"absent needle\" in \"src/\" at \"{}\";\n@e := retain @h;\n\
@v := project @e [path, binding];\nemit @v;\n",
        f.commit
    );
    let out = output(&f, &program)?;
    assert_eq!(out.find.coverage.state, "complete");
    assert_eq!(
        (
            out.find.coverage.examined_files,
            out.find.coverage.total_files
        ),
        (6, 6)
    );
    assert_eq!(out.find.hits, 0);
    assert!(out.rows.is_empty());
    Ok(())
}

#[test]
fn retained_bindings_verify_and_a_report_citing_them_prepares() -> Result<()> {
    let f = fixture()?;
    let out = output(&f, &pipeline(&f, 100_000))?;
    let trial = Trial::open(&f.state)?;
    for row in &out.rows {
        assert!(retained(&trial, &row.binding)?, "{}", row.binding);
    }
    let binding = &out.rows.first().ok_or("no rows")?.binding;
    let report = format!(
        "{{\"schema\":\"weft.report/1\",\"kind\":\"assessment\",\"scope\":\"local-symbolic\",\
\"execution\":\"complete\",\"writfold_grounded\":false,\"effect_authority\":false,\
\"review\":{{\"bindings\":[{{\"kind\":\"asset\",\"reference\":\"{binding}\"}}]}},\
\"checks\":[{{\"name\":\"c\",\"result\":\"entailed\"}}]}}"
    );
    let report_path = f.root.join("report.json");
    let recipe = f.root.join("r.spl");
    fs::write(&report_path, report)?;
    fs::write(
        &recipe,
        "edit \"src/lib.rs\" {\n  @n := one text \"run(1)\"\n  replace @n with \"run(2)\"\n}\n",
    )?;
    prepare(&f.state, &f.repo, "src/lib.rs", &recipe, Some(&report_path))?;
    Ok(())
}
