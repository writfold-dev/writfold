//! `gate cover <gate> --from FILE|-`: several cover rows of one checklist
//! gate from JSON lines, on one subject snapshot, all rows or none. Each
//! row is the row the single-item form would write.
mod checklist;
mod support;

use checklist::{RUN, STALL, cover, define, rows, run_unit, standing, status};
use std::path::Path;
use support::{TestResult, world};
use writfold_kb::Verb;
use writfold_kb::gate::GateVerb;
use writfold_kb::gate::checklist::{Batch, Outcome, Rows};

const THREE: &str = "{\"item\":\"errors\",\"outcome\":\"supported\",\"refs\":[\"gate:unit\"]}\n{\"item\":\"record\",\"outcome\":\"supported\",\"refs\":[\"⟦cite kind=record path=ktlo/2026/hbr-99-flush-stall.md state=done⟧\"]}\n\n{\"item\":\"perf\",\"outcome\":\"not-applicable\",\"reason\":\"no hot path is touched\"}\n";

/// Record the rows in `text` through `--from FILE`.
fn batch(w: &support::World, text: &str) -> writfold_kb::error::Result<writfold_kb::emit::Output> {
    let file = w.base.join("rows.jsonl");
    std::fs::write(&file, text)?;
    let verb = GateVerb::CoverBatch(Batch {
        gate: "review".into(),
        from: Rows::File(file),
        by: "agent".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    writfold_kb::run(&w.request(Verb::Gate(verb)))
}

fn refusal(result: writfold_kb::error::Result<writfold_kb::emit::Output>) -> (String, u8) {
    match result {
        Ok(out) => (format!("not refused: {}", out.text), out.exit),
        Err(e) => (e.to_string(), e.exit_code()),
    }
}

/// Every row file's bytes, keyed by gate and item (the name without its
/// time and random suffix).
fn contents(kb: &Path) -> TestResult<Vec<(String, String)>> {
    let mut out = Vec::new();
    for path in rows(kb)? {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let key = name.split('-').next().unwrap_or_default().to_owned();
        out.push((key, std::fs::read_to_string(&path)?));
    }
    out.sort();
    Ok(out)
}

#[test]
fn a_batch_of_three_writes_three_rows_and_covers_the_checklist() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    run_unit(&w)?;
    let out = batch(&w, THREE)?;
    assert_eq!(rows(&w.kb)?.len(), 3, "{}", out.text);
    assert_eq!(
        out.text.matches("# note: recorded runs/").count(),
        3,
        "{}",
        out.text
    );
    assert!(
        out.text
            .contains("gate review [attested] checklist 2/2 required covered"),
        "{}",
        out.text
    );
    let (text, exit) = status(&w)?;
    assert_eq!(
        (standing(&text, "review"), exit),
        ("attested".into(), 0),
        "{text}"
    );
    Ok(())
}

/// One refused row refuses the batch (exit 3) naming its line and item,
/// and nothing is written, though the other rows are sound.
#[test]
fn one_bad_row_writes_nothing() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    run_unit(&w)?;
    for (bad, want) in [
        (
            "{\"item\":\"perf\",\"outcome\":\"unsupported\",\"refs\":[\"gate:unit\"],\"reason\":\"r\"}",
            "--from line 2: item perf: gate cover … unsupported takes a reason, not references",
        ),
        (
            "{\"item\":\"speed\",\"outcome\":\"not-applicable\",\"reason\":\"r\"}",
            "--from line 2: item speed: gate review has no item speed",
        ),
        (
            "{\"item\":\"perf\",\"outcome\":\"supported\",\"refs\":[\"gate:missing\"]}",
            "--from line 2: item perf: reference \"gate:missing\" is not grounded",
        ),
        (
            "{\"item\":\"perf\",\"outcome\":\"not-applicable\"}",
            "--from line 2: item perf: gate cover … not-applicable needs --reason TEXT",
        ),
        (
            "{\"item\":\"errors\",\"outcome\":\"unsupported\",\"reason\":\"r\"}",
            "--from line 2: item errors is already covered on line 1",
        ),
    ] {
        let text = format!(
            "{{\"item\":\"errors\",\"outcome\":\"supported\",\"refs\":[\"gate:unit\"]}}\n{bad}\n"
        );
        let (why, exit) = refusal(batch(&w, &text));
        assert!(why.contains(want), "{bad}: {why}");
        assert_eq!(exit, 3, "{bad}: {why}");
        assert!(rows(&w.kb)?.is_empty(), "{bad}: a row was written");
    }
    let (why, exit) = refusal(batch(&w, "\n"));
    assert!(why.contains("holds no rows") && exit == 3, "{why}");
    Ok(())
}

