//! Kill tests of the candidate write and the check receipts: the test
//! binary re-runs itself as a child that runs `deliver prepare` (or
//! `deliver check`) and aborts at each step of the write protocol. After
//! every death the run holds no candidate or one complete candidate (both
//! files, the payload byte-equal to the source), and only complete,
//! valid receipts (each check its own file), never a half-written one;
//! leftovers are only `.tmp` names; the index equals a rebuild; and a rerun
//! succeeds.
mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use support::{NOW, TestResult, world};
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::{self, Prepare};
use writfold_kb::deliver::receipt;
use writfold_kb::write::Step;
use writfold_kb::{Request, Verb};

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const CHILD: &str = "WRITFOLD_KB_DELIVER_FAULT_CHILD";
const BODY: &str = "Batching is live in staging.\n";

fn request(kb: &Path, verb: DeliverVerb) -> Request {
    Request {
        verb: Verb::Deliver(verb),
        kb: Some(kb.to_path_buf()),
        cwd: kb.to_path_buf(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(NOW),
        probe: writfold_kb::write::no_probe,
    }
}

fn prepare(from: &Path) -> DeliverVerb {
    DeliverVerb::Prepare(Prepare {
        mode: "pr-comment".into(),
        from: from.to_path_buf(),
        audience: Some("internal".into()),
        destination: None,
        run: Some(RUN.into()),
    })
}

fn check() -> DeliverVerb {
    DeliverVerb::Check {
        candidate: None,
        check: None,
        run: Some(RUN.into()),
    }
}

fn abort_at(step: Step) {
    if std::env::var("WRITFOLD_KB_FAULT_STEP").ok().as_deref() == Some(step.name()) {
        std::process::abort();
    }
}

/// The child: one prepare or check that aborts at a step.
#[test]
fn child() -> TestResult {
    let Ok(kb) = std::env::var(CHILD) else {
        return Ok(());
    };
    let kb = PathBuf::from(kb);
    let verb = match std::env::var("WRITFOLD_KB_FAULT_VERB").ok().as_deref() {
        Some("check") => check(),
        _ => prepare(&kb.join("source.md")),
    };
    let mut req = request(&kb, verb);
    req.probe = abort_at;
    writfold_kb::run(&req)?;
    Ok(())
}

/// The complete candidates of the run; anything else present fails.
fn complete(kb: &Path) -> TestResult<usize> {
    let dir = kb.join(format!("runs/{RUN}/deliveries"));
    let mut found = 0;
    for entry in std::fs::read_dir(&dir).into_iter().flatten() {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(!name.starts_with('.'), "leftover {name}");
        let path = entry.path();
        assert_eq!(
            std::fs::read_to_string(path.join(candidate::PAYLOAD))?,
            BODY
        );
        let text = std::fs::read_to_string(path.join(candidate::MANIFEST))?;
        candidate::parse(&text, RUN, &name).map_err(|why| format!("{name}: {why}"))?;
        found += 1;
    }
    Ok(found)
}

/// Valid receipts of the run's only candidate; `.tmp` leftovers allowed.
fn receipts(kb: &Path) -> TestResult<usize> {
    let base = kb.join(format!("runs/{RUN}/deliveries"));
    let mut found = 0;
    for cand in std::fs::read_dir(&base)?.flatten() {
        let id = cand.file_name().to_string_lossy().into_owned();
        for entry in std::fs::read_dir(cand.path().join(receipt::DIR))
            .into_iter()
            .flatten()
        {
            let name = entry?.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                assert!(name.ends_with(".tmp"), "{name}");
                continue;
            }
            let text = std::fs::read_to_string(cand.path().join(receipt::DIR).join(&name))?;
            receipt::parse(&text, RUN, &id, &name).map_err(|why| format!("{name}: {why}"))?;
            found += 1;
        }
    }
    Ok(found)
}

fn child_at(kb: &Path, verb: &str, step: Step) -> TestResult {
    let status = Command::new(std::env::current_exe()?)
        .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
        .env(CHILD, kb)
        .env("WRITFOLD_KB_FAULT_VERB", verb)
        .env("WRITFOLD_KB_FAULT_STEP", step.name())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    assert!(!status.success(), "{verb} at {} did not stop", step.name());
    Ok(())
}

#[test]
fn every_fault_point_leaves_no_candidate_or_a_complete_one() -> TestResult {
    for step in Step::ALL {
        let w = world(&[])?;
        std::fs::write(w.kb.join("source.md"), BODY)?;
        child_at(&w.kb, "prepare", step)?;
        let found = complete(&w.kb)?;
        let expected = matches!(step, Step::Published | Step::DirSynced | Step::Unlocked);
        assert_eq!(found == 1, expected, "prepare at {}", step.name());
        let (verify, exit) = w.run(&w.request(Verb::IndexVerify))?;
        assert!(exit == 0 && !verify.contains("unreadable"), "{verify}");
        let (text, exit) = w.run(&request(&w.kb, prepare(&w.kb.join("source.md"))))?;
        assert_eq!(exit, 0, "rerun after {}:\n{text}", step.name());
        assert_eq!(complete(&w.kb)?, found + 1);
    }
    Ok(())
}

/// A user mode's script gate adds no write path: its receipt joins the
/// built-in check's in the same locked apply, with the same guarantees.
#[test]
fn a_script_gate_receipt_survives_every_fault_point() -> TestResult {
    for step in Step::ALL {
        let w = world(&["commit-message"])?;
        std::fs::write(w.kb.join("source.md"), BODY)?;
        let p = DeliverVerb::Prepare(Prepare {
            mode: "commit-message".into(),
            from: w.kb.join("source.md"),
            audience: Some("external".into()),
            destination: None,
            run: Some(RUN.into()),
        });
        w.run(&request(&w.kb, p))?;
        child_at(&w.kb, "check", step)?;
        let found = receipts(&w.kb)?;
        assert!(found <= 2, "check at {}", step.name());
        if matches!(step, Step::Unlocked) {
            assert_eq!(found, 2);
        }
        let (verify, exit) = w.run(&w.request(Verb::IndexVerify))?;
        assert!(exit == 0 && !verify.contains("unreadable"), "{verify}");
        // The attested language gate stays open, so the candidate is not
        // ready; the script gate is satisfied.
        let (text, _) = w.run(&request(&w.kb, check()))?;
        assert!(
            text.contains("commit-format [satisfied]"),
            "rerun after {}:\n{text}",
            step.name()
        );
        assert_eq!(receipts(&w.kb)?, found + 2);
    }
    Ok(())
}

#[test]
fn every_fault_point_leaves_only_complete_receipts() -> TestResult {
    for step in Step::ALL {
        let w = world(&[])?;
        std::fs::write(w.kb.join("source.md"), BODY)?;
        w.run(&request(&w.kb, prepare(&w.kb.join("source.md"))))?;
        child_at(&w.kb, "check", step)?;
        // Three receipts are written in one locked apply; a death between
        // two of them leaves the earlier ones, each complete and valid.
        let found = receipts(&w.kb)?;
        assert!(found <= 3, "check at {}", step.name());
        if matches!(step, Step::Unlocked) {
            assert_eq!(found, 3);
        }
        let (verify, exit) = w.run(&w.request(Verb::IndexVerify))?;
        assert!(exit == 0 && !verify.contains("unreadable"), "{verify}");
        let (text, _) = w.run(&request(&w.kb, check()))?;
        assert!(
            text.contains("delivery "),
            "rerun after {}:\n{text}",
            step.name()
        );
        assert_eq!(receipts(&w.kb)?, found + 3);
    }
    Ok(())
}
