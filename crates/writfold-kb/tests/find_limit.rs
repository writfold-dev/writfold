//! `find --limit N`: the rows are the first N rows of the full answer
//! (`limit_prefix`), and a record's own ID stops after the `exact` tier,
//! which again gives the first rows of the full answer (`stop_is_prefix`).
mod support;

use serde_json::Value;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::verbs::find::Filter;

fn find(w: &World, text: &str, limit: Option<usize>) -> TestResult<(Vec<Value>, Value)> {
    let mut request = w.request(Verb::Find {
        text: text.into(),
        filter: Filter {
            limit,
            ..Filter::default()
        },
        signatures: false,
    });
    request.json = true;
    request.budget = 100_000;
    let output = writfold_kb::run(&request)?;
    let body: Value = serde_json::from_str(&output.text)?;
    let rows = body["rows"].as_array().ok_or("rows")?.clone();
    Ok((rows, body["notes"].clone()))
}

fn notes(notes: &Value) -> String {
    notes.to_string()
}

#[test]
fn limited_rows_are_the_first_rows() -> TestResult {
    let w = world(&[])?;
    // Phrases, not IDs (an ID stops after the exact tier, tested below).
    for text in [
        "harbor api",
        "restart harbor",
        "harbor billing",
        "staging sweep",
    ] {
        let (all, _) = find(&w, text, None)?;
        for n in [1, 2, 3, 5] {
            let (some, _) = find(&w, text, Some(n))?;
            assert_eq!(some, all[..all.len().min(n)], "{text} --limit {n}");
        }
    }
    Ok(())
}

#[test]
fn own_id_stops_after_the_exact_tier_only_under_a_limit() -> TestResult {
    let w = world(&[])?;
    let (broad, _) = find(&w, "harbor api", None)?;
    let id = broad
        .iter()
        .filter_map(|r| r["id"].as_str())
        .find(|id| id.contains("harbor"))
        .ok_or("a harbor record")?
        .to_owned();
    let (all, trailer) = find(&w, &id, None)?;
    assert!(
        all.len() > 1,
        "{id}: the full answer has more than the exact row"
    );
    assert_eq!(all[0]["matched_on"], "exact");
    assert!(
        !notes(&trailer).contains("exact ID"),
        "no stop without a limit"
    );
    let (stopped, trailer) = find(&w, &id, Some(5))?;
    assert_eq!(
        stopped,
        all[..1],
        "{id}: only the exact row, the first of the full answer"
    );
    assert!(notes(&trailer).contains("an exact ID: the other tiers were not searched"));
    // A phrase that is not an ID is not stopped.
    let (phrase, trailer) = find(&w, "harbor api", Some(5))?;
    assert_eq!(phrase, broad[..broad.len().min(5)]);
    assert!(!notes(&trailer).contains("exact ID"));
    Ok(())
}
