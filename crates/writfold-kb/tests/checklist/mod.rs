//! Shared set-up for the checklist gate tests: a `unit` gate over
//! `src/ledger.rs`, and a `review` checklist over `src/batch.rs` with two
//! required items and one optional one.
#![allow(dead_code)]
use crate::support::{NOW, TestResult, World};
use std::path::{Path, PathBuf};
use writfold_kb::gate::GateVerb;
use writfold_kb::gate::checklist::{Cover, Outcome};
use writfold_kb::{Request, Verb};

pub const RUN: &str = "run-20261101T090000Z-a1b2c3";
pub const UNIT: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 30\nscope = [\"src/ledger.rs\"]\n";
pub const REVIEW: &str = "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"checklist\"\nscope = [\"src/batch.rs\"]\nitems = [\"errors: errors are handled or propagated\", \"record: the stall ticket is closed\", \"perf: no new hot-path allocation\"]\noptional = [\"perf\"]\ndescription = \"code review checklist\"\n";
/// A record citation that is fresh in the fixture KB.
pub const STALL: &str = "⟦cite kind=record path=ktlo/2026/hbr-99-flush-stall.md state=done⟧";

pub fn put(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

pub fn define(kb: &Path) -> TestResult {
    put(&kb.join(".writfold/gates/unit.toml"), UNIT)?;
    put(&kb.join(".writfold/gates/review.toml"), REVIEW)
}

pub fn request(kb: &Path, verb: Verb) -> Request {
    Request {
        verb,
        kb: Some(kb.to_path_buf()),
        cwd: kb.to_path_buf(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(NOW),
        probe: writfold_kb::write::no_probe,
    }
}

pub fn cover_verb(code: &Path, item: &str, outcome: Outcome, refs: &[&str]) -> GateVerb {
    let reason = match outcome {
        Outcome::Supported => None,
        _ => Some("no hot path is touched".to_owned()),
    };
    GateVerb::Cover(Cover {
        gate: "review".into(),
        item: item.into(),
        outcome,
        references: refs.iter().map(|r| (*r).to_owned()).collect(),
        reason,
        note: None,
        by: "agent".into(),
        run: Some(RUN.into()),
        repo: Some(code.to_path_buf()),
    })
}

/// Record one cover row; the error is the refusal.
pub fn cover(
    w: &World,
    item: &str,
    outcome: Outcome,
    refs: &[&str],
) -> writfold_kb::error::Result<writfold_kb::emit::Output> {
    let verb = cover_verb(&w.code, item, outcome, refs);
    writfold_kb::run(&w.request(Verb::Gate(verb)))
}

pub fn gate(w: &World, verb: GateVerb) -> TestResult<(String, u8)> {
    let mut req = w.request(Verb::Gate(verb));
    req.now = Some(NOW + 3600);
    w.run(&req)
}

pub fn run_unit(w: &World) -> TestResult {
    let (text, exit) = gate(
        w,
        GateVerb::Run {
            id: "unit".into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    )?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

pub fn status(w: &World) -> TestResult<(String, u8)> {
    gate(
        w,
        GateVerb::Status {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    )
}

/// `[standing]` of a gate in status text.
pub fn standing(text: &str, id: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix(&format!("gate {id} [")))
        .and_then(|rest| rest.split(']').next())
        .unwrap_or("absent")
        .to_owned()
}

/// `[standing]` of one item in status text.
pub fn item(text: &str, id: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix(&format!("  item: {id} [")))
        .and_then(|rest| rest.split(']').next())
        .unwrap_or("absent")
        .to_owned()
}

/// The run's cover row files, by name.
pub fn rows(kb: &Path) -> TestResult<Vec<PathBuf>> {
    let dir = kb.join(format!("runs/{RUN}/covers"));
    let mut out: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter(|p| {
            !p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        })
        .collect();
    out.sort();
    Ok(out)
}

pub fn golden(name: &str, text: &str, exit: u8) -> TestResult {
    let names = regex::Regex::new(r"-(\d{8}T\d{6}Z)-[0-9a-f]{6}")?;
    let text = names
        .replace_all(&text.replace(RUN, "<RUN>"), "-$1-XXXXXX")
        .replace("<RUN>", RUN);
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let actual = format!("{text}# exit={exit}\n");
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(&file, &actual)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&file).map_err(|e| format!("{name}: {e}"))?;
    assert_eq!(expected, actual, "golden {name} differs");
    Ok(())
}
