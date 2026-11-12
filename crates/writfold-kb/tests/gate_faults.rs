//! Kill tests of the gate receipt write: the test binary re-runs itself as
//! a child that runs `gate run` and aborts at each step of the write
//! protocol. After every death a receipt is either absent or complete and
//! valid, none appears before `published`, leftovers are only dot-named
//! `.tmp` files, the index equals a rebuild, and a rerun records a valid
//! receipt that satisfies the gate.
mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use support::{NOW, TestResult, world};
use writfold_kb::gate::GateVerb;
use writfold_kb::gate::receipt;
use writfold_kb::write::Step;
use writfold_kb::{Request, Verb};

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const CHILD: &str = "WRITFOLD_KB_GATE_FAULT_CHILD";

fn request(kb: &Path, verb: GateVerb) -> Request {
    Request {
        verb: Verb::Gate(verb),
        kb: Some(kb.to_path_buf()),
        cwd: kb.to_path_buf(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(NOW),
        probe: writfold_kb::write::no_probe,
    }
}

fn quick(kb: &Path) -> GateVerb {
    GateVerb::Run {
        id: "quick".into(),
        run: Some(RUN.into()),
        repo: Some(kb.to_path_buf()),
    }
}

fn abort_at(step: Step) {
    if std::env::var("WRITFOLD_KB_FAULT_STEP").ok().as_deref() == Some(step.name()) {
        std::process::abort();
    }
}

/// The child: one `gate run` that aborts at a step. Does nothing unless
/// the parent set the environment.
#[test]
fn child() -> TestResult {
    let Ok(kb) = std::env::var(CHILD) else {
        return Ok(());
    };
    let kb = PathBuf::from(kb);
    let mut req = request(&kb, quick(&kb));
    req.probe = abort_at;
    writfold_kb::run(&req)?;
    Ok(())
}

/// The receipt files of the run, each checked to be complete and valid.
fn valid_receipts(kb: &Path) -> TestResult<usize> {
    let dir = kb.join(format!("runs/{RUN}/gates"));
    let mut found = 0;
    for entry in std::fs::read_dir(&dir).into_iter().flatten() {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            assert!(name.ends_with(".tmp"), "{name}");
            continue;
        }
        let text = std::fs::read_to_string(entry.path())?;
        receipt::parse(&text, RUN, &name).map_err(|why| format!("{name}: {why}\n{text}"))?;
        found += 1;
    }
    Ok(found)
}

#[test]
fn every_fault_point_leaves_no_receipt_or_a_valid_one() -> TestResult {
    for step in Step::ALL {
        let w = world(&[])?;
        let gates = w.kb.join(".writfold/gates");
        std::fs::create_dir_all(&gates)?;
        std::fs::write(
            gates.join("quick.toml"),
            "schema = \"writfold-gate/1\"\nid = \"quick\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 30\nscope = [\"inventory/**\"]\n",
        )?;
        let status = Command::new(std::env::current_exe()?)
            .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
            .env(CHILD, &w.kb)
            .env("WRITFOLD_KB_FAULT_STEP", step.name())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        assert!(
            !status.success(),
            "gate run at {} did not stop",
            step.name()
        );
        let published = valid_receipts(&w.kb)?;
        let expected = matches!(step, Step::Published | Step::DirSynced | Step::Unlocked);
        assert_eq!(published == 1, expected, "at {}", step.name());
        assert!(published <= 1);
        let (verify, exit) = w.run(&w.request(Verb::IndexVerify))?;
        assert!(exit == 0 && !verify.contains("unreadable"), "{verify}");
        let (text, exit) = w.run(&request(&w.kb, quick(&w.kb)))?;
        assert_eq!(exit, 0, "rerun after {}:\n{text}", step.name());
        assert_eq!(valid_receipts(&w.kb)?, published + 1);
        let (text, exit) = w.run(&request(
            &w.kb,
            GateVerb::Status {
                run: Some(RUN.into()),
                repo: Some(w.kb.clone()),
            },
        ))?;
        assert!(
            exit == 0 && text.contains("gate quick [satisfied]"),
            "{text}"
        );
    }
    Ok(())
}
