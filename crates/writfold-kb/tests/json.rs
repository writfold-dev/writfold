//! Pins the `--json` envelope documented in `docs/guide/json.md`: the
//! top-level, row, `more` and trailer key sets, and the version. A key
//! added, removed or renamed fails here, and every key must be named on
//! the page, so a change to the shape must touch the page too.
mod support;

use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;
use support::{TestResult, world};
use writfold_kb::Verb;

const TOP: [&str; 5] = ["writfold_kb", "verb", "rows", "notes", "trailer"];
const ROW: [&str; 9] = [
    "kind", "id", "status", "text", "path", "line", "flags", "more", "section",
];
const MORE: [&str; 6] = ["label", "text", "path", "line", "flags", "status"];
const TRAILER: [&str; 15] = [
    "coverage",
    "records_examined",
    "records_total",
    "shown",
    "omitted",
    "omitted_by",
    "answer",
    "reason",
    "counts",
    "unreadable",
    "needs",
    "kb",
    "index",
    "cursor",
    "q",
];

fn keys(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

#[test]
fn the_json_envelope_has_the_documented_keys() -> TestResult {
    let w = world(&[])?;
    let verbs = [
        Verb::Why {
            text: "batch ledger".into(),
            sections: Vec::new(),
        },
        Verb::Stale {
            scope: None,
            offline: false,
            cached: false,
        },
    ];
    let mut with_more = 0;
    for verb in verbs {
        let mut request = w.request(verb);
        request.json = true;
        let output = writfold_kb::run(&request)?;
        assert_eq!(output.text.lines().count(), 1, "{}", output.text);
        let body: Value = serde_json::from_str(&output.text)?;
        assert_eq!(keys(&body), set(&TOP));
        assert_eq!(body["writfold_kb"], 1);
        assert_eq!(keys(&body["trailer"]), set(&TRAILER));
        let rows = body["rows"].as_array().ok_or("rows")?;
        assert!(!rows.is_empty());
        for row in rows {
            assert_eq!(keys(row), set(&ROW), "{row}");
            for more in row["more"].as_array().ok_or("more")? {
                with_more += 1;
                let required = set(&MORE[..5]);
                assert!(required.is_subset(&keys(more)), "{more}");
                assert!(keys(more).is_subset(&set(&MORE)), "{more}");
                if let Some(status) = more.get("status") {
                    assert!(status.is_string(), "{more}");
                }
            }
        }
    }
    assert!(with_more > 0, "no row carried a more line");
    Ok(())
}

#[test]
fn the_guide_page_names_every_key() -> TestResult {
    let page = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide/json.md");
    let text = std::fs::read_to_string(page)?;
    for key in TOP.iter().chain(&ROW).chain(&MORE).chain(&TRAILER) {
        assert!(
            text.contains(&format!("`{key}`")),
            "json.md does not name {key}"
        );
    }
    Ok(())
}
