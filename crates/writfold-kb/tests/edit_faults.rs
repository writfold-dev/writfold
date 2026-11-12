//! Kill tests of `edit apply`: the test binary re-runs itself as a child
//! that applies a two-file batch (recording `--worktree` in the run, so the
//! sidecar write is covered too) and aborts at the Nth step of the write
//! protocol, for every N until the child completes. After each death:
//!
//! - every file holds its complete before or after bytes;
//! - with no journal, either nothing was applied and there is no receipt,
//!   or everything was applied and there is exactly one receipt;
//! - with a journal, a new `edit apply` is refused, and `edit recover`
//!   finishes the batch: both files after, one receipt, the worktree
//!   recorded, no temporary left, and the index equal to a rebuild.
//!
//! At least one death lands between the two files' renames.
mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use support::{NOW, TestResult, World, world};
use writfold_kb::edit::EditVerb;
use writfold_kb::error::KbError;
use writfold_kb::write::Step;
use writfold_kb::{Request, Verb};

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const CHILD: &str = "WRITFOLD_KB_EDIT_FAULT_CHILD";
const AT: &str = "WRITFOLD_KB_EDIT_FAULT_AT";
const PATHS: [&str; 2] = ["src/retry.rs", "src/ledger.rs"];
const RETRY: &str =
    "edit \"src/retry.rs\" {\n  replace one text \"attempts = 5\" with \"attempts = 7\"\n}\n";
const LEDGER: &str =
    "edit \"src/ledger.rs\" {\n  replace one text \"append(row);\" with \"append(row)?;\"\n}\n";

static CALLS: AtomicUsize = AtomicUsize::new(0);

fn abort_at(_: Step) {
    let at: usize = std::env::var(AT)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if CALLS.fetch_add(1, Ordering::SeqCst) + 1 == at {
        std::process::abort();
    }
}

fn request(base: &Path, verb: EditVerb) -> Request {
    Request {
        verb: Verb::Edit(verb),
        kb: Some(base.join("kb")),
        cwd: base.to_path_buf(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(NOW),
        probe: writfold_kb::write::no_probe,
    }
}

fn apply(base: &Path) -> EditVerb {
    EditVerb::Apply {
        recipes: vec![base.join("retry.spl"), base.join("ledger.spl")],
        run: Some(RUN.into()),
        worktree: Some(base.join("demo-service")),
        dry_run: false,
    }
}

/// The child: one apply that aborts at the Nth probe call.
#[test]
fn child() -> TestResult {
    let Ok(base) = std::env::var(CHILD) else {
        return Ok(());
    };
    let base = PathBuf::from(base);
    let mut req = request(&base, apply(&base));
    req.probe = abort_at;
    writfold_kb::run(&req)?;
    Ok(())
}

/// Which files hold their after bytes; panics on any other content.
fn applied(w: &World, before: &[Vec<u8>], after: &[Vec<u8>]) -> TestResult<Vec<bool>> {
    let mut out = Vec::new();
    for (i, path) in PATHS.iter().enumerate() {
        let now = std::fs::read(w.code.join(path))?;
        assert!(now == before[i] || now == after[i], "{path} is torn");
        out.push(now == after[i]);
    }
    Ok(out)
}

fn receipts(w: &World) -> usize {
    std::fs::read_dir(w.kb.join(format!("runs/{RUN}/edits")))
        .map(|d| {
            d.flatten()
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .count()
        })
        .unwrap_or(0)
}

fn temporaries(w: &World) -> TestResult<usize> {
    let mut n = 0;
    for entry in std::fs::read_dir(w.code.join("src"))? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            assert!(
                name.starts_with(".writfold-edit-") && name.ends_with(".tmp"),
                "{name}"
            );
            n += 1;
        }
    }
    Ok(n)
}

/// The before and after bytes of each file.
type Bytes = (Vec<Vec<u8>>, Vec<Vec<u8>>);

fn setup(w: &World) -> TestResult<Bytes> {
    std::fs::write(w.base.join("retry.spl"), RETRY)?;
    std::fs::write(w.base.join("ledger.spl"), LEDGER)?;
    let before: Vec<Vec<u8>> = PATHS
        .iter()
        .map(|p| std::fs::read(w.code.join(p)))
        .collect::<Result<_, _>>()?;
    let after = vec![
        String::from_utf8(before[0].clone())?
            .replace("attempts = 5", "attempts = 7")
            .into_bytes(),
        String::from_utf8(before[1].clone())?
            .replace("append(row);", "append(row)?;")
            .into_bytes(),
    ];
    Ok((before, after))
}

