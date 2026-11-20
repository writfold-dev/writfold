//! #21: the word table stems English words (FTS5 `porter`), so a phrase
//! whose words occur only in other forms is a `text` hit, and every
//! `text` row cites the body line holding the word it matched.
mod support;

use serde_json::Value;
use support::{TestResult, world};
use writfold_kb::Verb;
use writfold_kb::verbs::find::Filter;

#[test]
fn other_word_forms_are_text_hits_with_a_cited_line() -> TestResult {
    let w = world(&[])?;
    let mut request = w.request(Verb::Find {
        text: "stalling flushes".into(),
        filter: Filter::default(),
        signatures: false,
    });
    request.json = true;
    request.budget = 100_000;
    let body: Value = serde_json::from_str(&writfold_kb::run(&request)?.text)?;
    let rows = body["rows"].as_array().ok_or("rows")?;
    let text: Vec<&Value> = rows.iter().filter(|r| r["matched_on"] == "text").collect();
    assert!(!text.is_empty(), "{body}");
    for row in text {
        let line = &row["more"][0];
        assert_eq!(line["label"], "match", "{row}");
        assert!(line["line"].as_u64().is_some_and(|n| n > 0), "{row}");
        assert!(line["path"].is_string(), "{row}");
        let shown = line["text"].as_str().ok_or("text")?.to_lowercase();
        assert!(shown.contains("stall") || shown.contains("flush"), "{row}");
    }
    Ok(())
}
