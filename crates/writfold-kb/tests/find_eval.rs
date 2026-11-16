//! `find --eval`: the fixture's `.writfold/queries.jsonl` (the `queries`
//! overlay on the basic KB, which keeps the basic KB's commit and so its
//! goldens unchanged) is answered
//! at rank 1 by the tier each phrase is there for; a miss, an unknown
//! ID, an empty file and a malformed line are not passes.
mod support;

use serde_json::Value;
use std::path::PathBuf;
use support::{TestResult, World, world};
use writfold_kb::Verb;

fn eval(w: &World, file: Option<PathBuf>) -> TestResult<(Value, u8)> {
    let mut request = w.request(Verb::FindEval { file });
    request.json = true;
    let output = writfold_kb::run(&request)?;
    Ok((serde_json::from_str(&output.text)?, output.exit))
}

fn counts(body: &Value) -> Value {
    body["trailer"]["counts"].clone()
}

#[test]
fn the_fixture_queries_all_hit_first() -> TestResult {
    let w = world(&["queries"])?;
    let (body, exit) = eval(&w, None)?;
    let rows = body["rows"].as_array().ok_or("rows")?;
    for row in rows {
        assert_eq!(row["status"], "hit@1", "{row}");
        assert_eq!(row["kind"], "query");
    }
    assert_eq!(exit, 0, "{body}");
    let c = counts(&body);
    assert_eq!(
        (
            c["queries"].as_u64(),
            c["hit1"].as_u64(),
            c["miss"].as_u64()
        ),
        (Some(8), Some(8), Some(0))
    );
    for (by, n) in [
        ("by-normalized", 4),
        ("by-prefix", 1),
        ("by-heading", 1),
        ("by-exact", 1),
        ("by-text", 1),
    ] {
        assert_eq!(c[by].as_u64(), Some(n), "{by}: {c}");
    }
    Ok(())
}

#[test]
fn misses_unknown_ids_and_empty_files_are_not_passes() -> TestResult {
    let w = world(&[])?;
    let file = w.base.join("q.jsonl");
    std::fs::write(
        &file,
        "{\"query\": \"zeppelin mooring\", \"expect\": [\"harbor-api\"]}\n{\"query\": \"harbor api\", \"expect\": [\"harbor-api\", \"no-such-thing\"]}\n",
    )?;
    let (body, exit) = eval(&w, Some(file.clone()))?;
    assert_eq!(exit, 2);
    let rows = body["rows"].as_array().ok_or("rows")?;
    assert_eq!(rows[0]["status"], "miss");
    assert_eq!(rows[0]["more"][0]["label"], "first");
    assert_eq!(rows[1]["status"], "miss");
    assert_eq!(rows[1]["flags"], serde_json::json!(["unknown-id"]));
    assert_eq!(counts(&body)["miss"], 2);
    // The second of several rows is a hit within five, and passes.
    std::fs::write(
        &file,
        "{\"query\": \"harbor api\", \"expect\": [\"dossier:harbor-api\"]}\n",
    )?;
    let (body, exit) = eval(&w, Some(file.clone()))?;
    assert_eq!(body["rows"][0]["status"], "hit@5", "{body}");
    assert_eq!((exit, counts(&body)["hit5"].as_u64()), (0, Some(1)));
    std::fs::write(&file, "\n")?;
    let (body, exit) = eval(&w, Some(file.clone()))?;
    assert_eq!(
        (exit, body["trailer"]["reason"].as_str()),
        (2, Some("no-queries"))
    );
    let (_, exit) = eval(&w, Some(w.base.join("absent.jsonl")))?;
    assert_eq!(exit, 2);
    for (bad, line) in [
        (
            "{\"query\": \"x\", \"expect\": [\"a\"], \"weight\": 2}\n",
            1,
        ),
        ("{\"query\": \"x\", \"expect\": []}\n", 1),
        ("{\"query\": \"x\", \"expect\": [\"a\"]}\nnot json\n", 2),
    ] {
        std::fs::write(&file, bad)?;
        let request = w.request(Verb::FindEval {
            file: Some(file.clone()),
        });
        let why = match writfold_kb::run(&request) {
            Ok(o) => format!("exit {}", o.exit),
            Err(e) => e.to_string(),
        };
        assert!(why.contains(&format!("q.jsonl:{line}:")), "{bad}: {why}");
    }
    Ok(())
}
