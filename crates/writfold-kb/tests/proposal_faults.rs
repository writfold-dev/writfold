//! Kill tests of the proposal queue's two write paths. The test binary
//! re-runs itself as a child that queues a proposal (or accepts one) and
//! aborts at each step. After every death:
//!
//! - **propose:** the queue holds no proposal or one complete one, any
//!   leftover is a dot-named `.tmp` file, nothing tracked changed, and a
//!   rerun leaves exactly one proposal;
//! - **accept:** the proposal is never lost without its record (the
//!   proposal file is removed only after the record is published), the
//!   ticket is absent or complete, the index equals a rebuild, and a rerun
//!   either accepts (before publication) or is refused with the proposal
//!   kept (after it).
mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use support::{NOW, TestResult, world};
use writfold_kb::proposal::{self, ProposalVerb, Propose};
use writfold_kb::write::record::Ticket;
use writfold_kb::write::{Step, Write};
use writfold_kb::{Request, Verb};

const CHILD: &str = "WRITFOLD_KB_PROPOSAL_FAULT_CHILD";
const TICKET: &str = "ktlo/2026/hbr-130-replay-is-slow.md";

fn propose() -> Verb {
    Verb::Proposal(ProposalVerb::Propose(Box::new(Propose {
        write: Write::Ticket(Ticket {
            ticket: "HBR-130".into(),
            title: "Replay is slow".into(),
            assets: vec!["harbor-api".into()],
            ..Ticket::default()
        }),
        session: Some("s1".into()),
        run: None,
        store: None,
    })))
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

fn abort_at(step: Step) {
    if std::env::var("WRITFOLD_KB_FAULT_STEP").ok().as_deref() == Some(step.name()) {
        std::process::abort();
    }
}

/// The child: one propose or accept that aborts at a step.
#[test]
fn child() -> TestResult {
    let Ok(kb) = std::env::var(CHILD) else {
        return Ok(());
    };
    let kb = PathBuf::from(kb);
    let verb = match std::env::var("WRITFOLD_KB_FAULT_ID") {
        Ok(id) => Verb::Proposal(ProposalVerb::Accept { id }),
        Err(_) => propose(),
    };
    let mut req = request(&kb, verb);
    req.probe = abort_at;
    writfold_kb::run(&req)?;
    Ok(())
}

fn die(kb: &Path, step: Step, accept: Option<&str>) -> TestResult {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
        .env(CHILD, kb)
        .env("WRITFOLD_KB_FAULT_STEP", step.name())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if let Some(id) = accept {
        command.env("WRITFOLD_KB_FAULT_ID", id);
    }
    let status = command.status()?;
    if status.success() {
        return Err(format!("the child did not stop at {}", step.name()).into());
    }
    Ok(())
}

/// The queue's files: complete proposals, and dot-named leftovers.
fn queue(kb: &Path) -> TestResult<(usize, usize)> {
    let dir = kb.join(".git/writfold/proposals");
    let (mut complete, mut leftovers) = (0, 0);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok((0, 0));
    };
    for entry in entries {
        let name = entry?.file_name().to_string_lossy().into_owned();
        match name.strip_suffix(".json") {
            Some(id) if !name.starts_with('.') => {
                proposal::load(&kb.join(".git/writfold"), id)?.ok_or("readable")?;
                complete += 1;
            }
            _ if name.starts_with('.') && name.ends_with(".tmp") => leftovers += 1,
            _ => return Err(format!("unexpected queue file {name}").into()),
        }
    }
    Ok((complete, leftovers))
}

#[test]
fn propose_dies_at_every_step() -> TestResult {
    for step in [Step::TempWritten, Step::TempSynced, Step::Published] {
        let w = world(&[])?;
        die(&w.kb, step, None)?;
        let (complete, _) = queue(&w.kb)?;
        assert_eq!(
            complete,
            usize::from(step == Step::Published),
            "{}",
            step.name()
        );
        assert_eq!(support::git(&w.kb, &["status", "--porcelain"])?, "");
        let output = writfold_kb::run(&request(&w.kb, propose()))?;
        assert_eq!(output.exit, 0, "{}", output.text);
        assert_eq!(queue(&w.kb)?.0, 1, "{}", step.name());
    }
    Ok(())
}

#[test]
fn accept_dies_at_every_step() -> TestResult {
    for step in Step::ALL {
        let w = world(&[])?;
        writfold_kb::run(&request(&w.kb, propose()))?;
        let id = proposal::all(&w.kb.join(".git/writfold"))
            .first()
            .map(|p| p.id.clone())
            .ok_or("a proposal")?;
        die(&w.kb, step, Some(&id))?;
        let written = std::fs::read_to_string(w.kb.join(TICKET)).ok();
        let (complete, _) = queue(&w.kb)?;
        assert_eq!(
            complete,
            1,
            "the proposal outlives a death at {}",
            step.name()
        );
        if let Some(text) = &written {
            assert!(text.contains("# HBR-130: Replay is slow"), "{text}");
        }
        let verify = writfold_kb::run(&request(&w.kb, Verb::IndexVerify))?;
        assert_eq!(verify.exit, 0, "{}", verify.text);
        let rerun = writfold_kb::run(&request(&w.kb, Verb::Proposal(ProposalVerb::Accept { id })));
        match (written.is_some(), rerun) {
            (false, Ok(output)) => {
                assert_eq!(output.exit, 0);
                assert_eq!(queue(&w.kb)?.0, 0);
            }
            (true, Err(error)) => {
                assert_eq!(error.exit_code(), 3, "{error}");
                assert_eq!(queue(&w.kb)?.0, 1);
            }
            (published, other) => {
                return Err(format!(
                    "{}: published={published} rerun ok={}",
                    step.name(),
                    other.is_ok()
                )
                .into());
            }
        }
    }
    Ok(())
}
