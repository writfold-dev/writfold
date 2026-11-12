//! Kill and fault tests of the write protocol (design section 8.4).
//!
//! The parent re-runs this test binary as a child that performs one write
//! and aborts the process at a named step (through the request's probe),
//! or is killed with SIGKILL at a random moment while it appends
//! checkpoints in a loop. After every death the target holds the complete
//! old bytes or the complete new bytes, the KB reads with nothing
//! unreadable, the index equals a rebuild, leftover temporary files are
//! dot-named and never indexed, and re-running the write either succeeds
//! or reports a conflict, never a duplicate entry.
mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use support::{NOW, TestResult, World, world};
use writfold_kb::write::record::Ticket;
use writfold_kb::write::retrieval::Retrieval;
use writfold_kb::write::run::{Checkpoint, NewRun};
use writfold_kb::write::status::SetStatus;
use writfold_kb::write::{Step, Write};
use writfold_kb::{Request, Verb};

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const CHILD: &str = "WRITFOLD_KB_FAULT_CHILD";
const RECEIPT_TRAILER: &str =
    "# coverage=complete files=1/1 hits=1 shown=1 omitted=0 cursor=none program=abababababab";

/// The writes under test, by name.
fn write(name: &str, note: &str) -> Write {
    match name {
        "checkpoint" => Write::Checkpoint(Checkpoint {
            run: Some(RUN.into()),
            new_run: None,
            note: note.into(),
        }),
        "new-run" => Write::Checkpoint(Checkpoint {
            run: None,
            new_run: Some(NewRun {
                project: "harbor".into(),
                task: "Faults".into(),
                mode: None,
            }),
            note: note.into(),
        }),
        "ticket" => Write::Ticket(Ticket {
            ticket: "HBR-120".into(),
            title: "Fault ticket".into(),
            assets: vec!["harbor-api".into()],
            ..Ticket::default()
        }),
        "retrieval" => Write::Retrieval(Retrieval {
            run: Some(RUN.into()),
            tool: "tuft/0".into(),
            program: format!("sha256:{}", "ab".repeat(32)),
            repo: "demo-service".into(),
            commit: "0".repeat(40),
            trailer: RECEIPT_TRAILER.into(),
            rows: 1,
            shown: 1,
            cites: Vec::new(),
        }),
        _ => Write::SetStatus(SetStatus {
            target: "HBR-101".into(),
            state: "blocked".into(),
            ..SetStatus::default()
        }),
    }
}

fn request(kb: &Path, verb: Verb) -> Request {
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

/// Abort the process at the step named in the environment.
fn abort_at(step: Step) {
    if std::env::var("WRITFOLD_KB_FAULT_STEP").ok().as_deref() == Some(step.name()) {
        std::process::abort();
    }
}

/// The child: one write that aborts at a step, or checkpoints in a loop
/// until killed. Does nothing unless the parent set the environment.
#[test]
fn child() -> TestResult {
    let (Ok(kb), Ok(what)) = (
        std::env::var(CHILD),
        std::env::var("WRITFOLD_KB_FAULT_WRITE"),
    ) else {
        return Ok(());
    };
    let kb = PathBuf::from(kb);
    if what == "loop" {
        loop {
            let n = entries(&kb)?;
            let note = format!("note {n}");
            let _ = writfold_kb::run(&request(&kb, Verb::Write(write("checkpoint", &note))));
        }
    }
    let mut req = request(&kb, Verb::Write(write(&what, "The fault note.")));
    req.probe = abort_at;
    writfold_kb::run(&req)?;
    Ok(())
}

fn spawn(kb: &Path, what: &str, step: Option<Step>) -> TestResult<std::process::Child> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
        .env(CHILD, kb)
        .env("WRITFOLD_KB_FAULT_WRITE", what)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if let Some(step) = step {
        command.env("WRITFOLD_KB_FAULT_STEP", step.name());
    }
    Ok(command.spawn()?)
}

fn read(kb: &Path, path: &str) -> Option<String> {
    std::fs::read_to_string(kb.join(path)).ok()
}

/// Entries of the fixture run's checkpoint file.
fn entries(kb: &Path) -> TestResult<usize> {
    let text = read(kb, &format!("runs/{RUN}/checkpoint.md")).ok_or("no checkpoint file")?;
    Ok(text.lines().filter(|l| l.starts_with("## ")).count())
}

