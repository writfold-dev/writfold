//! Claims inside text (#189): `min_citations` keeps `claims.cited-fresh`
//! from passing a draft with too few tokens, and `claims.accounted` needs
//! every prose block cited, tagged or excused; a bare block is named at its
//! line, a misspelt tag is a finding, an empty draft is unknown, and a mode
//! without the new keys behaves as before.
#[path = "delivery_round/helpers.rs"]
mod helpers;
mod support;

use helpers::{check, prepare};
use support::{NOW, TestResult, World};

const CITED: &str =
    "schema = \"writfold-delivery-mode/1\"\nmode = \"notes\"\nchecks = [\"claims.cited-fresh\"]\n";
const ACCOUNTED: &str = "schema = \"writfold-delivery-mode/1\"\nmode = \"brief\"\nchecks = [\"claims.accounted\"]\nstatus_tags = [\"inferred\", \"open\"]\n";

fn world(files: &[(&str, &str)]) -> TestResult<World> {
    helpers::kb(files)
}

/// The check's output and exit for a fresh candidate of `mode` holding `text`.
fn judged(w: &World, mode: &str, text: &str) -> TestResult<(String, u8)> {
    let id = prepare(w, mode, text, NOW + 60)?;
    check(w, &id, NOW + 120)
}

#[test]
fn a_draft_with_no_tokens_fails_only_under_a_minimum() -> TestResult {
    let text = "Batching is live in staging.\n";
    let w = world(&[("notes.toml", CITED)])?;
    let (out, _) = judged(&w, "notes", text)?;
    assert!(
        out.contains("claims.cited-fresh") && !out.contains("too-few"),
        "{out}"
    );
    let floored = format!("{CITED}min_citations = 1\n");
    let w = world(&[("notes.toml", &floored)])?;
    let (out, exit) = judged(&w, "notes", text)?;
    assert_ne!(exit, 0, "{out}");
    assert!(
        out.contains("too-few") && out.contains("min_citations = 1"),
        "{out}"
    );
    Ok(())
}

#[test]
fn a_bare_block_fails_naming_its_line_and_a_tag_accounts_for_it() -> TestResult {
    let w = world(&[("brief.toml", ACCOUNTED)])?;
    let (out, exit) = judged(
        &w,
        "brief",
        "# Brief\n\nBatching is live in staging.\n\nThe flush is next. [open]\n",
    )?;
    assert_ne!(exit, 0, "{out}");
    assert!(
        out.contains("line 3 bare") && out.contains("end it with one of [inferred], [open]"),
        "{out}"
    );
    let (out, _) = judged(
        &w,
        "brief",
        "Batching is live in staging. [inferred]\n\n- The flush is next. [open]\n",
    )?;
    assert!(
        !out.contains("[failed]") && !out.contains(" bare "),
        "{out}"
    );
    Ok(())
}

#[test]
fn a_misspelt_tag_is_a_finding_and_an_empty_draft_is_unknown() -> TestResult {
    let w = world(&[("brief.toml", ACCOUNTED)])?;
    let (out, exit) = judged(&w, "brief", "Batching is live in staging. [infered]\n")?;
    assert_ne!(exit, 0, "{out}");
    assert!(out.contains("did you mean [inferred]"), "{out}");
    let (out, _) = judged(&w, "brief", "# Only a heading\n\n```\ncode\n```\n")?;
    assert!(out.contains("0 blocks") && out.contains("unknown"), "{out}");
    Ok(())
}

#[test]
fn the_new_keys_need_their_checks() -> TestResult {
    let root = std::path::Path::new("/nonexistent");
    let bad = "schema = \"writfold-delivery-mode/1\"\nmode = \"x\"\nchecks = [\"paragraphs.bounded\"]\nmin_citations = 2\nstatus_tags = [\"open\"]\n";
    let Err(report) =
        writfold_kb::deliver::config::check_file(root, "x", ".writfold/deliveries/x.toml", bad)
    else {
        return Err("accepted".into());
    };
    let said: String = report
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert!(
        said.contains("min_citations") && said.contains("status_tags"),
        "{said}"
    );
    Ok(())
}