/// A batch writes byte for byte the rows the single-item form writes at
/// the same moment, and the checklist reads the same.
#[test]
fn a_batch_equals_the_single_item_form() -> TestResult {
    let single = world(&[])?;
    define(&single.kb)?;
    run_unit(&single)?;
    cover(&single, "errors", Outcome::Supported, &["gate:unit"])?;
    cover(&single, "record", Outcome::Supported, &[STALL])?;
    cover(&single, "perf", Outcome::NotApplicable, &[])?;
    let batched = world(&[])?;
    define(&batched.kb)?;
    run_unit(&batched)?;
    batch(&batched, THREE)?;
    let (a, b) = (contents(&single.kb)?, contents(&batched.kb)?);
    assert_eq!(a.len(), 3);
    // Only the cited receipt's random name, and so the digest, differ.
    let names = regex::Regex::new(r"-(\d{8}T\d{6}Z)-[0-9a-f]{6}")?;
    let same = |text: &str| {
        names
            .replace_all(text, "-$1-XXXXXX")
            .lines()
            .filter(|l| !l.contains("\"digest\""))
            .collect::<Vec<_>>()
            .join("\n")
    };
    for ((ka, ta), (kb, tb)) in a.iter().zip(&b) {
        assert_eq!(ka, kb);
        assert_eq!(same(ta), same(tb), "{ka}");
    }
    let ((sa, ea), (sb, eb)) = (status(&single)?, status(&batched)?);
    assert_eq!((same(&sa), ea), (same(&sb), eb));
    // A second batch of one item adds that item's next row.
    batch(
        &batched,
        "{\"item\":\"perf\",\"outcome\":\"unsupported\",\"reason\":\"allocates per call\"}\n",
    )?;
    let after = contents(&batched.kb)?;
    assert_eq!(after.len(), 4);
    assert!(
        after
            .iter()
            .any(|(k, t)| k == "review.perf" && t.contains("\"seq\": 2")),
        "{after:?}"
    );
    Ok(())
}

/// Record a `timeout` observation for `review.perf`; its KB path.
fn looked(w: &support::World) -> TestResult<String> {
    use writfold_kb::write::{Write, observe::Observe};
    checklist::put(
        &w.kb.join(".writfold/sources/probe.toml"),
        "schema = \"writfold-source/1\"\nkind = \"probe\"\nfreshness = \"ttl\"\ndefault_ttl = \"1d\"\n",
    )?;
    let o = Observe {
        source: "probe".into(),
        locator: "p99 latency, checkout".into(),
        status: "timeout".into(),
        revision: None,
        item: Some("review.perf".into()),
        note: None,
        cite: None,
        run: Some(RUN.into()),
    };
    let out = writfold_kb::run(&w.request(Verb::Write(Write::Observe(o))))?;
    let path = out
        .text
        .lines()
        .find_map(|l| l.split("recorded ").nth(1))
        .and_then(|rest| rest.split(' ').next())
        .map(str::to_owned);
    Ok(path.ok_or_else(|| format!("no path in {}", out.text))?)
}

/// `unsupported` and `not-applicable` rows may cite observations of the
/// run (any status) beside their reason, to record that someone looked;
/// nothing else, and the item stands as it would without them.
#[test]
fn a_reasoned_row_may_cite_observations_only() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    run_unit(&w)?;
    let path = looked(&w)?;
    let name = path
        .rsplit('/')
        .next()
        .and_then(|n| n.strip_suffix(".json"))
        .unwrap_or_default()
        .to_owned();
    let mut verb = checklist::cover_verb(&w.code, "perf", Outcome::Unsupported, &[&path]);
    if let GateVerb::Cover(c) = &mut verb {
        c.reason = Some("the probe timed out".into());
    }
    writfold_kb::run(&w.request(Verb::Gate(verb)))?;
    let (text, _) = status(&w)?;
    assert_eq!(checklist::item(&text, "perf"), "unsupported", "{text}");
    let row = contents(&w.kb)?;
    assert!(row[0].1.contains("\"standing\": \"timeout\""), "{row:?}");
    // The batch form takes the bare file name too.
    batch(
        &w,
        &format!(
            "{{\"item\":\"perf\",\"outcome\":\"not-applicable\",\"refs\":[\"{name}\"],\"reason\":\"no hot path\"}}\n"
        ),
    )?;
    let (text, _) = status(&w)?;
    assert_eq!(checklist::item(&text, "perf"), "not-applicable", "{text}");
    // Never anything but an observation; never one that is not there.
    let missing = format!("runs/{RUN}/observed/20261111T000000Z-000000.json");
    for bad in ["gate:unit", STALL, missing.as_str()] {
        let line = format!(
            "{{\"item\":\"perf\",\"outcome\":\"unsupported\",\"refs\":[{}],\"reason\":\"r\"}}\n",
            serde_json::Value::String(bad.into())
        );
        let (why, exit) = refusal(batch(&w, &line));
        assert!(
            why.contains("not references other than observations of this run") && exit == 3,
            "{bad}: {why}"
        );
    }
    // A timed-out look never supports an item.
    let (why, _) = refusal(cover(&w, "perf", Outcome::Supported, &[&path]));
    assert!(why.contains("is not grounded (timeout)"), "{why}");
    assert_eq!(rows(&w.kb)?.len(), 2);
    Ok(())
}