/// Dot-named temporary files Writfold left in the KB.
fn leftovers(dir: &Path, found: &mut Vec<String>) -> TestResult {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".git" {
            continue;
        }
        if entry.file_type()?.is_dir() {
            leftovers(&entry.path(), found)?;
        } else if name.starts_with(".writfold-kb-") {
            found.push(name);
        }
    }
    Ok(())
}

/// The KB reads completely and its index equals a rebuild.
fn sound(w: &World) -> TestResult {
    let (text, exit) = w.run(&w.request(Verb::IndexVerify))?;
    if exit != 0 || text.contains("unreadable") {
        return Err(format!("after a fault:\n{text}").into());
    }
    Ok(())
}

/// The new run directories (not the fixture's).
fn new_runs(kb: &Path) -> TestResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(kb.join("runs"))? {
        let entry = entry?;
        if entry.file_name() != RUN {
            out.push(entry.path());
        }
    }
    Ok(out)
}

/// The observable state of one write's target: `None` before, `Some`
/// after; anything in between is a failure.
fn state(kb: &Path, what: &str, old: &str) -> TestResult<bool> {
    match what {
        "checkpoint" => {
            let now = read(kb, &format!("runs/{RUN}/checkpoint.md")).ok_or("checkpoint gone")?;
            let new = format!("{old}\n## 2026-11-11T00:00:00Z\n\nThe fault note.\n");
            if now == old || now == new {
                Ok(now == new)
            } else {
                Err(format!("checkpoint holds neither old nor new bytes:\n{now}").into())
            }
        }
        "new-run" => {
            let runs = new_runs(kb)?;
            for dir in &runs {
                let manifest = std::fs::read_to_string(dir.join("manifest.json"))?;
                serde_json::from_str::<serde_json::Value>(&manifest)?;
                let checkpoint = std::fs::read_to_string(dir.join("checkpoint.md"))?;
                assert_eq!(checkpoint, "\n## 2026-11-11T00:00:00Z\n\nThe fault note.\n");
                assert!(dir.join("writfold-run.json").is_file() && dir.join("receipts").is_dir());
            }
            Ok(!runs.is_empty())
        }
        "retrieval" => {
            // Receipts are create-new under fresh names: each one present
            // is complete, and none means the write never published.
            let dir = kb.join(format!("runs/{RUN}/retrievals"));
            let mut found = 0;
            for entry in std::fs::read_dir(&dir).into_iter().flatten() {
                let entry = entry?;
                if entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                let text = std::fs::read_to_string(entry.path())?;
                let tail = format!("trailer: `{RECEIPT_TRAILER}`\n");
                if !(text.starts_with("---\nkind: retrieval\n") && text.ends_with(&tail)) {
                    return Err(format!("partial receipt:\n{text}").into());
                }
                found += 1;
            }
            Ok(found > 0)
        }
        "ticket" => match read(kb, "ktlo/2026/hbr-120-fault-ticket.md") {
            None => Ok(false),
            Some(text) if text.ends_with("## Outcome\n\n\n") => Ok(true),
            Some(text) => Err(format!("partial ticket:\n{text}").into()),
        },
        _ => {
            let now = read(kb, "ktlo/2026/hbr-101-slow-ledger-flush.md").ok_or("ticket gone")?;
            let new = old
                .replace("status: in-progress", "status: blocked")
                .replace("updated: 2026-11-03", "updated: 2026-11-11");
            if now == old || now == new {
                Ok(now == new)
            } else {
                Err(format!("ticket holds neither old nor new bytes:\n{now}").into())
            }
        }
    }
}

