//! Kill tests of the plan's three writes: the test binary re-runs itself
//! as a child that runs `plan set`, `plan advance` or `plan skip` and
//! aborts at each step of the write protocol. After every death each event
//! file is absent or complete and valid (a splice never without its
//! snapshot), leftovers are only dot-named `.tmp` files, the index equals
//! a rebuild, and a rerun completes the write.
mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use support::{NOW, TestResult, world};
use writfold_kb::plan::PlanVerb;
use writfold_kb::plan::event;
use writfold_kb::write::Step;
use writfold_kb::{Request, Verb};

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const CHILD: &str = "WRITFOLD_KB_PLAN_FAULT_CHILD";
const VERB: &str = "WRITFOLD_KB_PLAN_FAULT_VERB";
const TEMPLATE: &str =
    "schema = \"writfold-plan/1\"\nname = \"two\"\nsteps = [\"first:\", \"second:\"]\n";

fn verb(name: &str) -> PlanVerb {
    let run = Some(RUN.to_owned());
    match name {
        "set" => PlanVerb::Set {
            template: Some("two".into()),
            from: None,
            run,
            repo: None,
        },
        "skip" => PlanVerb::Skip {
            reason: "not needed".into(),
            by: "owner".into(),
            run,
            repo: None,
        },
        "status" => PlanVerb::Status { run, repo: None },
        _ => PlanVerb::Advance { run, repo: None },
    }
}

fn request(kb: &Path, name: &str) -> Request {
    Request {
        verb: Verb::Plan(verb(name)),
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

/// The child: one plan write that aborts at a step. Does nothing unless
/// the parent set the environment.
#[test]
fn child() -> TestResult {
    let (Ok(kb), Ok(name)) = (std::env::var(CHILD), std::env::var(VERB)) else {
        return Ok(());
    };
    let mut req = request(&PathBuf::from(kb), &name);
    req.probe = abort_at;
    writfold_kb::run(&req)?;
    Ok(())
}

/// Every event file is valid, every snapshot matches its name; returns
/// the event file names.
fn valid_events(kb: &Path) -> TestResult<Vec<String>> {
    let dir = kb.join(format!("runs/{RUN}/plan"));
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&dir).into_iter().flatten() {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            assert!(name.ends_with(".tmp"), "{name}");
            continue;
        }
        if name == event::TEMPLATES {
            continue;
        }
        let text = std::fs::read_to_string(entry.path())?;
        event::parse(&text, RUN, &name).map_err(|why| format!("{name}: {why}"))?;
        found.push(name);
    }
    found.sort();
    if found.iter().any(|n| n.starts_with("splice-")) {
        let snapshots = std::fs::read_dir(dir.join(event::TEMPLATES))?.count();
        assert_eq!(snapshots, 1, "a splice without its snapshot");
    }
    Ok(found)
}

#[test]
fn every_fault_point_leaves_no_event_or_a_valid_one() -> TestResult {
    for name in ["set", "advance", "skip"] {
        for step in Step::ALL {
            let w = world(&[])?;
            let plans = w.kb.join(".writfold/plans");
            std::fs::create_dir_all(&plans)?;
            std::fs::write(plans.join("two.toml"), TEMPLATE)?;
            if name != "set" {
                assert_eq!(w.run(&request(&w.kb, "set"))?.1, 0);
            }
            let before = valid_events(&w.kb)?.len();
            let status = Command::new(std::env::current_exe()?)
                .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
                .env(CHILD, &w.kb)
                .env(VERB, name)
                .env("WRITFOLD_KB_FAULT_STEP", step.name())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()?;
            let at = format!("plan {name} at {}", step.name());
            assert!(!status.success(), "{at} did not stop");
            let after = valid_events(&w.kb)?.len();
            let published = matches!(step, Step::Published | Step::DirSynced | Step::Unlocked);
            // `plan set` publishes its snapshot first, so the first
            // occurrence of every step but `unlocked` is the snapshot's.
            let expected = match name {
                "set" => step == Step::Unlocked,
                _ => published,
            };
            assert_eq!(after, before + usize::from(expected), "{at}");
            let (verify, exit) = w.run(&w.request(Verb::IndexVerify))?;
            assert!(
                exit == 0 && !verify.contains("unreadable"),
                "{at}: {verify}"
            );
            let (text, exit) = w.run(&request(&w.kb, name))?;
            assert_eq!(exit, 0, "rerun after {at}:\n{text}");
            let (text, _) = w.run(&request(&w.kb, "status"))?;
            assert!(text.contains("step two#1/second ["), "{at}: {text}");
            assert!(!text.contains("!invalid"), "{at}: {text}");
        }
    }
    Ok(())
}
