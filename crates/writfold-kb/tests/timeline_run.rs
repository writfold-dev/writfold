//! Run entries, mentioned records and literal run-directory matching.
mod support;

use serde_json::Value;
use std::path::Path;
use support::{TestResult, World, git, world};
use writfold_kb::Verb;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const DECISION: &str = "proposal/decision/2026-03-02-batch-ledger-writes";

fn timeline(target: &str) -> Verb {
    Verb::Timeline {
        target: target.into(),
    }
}

fn run_world() -> TestResult<World> {
    let w = world(&[])?;
    let text = format!(
        "## 2026-11-03T16:00:00Z\n\nFinished {RUN}.\n\n\
         ## 2026-11-01T09:30:00Z\n\nStarted {DECISION}.\n\n\
         ## 2026-11-02T12:00:00Z\n\n{} trailing text\n",
        "é".repeat(105)
    );
    std::fs::write(w.kb.join(format!("runs/{RUN}/checkpoint.md")), text)?;
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", "timeline entries"])?;
    Ok(w)
}

fn golden(w: &World, name: &str, target: &str, bless: bool) -> TestResult {
    let (text, exit) = w.run(&w.request(timeline(target)))?;
    let actual = format!("{text}# exit={exit}\n");
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if bless && std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(file, actual)?;
    } else {
        assert_eq!(std::fs::read_to_string(file)?, actual, "{name}");
    }
    Ok(())
}

#[test]
fn run_timeline_golden() -> TestResult {
    golden(&run_world()?, "timeline-run.txt", RUN, true)
}

#[test]
fn own_entries_have_timestamps_previews_sources_and_current_decision_status() -> TestResult {
    let w = run_world()?;
    let mut request = w.request(timeline(RUN));
    request.json = true;
    let (text, exit) = w.run(&request)?;
    assert_eq!(exit, 0);
    let body: Value = serde_json::from_str(&text)?;
    let rows = body["rows"].as_array().ok_or("rows")?;
    let entries: Vec<_> = rows
        .iter()
        .filter(|row| row["kind"] == "checkpoint")
        .collect();
    assert_eq!(entries.len(), 3);
    let expected = [
        (
            "2026-11-01T09:30:00Z",
            5,
            format!("entry 2 of 3: Started {DECISION}."),
        ),
        (
            "2026-11-02T12:00:00Z",
            9,
            format!("entry 3 of 3: {}", "é".repeat(100)),
        ),
        (
            "2026-11-03T16:00:00Z",
            1,
            format!("entry 1 of 3: Finished {RUN}."),
        ),
    ];
    for (row, (stamp, line, preview)) in entries.iter().zip(expected) {
        assert_eq!(row["id"], stamp);
        assert_eq!(row["text"], preview);
        assert_eq!(row["path"], format!("runs/{RUN}/checkpoint.md"));
        assert_eq!(row["line"], line);
    }
    let decisions: Vec<_> = rows.iter().filter(|row| row["id"] == DECISION).collect();
    assert_eq!(decisions.len(), 2);
    assert!(decisions.iter().all(|row| row["status"] == "accepted"));
    assert_eq!(body["trailer"]["counts"]["checkpoints"], 3);
    assert_eq!(rows.last().ok_or("last row")?["id"], "2026-11-03T16:00:00Z");
    Ok(())
}

#[test]
fn non_run_timelines_keep_existing_goldens() -> TestResult {
    let w = world(&[])?;
    w.run(&w.request(Verb::IndexStatus))?;
    golden(&w, "timeline-asset.txt", "harbor-db", false)?;
    golden(&w, "timeline-ticket.txt", "HBR-101", false)
}

#[test]
fn source_directory_prefix_is_literal_and_bounded() -> TestResult {
    let conn = rusqlite::Connection::open_in_memory()?;
    conn.execute_batch("CREATE TABLE edges(src TEXT PRIMARY KEY, dst TEXT NOT NULL);")?;
    for (src, dst) in [
        ("doc:runs/run_%/checkpoint.md", "own"),
        ("doc:runs/run_%/notes.md", "notes"),
        ("doc:runs/run_ab/checkpoint.md", "wildcard"),
        ("doc:runs/run_%extra/checkpoint.md", "neighbor"),
        ("doc:runs/run_%", "directory"),
    ] {
        conn.execute("INSERT INTO edges VALUES (?1, ?2)", (src, dst))?;
    }
    assert_eq!(
        writfold_kb::index::query::run_destinations(&conn, "run_%")?,
        vec!["notes", "own"]
    );
    Ok(())
}
