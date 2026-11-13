//! The re-check under the write lock: two sessions in one clone. Session
//! A folds the plan and decides; session B's write lands between A's read
//! and A's lock (injected by the write protocol's probe at `locked`, the
//! first point under the lock, before any guard is checked). A must then
//! refuse (exit 3) and write nothing, for each race the run-graph model
//! found: a splice against a done record, two splices decided at one
//! ordinal, two advances, a skip against a splice, two plans set.
mod support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::plan::PlanVerb;
use writfold_kb::write::Step;

const RUN: &str = "run-20261101T090000Z-a1b2c3";

/// Files the probe publishes once, just after the lock is taken. The
/// probe is a plain function, so this is global: [`SERIAL`] keeps one
/// probed write at a time across this binary's test threads.
static INJECT: Mutex<Vec<(PathBuf, Vec<u8>)>> = Mutex::new(Vec::new());
static SERIAL: Mutex<()> = Mutex::new(());

fn inject(step: Step) {
    if step != Step::Locked {
        return;
    }
    let files = INJECT.lock().map(|mut i| std::mem::take(&mut *i));
    for (path, bytes) in files.unwrap_or_default() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, bytes);
    }
}

fn template(w: &World, name: &str, steps: &[&str]) -> TestResult {
    let list: Vec<String> = steps.iter().map(|s| format!("\"{s}\"")).collect();
    let text = format!(
        "schema = \"writfold-plan/1\"\nname = \"{name}\"\nsteps = [{}]\n",
        list.join(", ")
    );
    let dir = w.kb.join(".writfold/plans");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(format!("{name}.toml")), text)?;
    Ok(())
}

/// `set NAME`, `use NAME`, `advance`, `skip`, `status`.
fn verb(command: &str) -> PlanVerb {
    let run = Some(RUN.to_owned());
    let (head, name) = command.split_once(' ').unwrap_or((command, ""));
    match head {
        "set" => PlanVerb::Set {
            template: Some(name.into()),
            from: None,
            run,
            repo: None,
        },
        "use" => PlanVerb::Use {
            template: name.into(),
            again: false,
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

fn plan(w: &World, command: &str) -> TestResult<(String, u8)> {
    w.run(&w.request(Verb::Plan(verb(command))))
}

/// The run's plan files (events and snapshots) with their bytes.
fn files(w: &World) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
    fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) -> TestResult {
        for entry in std::fs::read_dir(dir).into_iter().flatten() {
            let path = entry?.path();
            if path.is_dir() {
                walk(&path, out)?;
            } else {
                out.insert(path.clone(), std::fs::read(&path)?);
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    walk(&w.kb.join(format!("runs/{RUN}/plan")), &mut out)?;
    Ok(out)
}

/// Run `theirs` (session B) to learn the files it writes, take them back
/// out, then run `mine` (session A) with B's files landing between A's
/// read and its lock. Returns A's refusal.
fn race(w: &World, mine: &str, theirs: &[&str]) -> TestResult<String> {
    let before = files(w)?;
    for command in theirs {
        assert_eq!(plan(w, command)?.1, 0, "{command}");
    }
    let mut written: Vec<(PathBuf, Vec<u8>)> = files(w)?
        .into_iter()
        .filter(|(p, _)| !before.contains_key(p))
        .collect();
    written.sort();
    assert!(!written.is_empty(), "{theirs:?} wrote nothing");
    for (path, _) in &written {
        std::fs::remove_file(path)?;
    }
    assert_eq!(files(w)?, before, "B's files taken back out");
    let serial = SERIAL.lock().map_err(|_| "a probed test panicked")?;
    *INJECT.lock().map_err(|_| "poisoned")? = written.clone();
    let mut request = w.request(Verb::Plan(verb(mine)));
    request.probe = inject;
    let result = writfold_kb::run(&request);
    drop(serial);
    let refused = match result {
        Ok(out) => return Err(format!("{mine} was not refused:\n{}", out.text).into()),
        Err(e) => e,
    };
    assert_eq!(refused.exit_code(), 3, "{refused}");
    let mut expected = before;
    expected.extend(written);
    assert_eq!(files(w)?, expected, "{mine} wrote nothing");
    let (status, _) = plan(w, "status")?;
    assert!(!status.contains("!out-of-order"), "{status}");
    Ok(refused.to_string())
}

fn planned() -> TestResult<World> {
    let w = world(&[])?;
    template(&w, "two", &["first:", "second:"])?;
    template(&w, "side", &["only:"])?;
    template(&w, "other", &["x:"])?;
    assert_eq!(plan(&w, "set two")?.1, 0);
    Ok(w)
}

const CHANGED: &str = "the plan changed since this command read it";

#[test]
fn a_splice_decided_before_a_done_record_is_refused() -> TestResult {
    let w = planned()?;
    let why = race(&w, "use side", &["advance"])?;
    assert!(
        why.contains(CHANGED) && why.contains("two#1/first is no longer the current step"),
        "{why}"
    );
    Ok(())
}

#[test]
fn a_done_record_decided_before_a_splice_is_refused() -> TestResult {
    let w = planned()?;
    let why = race(&w, "advance", &["use side"])?;
    assert!(why.contains(CHANGED), "{why}");
    Ok(())
}

/// B splices `other` under the step and skips its one step, so the step
/// is current again; only the ordinal clause sees that A's ordinal 1 is
/// taken (the model's `probe-RecheckCurrentOnly`).
#[test]
fn two_splices_decided_at_one_ordinal_are_refused() -> TestResult {
    let w = planned()?;
    let why = race(&w, "use side", &["use other", "skip"])?;
    assert!(
        why.contains(CHANGED) && why.contains("another splice under two#1/first took ordinal 1"),
        "{why}"
    );
    let w = planned()?;
    let why = race(&w, "use side", &["use other"])?;
    assert!(why.contains(CHANGED), "{why}");
    Ok(())
}

#[test]
fn two_advances_of_one_step_are_refused() -> TestResult {
    let w = planned()?;
    let why = race(&w, "advance", &["advance"])?;
    assert!(why.contains(CHANGED), "{why}");
    Ok(())
}

#[test]
fn a_skip_decided_before_a_splice_is_refused() -> TestResult {
    let w = planned()?;
    let why = race(&w, "skip", &["use side"])?;
    assert!(why.contains(CHANGED), "{why}");
    let w = planned()?;
    let why = race(&w, "use side", &["skip"])?;
    assert!(why.contains(CHANGED), "{why}");
    Ok(())
}

#[test]
fn two_plans_set_in_one_clone_are_refused() -> TestResult {
    let w = world(&[])?;
    template(&w, "two", &["first:", "second:"])?;
    template(&w, "other", &["x:"])?;
    let why = race(&w, "set two", &["set other"])?;
    assert!(
        why.contains(CHANGED) && why.contains("a plan was set meanwhile"),
        "{why}"
    );
    Ok(())
}

/// Without a competing write the guard passes: the plan is written.
#[test]
fn an_undisturbed_write_passes_the_recheck() -> TestResult {
    let w = planned()?;
    let mut request = w.request(Verb::Plan(verb("use side")));
    request.probe = inject;
    let serial = SERIAL.lock().map_err(|_| "a probed test panicked")?;
    let exit = w.run(&request)?.1;
    drop(serial);
    assert_eq!(exit, 0);
    let (status, _) = plan(&w, "status")?;
    assert!(
        status.contains("step two#1/first/side#1/only [current]"),
        "{status}"
    );
    Ok(())
}
