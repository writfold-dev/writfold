//! The shadow check (`WRITFOLD_KB_VERIFY=1`): every refresh is compared
//! with a rebuild; a difference replaces the index and fails the call. The
//! variable is process-wide, so the check runs in a child process: this
//! test binary re-executed on its `child` test with the variable set.
mod support;

use std::path::PathBuf;
use std::process::Command;
use support::{TestResult, world};
use writfold_kb::{DEFAULT_BUDGET, Request, Verb};

const CHILD_KB: &str = "WRITFOLD_KB_SHADOW_CHILD";

#[test]
fn a_refresh_that_differs_from_a_rebuild_is_caught_and_replaced() -> TestResult {
    let w = world(&[])?;
    w.run(&w.request(Verb::IndexStatus))?;
    let index = std::fs::read_dir(w.kb.join(".git/writfold"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.extension().is_some_and(|x| x == "sqlite"))
        .ok_or("index file")?;
    // Tamper with a Phase A row of a file the next refresh does not touch.
    rusqlite::Connection::open(&index)?.execute_batch(
        "UPDATE sections SET name = 'Tampered' WHERE path = 'ktlo/2026/hbr-99-flush-stall.md';",
    )?;
    std::fs::write(w.kb.join("notes.md"), "# Notes\n\nharbor-api\n")?;
    let child = Command::new(std::env::current_exe()?)
        .args(["--exact", "child", "--nocapture", "--test-threads=1"])
        .env("WRITFOLD_KB_VERIFY", "1")
        .env(CHILD_KB, &w.kb)
        .output()?;
    let stdout = String::from_utf8_lossy(&child.stdout);
    assert!(child.status.success(), "{stdout}");
    assert!(
        stdout.contains("shadow: index: refresh differs from a rebuild at sections ktlo/2026/hbr-99-flush-stall.md"),
        "{stdout}"
    );
    let (again, exit) = w.run(&w.request(Verb::IndexVerify))?;
    assert_eq!(exit, 0, "the rebuild replaced the tampered index:\n{again}");
    Ok(())
}

/// Runs only when re-executed by the test above.
#[test]
fn child() -> TestResult {
    let Some(kb) = std::env::var_os(CHILD_KB) else {
        return Ok(());
    };
    let request = Request {
        verb: Verb::IndexStatus,
        kb: Some(PathBuf::from(kb)),
        cwd: PathBuf::from("/"),
        budget: DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(support::NOW),
    };
    match writfold_kb::run(&request) {
        Err(error) => println!("shadow: {error}"),
        Ok(output) => println!("shadow passed: {}", output.text),
    }
    Ok(())
}
