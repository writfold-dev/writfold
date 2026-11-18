//! `find --each`: several queries in one process, each answered exactly
//! as `find` answers it alone, grouped under a `query` row.
mod support;

use serde_json::Value;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::verbs::find::Filter;

fn rows(w: &World, verb: Verb) -> TestResult<(Vec<Value>, Value, u8)> {
    let mut request = w.request(verb);
    request.json = true;
    request.budget = 100_000;
    let output = writfold_kb::run(&request)?;
    let body: Value = serde_json::from_str(&output.text)?;
    let rows = body["rows"].as_array().ok_or("rows")?.clone();
    Ok((rows, body["trailer"].clone(), output.exit))
}

fn one(w: &World, text: &str) -> TestResult<(Vec<Value>, u8)> {
    let (rows, _, exit) = rows(
        w,
        Verb::Find {
            text: text.into(),
            filter: Filter::default(),
            signatures: false,
        },
    )?;
    Ok((rows, exit))
}

fn each(w: &World, texts: &[&str]) -> TestResult<(Vec<Value>, Value, u8)> {
    rows(
        w,
        Verb::FindEach {
            texts: texts.iter().map(|t| (*t).to_owned()).collect(),
            filter: Filter::default(),
            signatures: false,
        },
    )
}

/// One group per query, in the order given; each group is the first five
/// of the lone `find`'s rows with the query added, flagged when there are
/// more.
#[test]
fn each_query_is_answered_as_find_answers_it() -> TestResult {
    let w = world(&[])?;
    let texts = ["harbor api", "restart harbor", "HAPI"];
    let (found, trailer, exit) = each(&w, &texts)?;
    assert_eq!(exit, 0);
    assert_eq!(trailer["counts"]["queries"], 3);
    assert_eq!(trailer["counts"]["miss"], 0);
    let mut at = 0;
    for (index, text) in texts.iter().enumerate() {
        let (all, alone_exit) = one(&w, text)?;
        assert_eq!(alone_exit, 0);
        let alone = &all[..all.len().min(5)];
        let head = &found[at];
        let flags = if all.len() > 5 { "[\"more\"]" } else { "[]" };
        assert_eq!(head["flags"].to_string(), flags, "{text}");
        assert_eq!(head["kind"], "query");
        assert_eq!(head["id"], (index + 1).to_string());
        assert_eq!(head["status"], "hit");
        assert_eq!(head["text"], *text);
        assert_eq!(head["query"], *text);
        assert_eq!(head["hits"], alone.len());
        for (offset, want) in alone.iter().enumerate() {
            let mut got = found[at + 1 + offset].clone();
            assert_eq!(got["query"], *text);
            got.as_object_mut().ok_or("row")?.remove("query");
            assert_eq!(&got, want, "{text} row {offset}");
        }
        at += 1 + alone.len();
    }
    assert_eq!(at, found.len(), "no rows beyond the groups");
    // The fixture has a broad query: the cap is what keeps the last
    // query's group on the page.
    assert!(one(&w, "harbor api")?.0.len() > 5);
    Ok(())
}

/// A query that finds nothing is a `none` row with zero hits; its near
/// names are flagged, and the answer exits 2 as a lone miss does.
#[test]
fn a_miss_is_counted_and_its_suggestions_are_flagged() -> TestResult {
    let w = world(&[])?;
    let (alone, alone_exit) = one(&w, "zzqx-nothing-like-this")?;
    assert_eq!(alone_exit, 2);
    let (found, trailer, exit) = each(&w, &["harbor api", "zzqx-nothing-like-this"])?;
    assert_eq!(exit, 2);
    assert_eq!(trailer["counts"]["queries"], 2);
    assert_eq!(trailer["counts"]["miss"], 1);
    let heads: Vec<&Value> = found.iter().filter(|r| r["kind"] == "query").collect();
    assert_eq!(heads.len(), 2);
    assert_eq!(heads[0]["status"], "hit");
    assert_eq!(heads[1]["status"], "none");
    assert_eq!(heads[1]["hits"], 0);
    let under: Vec<&Value> = found
        .iter()
        .filter(|r| r["kind"] != "query" && r["query"] == "zzqx-nothing-like-this")
        .collect();
    assert_eq!(under.len(), alone.len());
    for row in under {
        let flags = row["flags"].to_string();
        assert!(flags.contains("suggested"), "{row}");
    }
    Ok(())
}
