//! `kb delta`: what differs since a mark or a commit, classified by the
//! layout; blob IDs, not paths, decide; ignored files never show; the
//! session-start hook takes the session's mark once; a kill during the
//! mark write leaves no mark or a whole one.
mod support;

use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::delta::DeltaVerb;
use writfold_kb::hook::{HookEnv, run};
use writfold_kb::write::Step;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const CHILD: &str = "WRITFOLD_KB_DELTA_CHILD";

fn put(path: &Path, text: &str) -> TestResult {
    std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
    std::fs::write(path, text)?;
    Ok(())
}

fn delta(w: &World, verb: DeltaVerb) -> writfold_kb::error::Result<(Value, u8)> {
    let mut request = w.request(Verb::Delta(verb));
    request.json = true;
    request.budget = writfold_kb::MAX_BUDGET;
    let out = writfold_kb::run(&request)?;
    Ok((
        serde_json::from_str(&out.text).unwrap_or(Value::Null),
        out.exit,
    ))
}

fn rows(body: &Value) -> Vec<(String, String, String, String)> {
    body["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| {
            let s = |k: &str| r[k].as_str().unwrap_or("").to_owned();
            (s("path"), s("status"), s("text"), s("section"))
        })
        .collect()
}

#[test]
fn writes_after_a_mark_are_rows_by_section() -> TestResult {
    let w = world(&[])?;
    // Dirty before the mark, and left alone: never a row.
    let status = w.kb.join("harbor/STATUS.md");
    let text = std::fs::read_to_string(&status)?;
    put(&status, &format!("{text}\nEdited before the mark.\n"))?;
    assert_eq!(delta(&w, DeltaVerb::Mark("s-1".into()))?.1, 0);
    let (body, exit) = delta(&w, DeltaVerb::SinceMark("s-1".into()))?;
    assert_eq!((rows(&body).len(), exit), (0, 2), "{body}");
    let ticket = "---\nticket: HBR-300\ntitle: Replay\nstatus: todo\n---\n\n# HBR-300: Replay\n";
    put(&w.kb.join("ktlo/2026/hbr-300-replay.md"), ticket)?;
    let run = w.kb.join("runs").join(RUN);
    put(
        &run.join("observed/look-20261111T000000Z-a1b2c3.json"),
        "{}\n",
    )?;
    put(
        &run.join("covers/review.tests-20261111T000000Z-a1b2c3.json"),
        "{}\n",
    )?;
    let exclude = w.kb.join(".git/info/exclude");
    put(&exclude, "scratch/\n")?;
    put(&w.kb.join("scratch/ignored.md"), "never a row\n")?;
    let (body, exit) = delta(&w, DeltaVerb::SinceMark("s-1".into()))?;
    assert_eq!(exit, 0);
    let mut got = rows(&body);
    got.sort();
    let want = [
        ("ktlo/2026/hbr-300-replay.md", "added", "ticket", "records"),
        (
            &format!("runs/{RUN}/covers/review.tests-20261111T000000Z-a1b2c3.json")[..],
            "added",
            "cover",
            "run",
        ),
        (
            &format!("runs/{RUN}/observed/look-20261111T000000Z-a1b2c3.json")[..],
            "added",
            "observation",
            "run",
        ),
    ];
    let want: Vec<(String, String, String, String)> = want
        .iter()
        .map(|(a, b, c, d)| ((*a).into(), (*b).into(), (*c).into(), (*d).into()))
        .collect();
    assert_eq!(got, want);
    let ticket_row = body["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|r| r["text"] == "ticket");
    assert_eq!(
        ticket_row.map(|r| r["id"].clone()),
        Some(json!("HBR-300")),
        "{body}"
    );
    let counts = &body["trailer"]["counts"];
    assert_eq!(
        (
            counts["changed"].as_u64(),
            counts["records"].as_u64(),
            counts["run"].as_u64()
        ),
        (Some(3), Some(1), Some(2))
    );
    // The file dirty at the mark, edited again: now a row.
    put(&status, &format!("{text}\nEdited after the mark.\n"))?;
    let (body, _) = delta(&w, DeltaVerb::SinceMark("s-1".into()))?;
    assert!(
        rows(&body)
            .iter()
            .any(|r| r.0 == "harbor/STATUS.md" && r.1 == "modified"),
        "{body}"
    );
    Ok(())
}

#[test]
fn since_a_revision() -> TestResult {
    let w = world(&[])?;
    let (body, exit) = delta(&w, DeltaVerb::Since("HEAD".into()))?;
    assert_eq!(exit, 2);
    assert!(
        body["notes"]
            .to_string()
            .contains("nothing in the KB differs from HEAD ("),
        "{body}"
    );
    let refused = delta(&w, DeltaVerb::Since("no-such-branch".into()));
    assert!(
        matches!(refused, Err(writfold_kb::error::KbError::Refused(_))),
        "{refused:?}"
    );
    // A mark whose commit this clone does not have.
    let gone = json!({"schema": "writfold-kb-mark/1", "name": "gone", "head": "a".repeat(40),
        "taken": "2026-11-10T00:00:00Z", "dirty": {}});
    put(
        &w.kb.join(".git/writfold/marks/gone.json"),
        &gone.to_string(),
    )?;
    let refused = delta(&w, DeltaVerb::SinceMark("gone".into()));
    assert!(
        matches!(&refused, Err(writfold_kb::error::KbError::Refused(why)) if why.contains("not in this clone")),
        "{refused:?}"
    );
    let refused = delta(&w, DeltaVerb::SinceMark("never-taken".into()));
    assert!(matches!(
        refused,
        Err(writfold_kb::error::KbError::Refused(_))
    ));
    Ok(())
}

fn start(w: &World, source: &str) {
    let input = json!({"session_id": "s-hook", "cwd": w.kb, "hook_event_name": "SessionStart", "source": source});
    let env = HookEnv {
        home: None,
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW),
    };
    run("session-start", &input.to_string(), &env);
}

