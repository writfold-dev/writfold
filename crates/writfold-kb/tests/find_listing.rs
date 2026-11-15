//! `find --kind K [--project P]` with no text: every record of the kinds,
//! in ID order, once each, paged by the budget; and the JSON-only `edge`
//! and `title` keys of `impact` rows. Text goldens of `impact` stay as
//! they were (`graph.rs`), so the text form is unchanged.
mod support;

use serde_json::Value;
use std::path::Path;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::verbs::find::Filter;
use writfold_kb::verbs::impact::Direction;

fn listing(kinds: &[&str], project: Option<&str>) -> Verb {
    Verb::Find {
        text: String::new(),
        filter: Filter {
            kinds: kinds.iter().map(|k| (*k).to_owned()).collect(),
            project: project.map(str::to_owned),
        },
        signatures: false,
    }
}

fn json(w: &World, verb: Verb) -> TestResult<Value> {
    let mut request = w.request(verb);
    request.json = true;
    request.budget = writfold_kb::MAX_BUDGET;
    Ok(serde_json::from_str(&writfold_kb::run(&request)?.text)?)
}

fn rows(body: &Value) -> Vec<Value> {
    body["rows"].as_array().cloned().unwrap_or_default()
}

fn ids(body: &Value) -> Vec<String> {
    rows(body)
        .iter()
        .map(|r| r["id"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn golden(w: &World, name: &str, verb: Verb) -> TestResult<String> {
    let (text, exit) = w.run(&w.request(verb))?;
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let actual = format!("{text}# exit={exit}\n");
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(&file, &actual)?;
    } else {
        assert_eq!(std::fs::read_to_string(&file)?, actual, "golden {name}");
    }
    Ok(text)
}

#[test]
fn a_kind_listing_is_every_record_of_the_kind_in_id_order() -> TestResult {
    let w = world(&[])?;
    golden(&w, "find-kind-decision.txt", listing(&["decision"], None))?;
    let body = json(&w, listing(&["decision"], None))?;
    let listed = ids(&body);
    let mut sorted = listed.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(listed, sorted, "ID order, once each");
    assert_eq!(listed.len(), 4, "{listed:?}");
    for row in rows(&body) {
        assert_eq!(row["kind"], "decision", "{row}");
        let more = row["more"].as_array().ok_or("more")?;
        assert!(
            more.iter()
                .any(|m| m["label"] == "match" && m["text"] == "listed"),
            "{row}"
        );
        // Titles stay in `text`, as every other `find` row prints them.
        assert_ne!(row["text"], row["id"], "{row}");
    }
    let two = ids(&json(&w, listing(&["investigation", "ticket"], None))?);
    let mut sorted = two.clone();
    sorted.sort();
    assert_eq!(two, sorted);
    assert!(two.contains(&"HBR-101".to_owned()), "{two:?}");
    assert!(two.iter().any(|id| id.starts_with("inv:")), "{two:?}");
    Ok(())
}

#[test]
fn a_kind_listing_narrows_by_project_and_says_when_empty() -> TestResult {
    let w = world(&[])?;
    let all = ids(&json(&w, listing(&["decision", "ticket"], None))?);
    let harbor = ids(&json(&w, listing(&["decision", "ticket"], Some("harbor")))?);
    assert!(!harbor.is_empty() && harbor.len() < all.len(), "{harbor:?}");
    assert!(harbor.iter().all(|id| all.contains(id)));
    let lantern = json(&w, listing(&["decision"], Some("lantern")))?;
    assert!(rows(&lantern).is_empty(), "{lantern}");
    let none = json(&w, listing(&["no-such-kind"], None))?;
    assert!(rows(&none).is_empty());
    assert_eq!(none["trailer"]["reason"], "no-record");
    Ok(())
}

#[test]
fn a_kind_listing_pages_by_the_budget() -> TestResult {
    let w = world(&[])?;
    let verb = listing(&["decision", "ticket", "asset"], None);
    let mut whole = w.request(verb.clone());
    whole.budget = writfold_kb::MAX_BUDGET;
    let expected: Vec<String> = w.run(&whole)?.0.lines().map(str::to_owned).collect();
    let expected: Vec<_> = expected
        .into_iter()
        .filter(|l| !l.starts_with('#'))
        .collect();
    let mut request = w.request(verb);
    request.budget = 60;
    let mut joined = Vec::new();
    let mut pages = 0;
    loop {
        let text = w.run(&request)?.0;
        pages += 1;
        joined.extend(
            text.lines()
                .filter(|l| !l.starts_with('#'))
                .map(str::to_owned),
        );
        let trailer = text.lines().last().unwrap_or_default().to_owned();
        let cursor = trailer.split(' ').find_map(|p| p.strip_prefix("cursor="));
        match cursor {
            Some(c) if c != "none" && pages < 100 => request.cursor = Some(c.to_owned()),
            _ => break,
        }
    }
    assert!(pages > 1, "the budget must page this listing");
    assert_eq!(joined, expected);
    Ok(())
}

fn impact(target: &str, direction: Direction) -> Verb {
    Verb::Impact {
        target: target.into(),
        depth: 2,
        direction,
        inferred: false,
    }
}

#[test]
fn impact_rows_carry_their_edge_and_title_in_json_only() -> TestResult {
    let w = world(&[])?;
    let body = json(&w, impact("harbor-db", Direction::In))?;
    let rows = rows(&body);
    assert!(rows.len() > 3, "{body}");
    let mut titled = 0;
    for row in &rows {
        let edge = row.get("edge").ok_or("edge")?;
        let keys: Vec<_> = edge.as_object().ok_or("object")?.keys().cloned().collect();
        assert_eq!(keys, ["depth", "from", "origin", "rel", "to"], "{row}");
        let id = row["id"].as_str().unwrap_or_default();
        assert!(edge["from"] == id || edge["to"] == id, "{row}");
        let text = format!(
            "depth={} {} {} {} ({})",
            edge["depth"],
            edge["from"].as_str().unwrap_or_default(),
            edge["rel"].as_str().unwrap_or_default(),
            edge["to"].as_str().unwrap_or_default(),
            edge["origin"].as_str().unwrap_or_default()
        );
        assert_eq!(row["text"], text.as_str(), "{row}");
        if let Some(title) = row.get("title") {
            titled += 1;
            let status = json(&w, Verb::Status { target: id.into() })?;
            assert_eq!(status["rows"][0]["text"], *title, "{row}");
        }
    }
    assert!(titled > 0, "some reached record has a title");
    let ticket = rows
        .iter()
        .find(|r| r["id"] == "HBR-101")
        .ok_or("HBR-101")?;
    assert_eq!(ticket["edge"]["to"], "harbor-db");
    assert_eq!(ticket["edge"]["depth"], 1);
    let other = json(
        &w,
        Verb::Status {
            target: "harbor-db".into(),
        },
    )?;
    for row in other["rows"].as_array().ok_or("rows")? {
        assert!(
            row.get("edge").is_none() && row.get("title").is_none(),
            "{row}"
        );
    }
    Ok(())
}

#[test]
fn a_code_node_has_an_edge_and_no_title() -> TestResult {
    let w = world(&[])?;
    let body = json(
        &w,
        impact(
            "proposal/decision/2026-03-02-batch-ledger-writes",
            Direction::Out,
        ),
    )?;
    let code: Vec<_> = rows(&body)
        .into_iter()
        .filter(|r| r["kind"] == "code")
        .collect();
    assert!(!code.is_empty(), "{body}");
    for row in code {
        assert!(row.get("title").is_none(), "{row}");
        assert_eq!(row["edge"]["to"], row["id"], "{row}");
        assert_eq!(row["edge"]["origin"], "cite", "{row}");
    }
    Ok(())
}
