//! `plan repin` under the write lock: another session's plan write lands
//! between the repin's read and its lock (injected by the write
//! protocol's probe at `locked`). The repin holds what its fold showed, so
//! it is then refused (exit 3) and writes nothing.
mod checklist;
mod repin;
mod support;

use checklist::{RUN, put};
use repin::{START, STRICTER, pins, plan, set_up, verb};
use std::path::PathBuf;
use std::sync::Mutex;
use support::{TestResult, World};
use writfold_kb::Verb;
use writfold_kb::write::Step;

/// Files the probe publishes once, just after the lock is taken.
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

/// The files under the run of `other` that `w` lacks, placed in `w`.
fn newer(w: &World, other: &World) -> TestResult<Vec<(PathBuf, Vec<u8>)>> {
    let mut out = Vec::new();
    let mut dirs = vec![PathBuf::from(format!("runs/{RUN}"))];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(other.kb.join(&dir))?.flatten() {
            let path = dir.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                dirs.push(path);
            } else if !w.kb.join(&path).exists() {
                out.push((w.kb.join(&path), std::fs::read(entry.path())?));
            }
        }
    }
    Ok(out)
}

/// A repin in `w` while another session's `write` (made in the twin
/// `other`) lands between its read and its lock: refused, nothing written.
fn raced(w: &World, other: &World, write: &str, why: &str) -> TestResult {
    let _serial = SERIAL.lock().map_err(|e| e.to_string())?;
    let (text, exit) = plan(other, write)?;
    assert!(exit == 0 || exit == 2, "{text}");
    let before = pins(w)?.len();
    *INJECT.lock().map_err(|e| e.to_string())? = newer(w, other)?;
    let mut request = w.request(Verb::Plan(verb(w, "repin")));
    request.probe = inject;
    let error = writfold_kb::run(&request)
        .err()
        .ok_or("the repin was written")?;
    assert_eq!(error.exit_code(), 3, "{error}");
    let text = error.to_string();
    assert!(
        text.contains("the plan changed since this command read it"),
        "{text}"
    );
    assert!(text.contains(why), "{text}");
    let after = (pins(w)?.iter())
        .filter(|(_, p)| p["kind"] == "repin")
        .count();
    assert_eq!((before, after), (1, 0), "no repin is written");
    Ok(())
}

#[test]
fn a_repin_is_rechecked_under_the_lock() -> TestResult {
    // The current step is closed meanwhile.
    let twins = || -> TestResult<(World, World)> {
        let (w, other) = (set_up(START)?, set_up(START)?);
        for world in [&w, &other] {
            plan(world, "set")?;
            put(&world.kb.join(".writfold/gates/note.md"), STRICTER)?;
        }
        Ok((w, other))
    };
    let (w, other) = twins()?;
    raced(
        &w,
        &other,
        "skip",
        "research#1/research is no longer the current step",
    )?;
    // Every step was closed, and a splice (decided in the twin before its
    // last step closed) opens one meanwhile.
    let edited = format!("{START}# reviewed\n");
    let (w, other) = twins()?;
    for (world, skips) in [(&w, 3), (&other, 2)] {
        for _ in 0..skips {
            plan(world, "skip")?;
        }
        put(&world.kb.join(".writfold/plans/required.toml"), &edited)?;
    }
    let (text, _) = plan(&w, "status")?;
    assert!(text.contains("policy=moved"), "{text}");
    raced(&w, &other, "use", "a step is open again")?;
    // Without a race a repin decided with no current step lands.
    let (w, _) = twins()?;
    for _ in 0..3 {
        plan(&w, "skip")?;
    }
    put(&w.kb.join(".writfold/plans/required.toml"), &edited)?;
    let (text, exit) = plan(&w, "repin")?;
    assert!(
        text.contains("repinned=1") && text.contains("policy=same"),
        "{exit} {text}"
    );
    Ok(())
}
