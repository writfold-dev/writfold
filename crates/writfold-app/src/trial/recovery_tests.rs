//! A crash after `Claimed` and before an outcome, then `recover`.
use super::apply::append;
use super::flow::{Event, Premises};
use super::{Result, Trial, apply, approve, prepare, recover, stream};
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const SOURCE: &str = "fn main() {\n    run(1);\n}\n";
const RECIPE: &str =
    "edit \"src/main.rs\" {\n  @n := one text \"run(1)\"\n  replace @n with \"run(2)\"\n}\n";

struct Run {
    root: PathBuf,
    state: PathBuf,
    run: String,
    file: PathBuf,
}
impl Drop for Run {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<()> {
    let ok = Command::new("git")
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args(args)
        .status()?
        .success();
    if ok {
        Ok(())
    } else {
        Err(format!("git {args:?} failed").into())
    }
}

/// Prepared and approved, then `Claimed` appended with no install: the state
/// a crash between claim and outcome leaves behind.
fn claimed() -> Result<Run> {
    let root = std::env::temp_dir().join(format!(
        ".writfold-recover-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("src"))?;
    fs::write(repo.join("src/main.rs"), SOURCE)?;
    git(&repo, &["init", "-q"])?;
    git(&repo, &["add", "-A"])?;
    git(&repo, &["commit", "-qm", "fixture"])?;
    fs::write(root.join("r.spl"), RECIPE)?;
    let state = root.join("state");
    Trial::init(&state)?;
    let p = prepare(&state, &repo, "src/main.rs", &root.join("r.spl"), None)?;
    approve(&state, &p.run)?;
    let trial = Trial::open(&state)?;
    let executable = trial.executable()?;
    let lease = trial.writer.try_lease()?;
    let ledger = trial.ledger(&lease, &executable)?;
    let loaded = ledger.load::<Event, Premises>(&stream(&p.run)?)?;
    let [first, second] = loaded.records.as_slice() else {
        return Err("expected two records".into());
    };
    let Event::Prepared { candidate, .. } = first.payload else {
        return Err("expected prepared".into());
    };
    append(
        &ledger,
        &p.run,
        second,
        3,
        "claimed",
        Event::Claimed { candidate },
    )?;
    Ok(Run {
        file: p.worktree.join("src/main.rs"),
        root,
        state,
        run: p.run,
    })
}

fn outcome(r: &Run) -> Result<Event> {
    let trial = Trial::open(&r.state)?;
    let executable = trial.executable()?;
    let lease = trial.writer.try_lease()?;
    let loaded = trial
        .ledger(&lease, &executable)?
        .load::<Event, Premises>(&stream(&r.run)?)?;
    loaded
        .records
        .last()
        .map(|x| x.payload.clone())
        .ok_or_else(|| "empty".into())
}

#[test]
fn a_claim_whose_install_happened_reconciles_to_applied() -> Result<()> {
    let r = claimed()?;
    assert!(
        apply(&r.state, &r.run).is_err(),
        "apply must not re-dispatch a claim"
    );
    fs::write(&r.file, SOURCE.replace("run(1)", "run(2)"))?;
    recover(&r.state, &r.run)?;
    assert!(matches!(
        outcome(&r)?,
        Event::Applied {
            reconciled: true,
            ..
        }
    ));
    assert!(recover(&r.state, &r.run).is_err(), "an outcome is final");
    Ok(())
}

#[test]
fn a_claim_whose_install_never_happened_is_abandoned() -> Result<()> {
    let r = claimed()?;
    recover(&r.state, &r.run)?;
    assert!(matches!(outcome(&r)?, Event::Abandoned { .. }));
    assert_eq!(fs::read_to_string(&r.file)?, SOURCE);
    Ok(())
}

#[test]
fn a_claim_over_unexpected_bytes_stays_unknown() -> Result<()> {
    let r = claimed()?;
    fs::write(&r.file, "something else\n")?;
    assert!(recover(&r.state, &r.run).is_err());
    assert!(matches!(outcome(&r)?, Event::Claimed { .. }));
    assert_eq!(fs::read_to_string(&r.file)?, "something else\n");
    Ok(())
}
