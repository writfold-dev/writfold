//! Index assertions concern emitted evidence, refusal and coverage contracts.
use super::expansion_tests::{program, run_at};
use super::tests::{Fixture, fixture};
use crate::trial::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tuft::emit::Format;

fn index(f: &Fixture, records: &str, commit: &str, digest: Option<&str>) -> Result<PathBuf> {
    let path = f.root.join("tags.jsonl");
    fs::write(&path, records)?;
    let actual = tuft::digest_hex(records.as_bytes());
    let meta = serde_json::json!({"commit": commit, "sha256": digest.unwrap_or(&actual)});
    fs::write(
        f.root.join("tags.jsonl.meta.json"),
        serde_json::to_vec(&meta)?,
    )?;
    Ok(path)
}

fn records() -> &'static str {
    "{\"_type\":\"ptag\",\"name\":\"JSON_OUTPUT_VERSION\",\"path\":\"1.0\"}\n\
{\"_type\":\"tag\",\"name\":\"a\",\"path\":\"src/lib.rs\",\"line\":1,\"roles\":\"def\"}\n\
{\"_type\":\"tag\",\"name\":\"a\",\"path\":\"src/lib.rs\",\"line\":2,\"roles\":[\"read\"]}\n"
}

#[test]
fn definitions_and_reference_roles_are_distinct_partial_results() -> Result<()> {
    let f = fixture()?;
    let path = index(&f, records(), &f.commit, None)?;
    for (matcher, expected_line) in [("def \"a\"", 1), ("refs \"a\"", 2)] {
        let source = program(&f, matcher, "src/*.rs");
        let result = run_at(&f, &f.repo, &source, false, Some(&path))?;
        assert_eq!(result.projected.rows.len(), 1);
        assert_eq!(result.projected.rows[0].line, expected_line);
        assert!(result.emitted.trailer.contains("coverage=partial"));
        assert!(result.emitted.trailer.contains(&f.commit));
        assert!(
            result
                .emitted
                .trailer
                .contains(&tuft::digest_hex(records().as_bytes()))
        );
        assert!(result.dirty_files.is_empty());
        let metadata = result.index.ok_or("index metadata missing")?;
        assert_eq!(metadata.commit, f.commit);
        assert_eq!(metadata.sha256, tuft::digest_hex(records().as_bytes()));
    }
    Ok(())
}

#[test]
fn empty_symbol_result_never_claims_complete_coverage() -> Result<()> {
    let f = fixture()?;
    let path = index(&f, records(), &f.commit, None)?;
    let source = program(&f, "refs \"missing\"", "docs/*.md");
    for format in [Format::Compact, Format::Verbose, Format::Json] {
        let result = super::execute(&super::Invocation {
            source: source.as_bytes(),
            name: "index.tuft",
            repo: &f.repo,
            state: &f.state,
            cursor: None,
            format,
            working_tree: false,
            index: Some(&path),
        })?;
        assert!(result.projected.rows.is_empty());
        let output = result.emitted.printed();
        assert!(output.contains("partial"));
        assert!(output.contains(&tuft::digest_hex(records().as_bytes())));
        assert!(output.contains(&f.commit));
    }
    Ok(())
}

fn refused(f: &Fixture, path: &Path) -> Result<()> {
    let source = program(f, "def \"a\"", "");
    let result = run_at(f, &f.repo, &source, false, Some(path));
    match result {
        Ok(_) => Err("invalid index accepted".into()),
        Err(_) => Ok(()),
    }
}

#[test]
fn stale_or_digest_wrong_index_is_refused() -> Result<()> {
    let f = fixture()?;
    let path = index(&f, records(), &"1".repeat(40), None)?;
    refused(&f, &path)?;
    let path = index(&f, records(), &f.commit, Some(&"0".repeat(64)))?;
    refused(&f, &path)?;
    Ok(())
}

#[test]
fn invalid_records_and_out_of_blob_ranges_are_refused() -> Result<()> {
    let f = fixture()?;
    for record in [
        r#"{"_type":"tag","name":"a","path":"src/lib.rs","line":99}"#,
        r#"{"_type":"tag","name":"a","path":"src/lib.rs","line":1,"end":99}"#,
        r#"{"_type":"tag","name":"a","path":"src/lib.rs","line":1,"roles":"def,call"}"#,
        r#"{"_type":"tag","name":"a","path":"src/lib.rs","pattern":"/^let/"}"#,
        r#"{"_type":"tag","name":"a","path":"../outside","line":1}"#,
        r#"{"_type":"unknown"}"#,
        r#"{"_type":"tag","name":"a","path":"src/lib.rs","line":1,"extras":"reference"}"#,
    ] {
        let path = index(&f, record, &f.commit, None)?;
        refused(&f, &path)?;
    }
    Ok(())
}

#[test]
fn symbol_query_requires_index_and_refuses_working_tree() -> Result<()> {
    let f = fixture()?;
    let source = program(&f, "def \"a\"", "");
    assert!(run_at(&f, &f.repo, &source, false, None).is_err());
    let path = index(&f, records(), &f.commit, None)?;
    assert!(run_at(&f, &f.repo, &source, true, Some(&path)).is_err());
    Ok(())
}
