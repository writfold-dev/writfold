//! `find`'s name tiers: `normalized`, `prefix` and `heading` between
//! `exact` and `text`, each record once under its first tier, and the
//! tier in every row's `matched_on`.
mod support;

use serde_json::Value;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::verbs::find::Filter;

fn find(w: &World, text: &str) -> TestResult<(Vec<Value>, u8)> {
    let mut request = w.request(Verb::Find {
        text: text.into(),
        filter: Filter::default(),
        signatures: false,
    });
    request.json = true;
    let output = writfold_kb::run(&request)?;
    let body: Value = serde_json::from_str(&output.text)?;
    let rows = body["rows"].as_array().ok_or("rows")?.clone();
    Ok((rows, output.exit))
}

fn first(rows: &[Value]) -> TestResult<(String, String)> {
    let row = rows.first().ok_or("no row")?;
    let id = row["id"].as_str().ok_or("id")?.to_owned();
    let tier = row["matched_on"].as_str().ok_or("matched_on")?.to_owned();
    Ok((id, tier))
}

fn tiers(rows: &[Value]) -> Vec<String> {
    rows.iter()
        .filter_map(|r| r["matched_on"].as_str().map(str::to_owned))
        .collect()
}

#[test]
fn a_phrase_for_an_id_is_normalized() -> TestResult {
    let w = world(&[])?;
    for phrase in ["harbor api", "harbor_api", "HAPI"] {
        let (rows, exit) = find(&w, phrase)?;
        assert_eq!(exit, 0);
        assert_eq!(
            first(&rows)?,
            ("harbor-api".into(), "normalized".into()),
            "{phrase}"
        );
        let label = rows[0]["more"][0]["text"].as_str().ok_or("label")?;
        assert!(label.starts_with("normalized "), "{label}");
    }
    let (rows, _) = find(&w, "restart harbor")?;
    assert_eq!(first(&rows)?.1, "normalized");
    assert!(
        first(&rows)?.0.ends_with("ops/restart-harbor"),
        "{:?}",
        first(&rows)?
    );
    Ok(())
}

#[test]
fn part_of_a_name_matches_by_prefix_in_order() -> TestResult {
    let w = world(&[])?;
    let (rows, _) = find(&w, "quay work")?;
    assert_eq!(first(&rows)?, ("quay-worker".into(), "prefix".into()));
    assert_eq!(rows[0]["more"][0]["text"], "prefix quay-worker");
    let (rows, _) = find(&w, "work quay")?;
    assert!(!tiers(&rows).contains(&"prefix".to_owned()), "{rows:?}");
    // One token needs four characters.
    let (rows, _) = find(&w, "quay")?;
    assert!(tiers(&rows).contains(&"prefix".to_owned()), "{rows:?}");
    let (rows, _) = find(&w, "qua")?;
    assert!(!tiers(&rows).contains(&"prefix".to_owned()), "{rows:?}");
    Ok(())
}

#[test]
fn a_section_name_cites_its_heading() -> TestResult {
    let w = world(&[])?;
    let (rows, _) = find(&w, "failure modes")?;
    let row = rows
        .iter()
        .find(|r| r["matched_on"] == "heading")
        .ok_or("no heading row")?;
    let more = &row["more"][0];
    assert_eq!(more["text"], "heading Failure modes");
    assert_eq!(more["path"], "inventory/dossiers/harbor-api.md");
    assert_eq!(more["line"], 7);
    assert_eq!(rows[0]["matched_on"], "heading", "{rows:?}");
    Ok(())
}

#[test]
fn an_exact_id_is_shown_once_and_every_row_names_its_tier() -> TestResult {
    let w = world(&[])?;
    let (rows, _) = find(&w, "harbor-api")?;
    let ids: Vec<&str> = rows.iter().filter_map(|r| r["id"].as_str()).collect();
    assert_eq!(ids.iter().filter(|i| **i == "harbor-api").count(), 1);
    assert_eq!(first(&rows)?, ("harbor-api".into(), "exact".into()));
    let known = [
        "signature",
        "exact",
        "normalized",
        "prefix",
        "heading",
        "text",
        "substring",
        "typo",
        "text-any",
    ];
    for phrase in [
        "harbor-api",
        "batch ledger",
        "flush",
        "harbr-api",
        "dger sto",
    ] {
        let (rows, _) = find(&w, phrase)?;
        for tier in tiers(&rows) {
            assert!(known.contains(&tier.as_str()), "{tier}");
        }
        assert_eq!(
            tiers(&rows).len(),
            rows.len(),
            "{phrase}: a row without matched_on"
        );
    }
    Ok(())
}

#[test]
fn an_empty_answer_suggests_names_and_stays_unknown() -> TestResult {
    let w = world(&[])?;
    let (rows, exit) = find(&w, "batchng")?;
    assert_eq!(exit, 2);
    assert!(!rows.is_empty() && rows.len() <= 3, "{rows:?}");
    for row in &rows {
        assert_eq!(row["status"], "suggested");
        assert_eq!(row["matched_on"], "suggestion");
        let flags = row["flags"].as_array().ok_or("flags")?;
        assert!(flags.contains(&serde_json::json!("suggestion")), "{row}");
    }
    let mut request = w.request(Verb::Find {
        text: "batchng".into(),
        filter: Filter::default(),
        signatures: false,
    });
    request.json = true;
    let body: Value = serde_json::from_str(&writfold_kb::run(&request)?.text)?;
    assert_eq!(body["trailer"]["reason"], "no-record");
    // Too far from every name: nothing is suggested.
    let (rows, exit) = find(&w, "zeppelin mooring")?;
    assert_eq!((rows.len(), exit), (0, 2));
    // A suggestion never counts as a hit in an evaluation.
    let file = w.base.join("q.jsonl");
    let first = rows_first_id(&w, "batchng")?;
    std::fs::write(
        &file,
        format!("{{\"query\": \"batchng\", \"expect\": [\"{first}\"]}}\n"),
    )?;
    let mut request = w.request(Verb::FindEval { file: Some(file) });
    request.json = true;
    let output = writfold_kb::run(&request)?;
    assert_eq!(output.exit, 2, "{}", output.text);
    Ok(())
}

fn rows_first_id(w: &World, text: &str) -> TestResult<String> {
    let (rows, _) = find(w, text)?;
    Ok(rows
        .first()
        .and_then(|r| r["id"].as_str())
        .ok_or("no row")?
        .to_owned())
}