#[test]
fn session_start_takes_the_sessions_mark_once() -> TestResult {
    let w = world(&[])?;
    let mark = w.kb.join(".git/writfold/marks/s-hook.json");
    start(&w, "startup");
    let first = std::fs::read_to_string(&mark)?;
    put(&w.kb.join("harbor/STATUS.md"), "changed\n")?;
    start(&w, "compact");
    assert_eq!(std::fs::read_to_string(&mark)?, first);
    let (body, exit) = delta(&w, DeltaVerb::SinceMark("s-hook".into()))?;
    assert_eq!(exit, 0);
    assert_eq!(rows(&body)[0].0, "harbor/STATUS.md");
    Ok(())
}

fn abort_at(step: Step) {
    if std::env::var("WRITFOLD_KB_FAULT_STEP").ok().as_deref() == Some(step.name()) {
        std::process::abort();
    }
}

/// The child: one mark that aborts at the step in the environment.
#[test]
fn child() -> TestResult {
    let Ok(kb) = std::env::var(CHILD) else {
        return Ok(());
    };
    let mut request = writfold_kb::Request {
        verb: Verb::Delta(DeltaVerb::Mark("s-kill".into())),
        kb: Some(kb.clone().into()),
        cwd: kb.into(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(NOW),
        probe: abort_at,
    };
    request.json = false;
    writfold_kb::run(&request)?;
    Ok(())
}

#[test]
fn a_kill_during_the_mark_write_leaves_none_or_a_whole_one() -> TestResult {
    let w = world(&[])?;
    let mark = w.kb.join(".git/writfold/marks/s-kill.json");
    for step in [Step::TempWritten, Step::Published] {
        let _ = std::fs::remove_file(&mark);
        let status = Command::new(std::env::current_exe()?)
            .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
            .env(CHILD, &w.kb)
            .env("WRITFOLD_KB_FAULT_STEP", step.name())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        assert!(
            !status.success(),
            "the child did not stop at {}",
            step.name()
        );
        match std::fs::read_to_string(&mark) {
            Ok(text) => {
                let value: Value = serde_json::from_str(&text)?;
                assert_eq!(value["schema"], "writfold-kb-mark/1");
            }
            Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
        }
    }
    assert!(
        mark.is_file(),
        "a kill after the rename leaves the whole mark"
    );
    Ok(())
}