#[test]
fn every_fault_point_leaves_old_or_new_bytes() -> TestResult {
    for what in ["checkpoint", "new-run", "ticket", "status", "retrieval"] {
        for step in Step::ALL {
            let w = world(&[])?;
            let old = match what {
                "checkpoint" => read(&w.kb, &format!("runs/{RUN}/checkpoint.md")),
                _ => read(&w.kb, "ktlo/2026/hbr-101-slow-ledger-flush.md"),
            }
            .ok_or("fixture file")?;
            let status = spawn(&w.kb, what, Some(step))?.wait()?;
            assert!(!status.success(), "{what} at {} did not stop", step.name());
            let published = state(&w.kb, what, &old)?;
            let expected = matches!(step, Step::Published | Step::DirSynced | Step::Unlocked);
            assert_eq!(published, expected, "{what} at {}", step.name());
            let mut temps = Vec::new();
            leftovers(&w.kb, &mut temps)?;
            assert!(temps.iter().all(|t| t.ends_with(".tmp")), "{temps:?}");
            sound(&w)?;
            // Re-running succeeds before publication and is refused after.
            let rerun = writfold_kb::run(&w.request(Verb::Write(write(what, "The fault note."))));
            match (&rerun, published, what) {
                (Ok(_), false, _) | (Ok(_), true, "new-run") => {}
                (Err(error), true, _) if error.exit_code() == 3 => {}
                (Ok(_), true, "status" | "retrieval") => {}
                _ => {
                    return Err(
                        format!("{what} at {}: rerun gave {:?}", step.name(), rerun.err()).into(),
                    );
                }
            }
            assert!(
                state(&w.kb, what, &old)?,
                "{what} at {}: not written after rerun",
                step.name()
            );
            if what == "checkpoint" {
                assert_eq!(
                    entries(&w.kb)?,
                    3,
                    "a duplicate entry after {}",
                    step.name()
                );
            }
            sound(&w)?;
        }
    }
    Ok(())
}

#[test]
fn sigkill_at_random_moments() -> TestResult {
    let w = world(&[])?;
    let mut rng: u64 = 0x5eed_0000_0000_0003;
    let rounds: usize = std::env::var("WRITFOLD_KB_KILL_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(40);
    for round in 0..rounds {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let mut child = spawn(&w.kb, "loop", None)?;
        // The last round waits long enough for at least one write to finish
        // even on a loaded machine (the unit gate runs tests in parallel), so
        // "some write completed" checks progress, not scheduling luck.
        let wait = if round + 1 == rounds {
            5_000
        } else {
            20 + rng % 180
        };
        std::thread::sleep(Duration::from_millis(wait));
        child.kill()?;
        child.wait()?;
        let text = read(&w.kb, &format!("runs/{RUN}/checkpoint.md")).ok_or("checkpoint")?;
        let notes: Vec<&str> = text.lines().filter(|l| l.starts_with("note ")).collect();
        for (n, note) in notes.iter().enumerate() {
            assert_eq!(*note, format!("note {}", n + 2), "{text}");
        }
        assert!(text.ends_with('\n'));
    }
    let mut temps = Vec::new();
    leftovers(&w.kb, &mut temps)?;
    assert!(temps.iter().all(|t| t.ends_with(".tmp")), "{temps:?}");
    sound(&w)?;
    assert!(entries(&w.kb)? > 2, "no checkpoint was written");
    let (text, _) = w.run(&w.request(Verb::Resume {
        run: Some(RUN.into()),
    }))?;
    assert!(text.contains("note: note "), "{text}");
    Ok(())
}

/// The file an in-process probe edits behind the writer's back.
static EDIT: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

fn edit_after_temp_synced(step: Step) {
    if step != Step::TempSynced {
        return;
    }
    if let Ok(guard) = EDIT.lock()
        && let Some(path) = guard.as_ref()
    {
        let mut text = std::fs::read_to_string(path).unwrap_or_default();
        text.push_str("\nAn edit from another tool.\n");
        let _ = std::fs::write(path, text);
    }
}

/// An edit that lands after the writer read the file (here: after its
/// temporary file is synced) is never overwritten: the re-check refuses.
#[test]
fn recheck_refuses_an_edit_made_during_the_write() -> TestResult {
    let w = world(&[])?;
    let path = w.kb.join(format!("runs/{RUN}/checkpoint.md"));
    if let Ok(mut guard) = EDIT.lock() {
        *guard = Some(path.clone());
    }
    let mut req = w.request(Verb::Write(write("checkpoint", "Racing note.")));
    req.probe = edit_after_temp_synced;
    let outcome = writfold_kb::run(&req);
    if let Ok(mut guard) = EDIT.lock() {
        *guard = None;
    }
    match outcome {
        Err(error) if error.exit_code() == 3 => {}
        other => {
            return Err(format!("expected a conflict, got {:?}", other.map(|o| o.text)).into());
        }
    }
    let text = std::fs::read_to_string(&path)?;
    assert!(text.ends_with("An edit from another tool.\n") && !text.contains("Racing note."));
    let mut temps = Vec::new();
    leftovers(&w.kb, &mut temps)?;
    assert!(temps.is_empty(), "{temps:?}");
    Ok(())
}
