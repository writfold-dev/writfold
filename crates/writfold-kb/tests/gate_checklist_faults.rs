//! Kill tests of the cover row write: the test binary re-runs itself as a
//! child that runs `gate cover` and aborts at each step of the write
//! protocol. After every death a cover row is either absent or complete
//! and valid, none appears before `published`, leftovers are only
//! dot-named `.tmp` files, the index equals a rebuild, and a rerun
//! records a valid row that covers the item.
mod checklist;
mod support;

use checklist::{RUN, cover_verb, define, item, request, rows, run_unit, status};
use std::path::PathBuf;
use std::process::Command;
use support::{TestResult, world};
use writfold_kb::Verb;
use writfold_kb::gate::checklist::{Outcome, cover};
use writfold_kb::write::Step;

const CHILD: &str = "WRITFOLD_KB_COVER_FAULT_CHILD";
const CODE: &str = "WRITFOLD_KB_COVER_FAULT_CODE";

fn abort_at(step: Step) {
    if std::env::var("WRITFOLD_KB_FAULT_STEP").ok().as_deref() == Some(step.name()) {
        std::process::abort();
    }
}

/// The child: one `gate cover` that aborts at a step. Does nothing unless
/// the parent set the environment.
#[test]
fn child() -> TestResult {
    let (Ok(kb), Ok(code)) = (std::env::var(CHILD), std::env::var(CODE)) else {
        return Ok(());
    };
    let (kb, code) = (PathBuf::from(kb), PathBuf::from(code));
    let verb = cover_verb(&code, "errors", Outcome::Supported, &["gate:unit"]);
    let mut req = request(&kb, Verb::Gate(verb));
    req.probe = abort_at;
    writfold_kb::run(&req)?;
    Ok(())
}

/// The run's cover rows, each checked to be complete and valid.
fn valid_rows(kb: &std::path::Path) -> TestResult<usize> {
    let dir = kb.join(format!("runs/{RUN}/covers"));
    let mut found = 0;
    for entry in std::fs::read_dir(&dir).into_iter().flatten() {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            assert!(name.ends_with(".tmp"), "{name}");
            continue;
        }
        let text = std::fs::read_to_string(entry.path())?;
        cover::parse(&text, RUN, &name).map_err(|why| format!("{name}: {why}\n{text}"))?;
        found += 1;
    }
    Ok(found)
}

#[test]
fn every_fault_point_leaves_no_row_or_a_valid_one() -> TestResult {
    for step in Step::ALL {
        let w = world(&[])?;
        define(&w.kb)?;
        run_unit(&w)?;
        let status_code = Command::new(std::env::current_exe()?)
            .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
            .env(CHILD, &w.kb)
            .env(CODE, &w.code)
            .env("WRITFOLD_KB_FAULT_STEP", step.name())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        assert!(
            !status_code.success(),
            "gate cover at {} did not stop",
            step.name()
        );
        let published = valid_rows(&w.kb)?;
        let expected = matches!(step, Step::Published | Step::DirSynced | Step::Unlocked);
        assert_eq!(published == 1, expected, "at {}", step.name());
        assert_eq!(rows(&w.kb)?.len(), published);
        let (verify, exit) = w.run(&w.request(Verb::IndexVerify))?;
        assert!(exit == 0 && !verify.contains("unreadable"), "{verify}");
        let verb = cover_verb(&w.code, "errors", Outcome::Supported, &["gate:unit"]);
        writfold_kb::run(&w.request(Verb::Gate(verb)))?;
        assert_eq!(valid_rows(&w.kb)?, published + 1);
        let (text, _) = status(&w)?;
        assert_eq!(
            item(&text, "errors"),
            "supported",
            "after {}:\n{text}",
            step.name()
        );
    }
    Ok(())
}