/// One death at the Nth step; `None` once the child no longer dies.
fn one(at: usize, between: &mut usize) -> TestResult<Option<()>> {
    let w = world(&[])?;
    let (before, after) = setup(&w)?;
    let status = Command::new(std::env::current_exe()?)
        .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
        .env(CHILD, &w.base)
        .env(AT, at.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    if status.success() {
        assert_eq!(applied(&w, &before, &after)?, [true, true]);
        assert_eq!(receipts(&w), 1);
        return Ok(None);
    }
    let state = applied(&w, &before, &after)?;
    let journal = w.kb.join(format!(".git/writfold/edits/{RUN}.json"));
    if !journal.exists() {
        let all = state == [true, true];
        assert!(
            all || state == [false, false],
            "at {at}: {state:?} with no journal"
        );
        assert_eq!(receipts(&w), usize::from(all), "at {at}");
        assert_eq!(temporaries(&w)?, 0, "at {at}");
        return Ok(Some(()));
    }
    if state == [true, false] {
        *between += 1;
    }
    match writfold_kb::run(&request(&w.base, apply(&w.base))) {
        Err(KbError::Refused(why)) => assert!(why.contains("edit recover"), "{why}"),
        other => {
            return Err(
                format!("apply with a pending journal: {:?}", other.map(|o| o.text)).into(),
            );
        }
    }
    let recover = EditVerb::Recover {
        run: Some(RUN.into()),
        abandon: false,
    };
    let output = writfold_kb::run(&request(&w.base, recover))?;
    assert_eq!(output.exit, 0, "at {at}: {}", output.text);
    assert_eq!(applied(&w, &before, &after)?, [true, true], "at {at}");
    assert_eq!(receipts(&w), 1, "at {at}");
    assert_eq!(temporaries(&w)?, 0, "at {at}");
    assert!(!journal.exists());
    let sidecar = std::fs::read_to_string(w.kb.join(format!("runs/{RUN}/writfold-run.json")))?;
    assert!(sidecar.contains("demo-service"), "at {at}: {sidecar}");
    let (verify, exit) = w.run(&w.request(Verb::IndexVerify))?;
    assert!(exit == 0 && !verify.contains("unreadable"), "{verify}");
    Ok(Some(()))
}

#[test]
fn every_step_of_a_two_file_apply_recovers() -> TestResult {
    let mut between = 0;
    let mut at = 1;
    while one(at, &mut between)?.is_some() {
        at += 1;
        assert!(at < 64, "the child never completed");
    }
    // locked, journaled, 2 x (temp-written, temp-synced), 2 x (rechecked,
    // published, dir-synced), receipt and sidecar 2 x 5, journal-cleared,
    // unlocked.
    assert_eq!(at, 25, "steps seen");
    assert!(between >= 3, "no death landed between the two files");
    Ok(())
}

#[test]
fn recover_refuses_a_file_changed_after_the_crash_and_abandon_drops_it() -> TestResult {
    let w = world(&[])?;
    let (before, after) = setup(&w)?;
    // Die right after the first file's rename (locked, journaled, four
    // temp steps, rechecked, published).
    let status = Command::new(std::env::current_exe()?)
        .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
        .env(CHILD, &w.base)
        .env(AT, "8")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    assert!(!status.success());
    assert_eq!(applied(&w, &before, &after)?, [true, false]);
    std::fs::write(w.code.join(PATHS[1]), "// someone else\n")?;
    let recover = |abandon| EditVerb::Recover {
        run: Some(RUN.into()),
        abandon,
    };
    match writfold_kb::run(&request(&w.base, recover(false))) {
        Err(KbError::Refused(why)) => assert!(why.contains("src/ledger.rs hold neither"), "{why}"),
        other => return Err(format!("expected a refusal: {:?}", other.map(|o| o.text)).into()),
    }
    let output = writfold_kb::run(&request(&w.base, recover(true)))?;
    assert!(
        output.text.contains("edit src/retry.rs [applied]"),
        "{}",
        output.text
    );
    assert!(
        output.text.contains("edit src/ledger.rs [changed]"),
        "{}",
        output.text
    );
    assert_eq!(receipts(&w), 0);
    assert_eq!(temporaries(&w)?, 0);
    assert!(
        !w.kb
            .join(format!(".git/writfold/edits/{RUN}.json"))
            .exists()
    );
    Ok(())
}
