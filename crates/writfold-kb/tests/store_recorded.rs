//! #4 (U3): `kb stale --record` records the verdicts of pinned store
//! citations judged on a machine that has the store's clone; a machine
//! without it reads them instead of `unverifiable: store-not-local`, only
//! when the store declares `recorded_max_age` and the receipt is within
//! it. A verdict this machine reaches itself always wins.
mod support;

use std::path::PathBuf;
use support::{NOW, TestResult, World, git, world};
use writfold_kb::Verb;

/// A store with one probe, its clone, a declaration (with `extra` lines)
/// whose hint names the clone, and a KB note citing the probe at its pin.
fn stored(extra: &str) -> TestResult<(World, PathBuf)> {
    let w = world(&[])?;
    let origin = w.base.join("scratch");
    std::fs::create_dir_all(origin.join("work/probe"))?;
    git(&origin, &["init", "-q", "-b", "main"])?;
    git(&origin, &["config", "user.email", "t@example.invalid"])?;
    git(&origin, &["config", "user.name", "t"])?;
    std::fs::write(origin.join("work/probe/run.sh"), "echo probe\n")?;
    git(&origin, &["add", "-A"])?;
    git(&origin, &["commit", "-q", "-m", "probe"])?;
    let clone = w.base.join("scratch-clone");
    let url = format!("file://{}", origin.display());
    git(
        &w.base,
        &["clone", "-q", &url, &clone.display().to_string()],
    )?;
    std::fs::create_dir_all(w.kb.join(".writfold/stores"))?;
    let text = format!(
        "schema = \"writfold-store/1\"\nname = \"scratch\"\nurl = \"{url}\"\nrole = \"attached\"\npaths = [\"{}\"]\n{extra}",
        clone.display()
    );
    std::fs::write(w.kb.join(".writfold/stores/scratch.toml"), text)?;
    let rev = git(&clone, &["rev-parse", "HEAD"])?;
    let blob = git(&clone, &["rev-parse", "HEAD:work/probe/run.sh"])?;
    let note = format!(
        "# Probe\n\nThe probe ⟦cite kind=code repo=scratch rev={rev} path=work/probe/run.sh L=1-1 blob={blob}⟧.\n"
    );
    std::fs::write(w.kb.join("lantern/probe.md"), note)?;
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", "probe"])?;
    Ok((w, clone))
}

fn stale(w: &World, record: bool, at: i64) -> TestResult<String> {
    let scope = Some("lantern/probe.md".to_owned());
    let verb = if record {
        Verb::StaleRecord { scope }
    } else {
        Verb::Stale {
            scope,
            offline: false,
            cached: false,
        }
    };
    let mut request = w.request(verb);
    request.now = Some(at);
    Ok(w.run(&request)?.0)
}

/// The other machine: the clone is not where the hint says.
fn away(clone: &PathBuf) -> TestResult<PathBuf> {
    let moved = clone.with_extension("away");
    std::fs::rename(clone, &moved)?;
    Ok(moved)
}

#[test]
fn a_machine_without_the_clone_reads_the_recorded_verdict() -> TestResult {
    let (w, clone) = stored("recorded_max_age = \"30d\"\n")?;
    let text = stale(&w, true, NOW)?;
    assert!(
        text.contains("recorded 1 verdicts of kind store@scratch"),
        "{text}"
    );
    let dir = w.kb.join(".writfold/checks/store@scratch");
    assert_eq!(std::fs::read_dir(&dir)?.count(), 1);
    let moved = away(&clone)?;
    // Within recorded_max_age: fresh, marked as recorded.
    let text = stale(&w, false, NOW + 86_400)?;
    assert!(
        text.contains(" unverifiable=0 ") && text.contains(" recorded=1"),
        "{text}"
    );
    // Past it: unverifiable again, as before.
    let text = stale(&w, false, NOW + 31 * 86_400)?;
    assert!(
        text.contains("store-not-local") && text.contains(" unverifiable=1"),
        "{text}"
    );
    // The clone back: this machine's own verdict, no receipt read.
    std::fs::rename(&moved, &clone)?;
    let text = stale(&w, false, NOW + 86_400)?;
    assert!(
        text.contains(" unverifiable=0") && !text.contains("recorded="),
        "{text}"
    );
    Ok(())
}

#[test]
fn without_recorded_max_age_nothing_is_recorded_or_read() -> TestResult {
    let (w, clone) = stored("")?;
    let text = stale(&w, true, NOW)?;
    assert!(!text.contains("store@scratch"), "{text}");
    assert!(!w.kb.join(".writfold/checks/store@scratch").exists());
    // A receipt recorded under the key is ignored once the key is gone.
    let (w2, clone2) = stored("recorded_max_age = \"30d\"\n")?;
    stale(&w2, true, NOW)?;
    let decl = w2.kb.join(".writfold/stores/scratch.toml");
    let text = std::fs::read_to_string(&decl)?.replace("recorded_max_age = \"30d\"\n", "");
    std::fs::write(&decl, text)?;
    away(&clone2)?;
    let text = stale(&w2, false, NOW + 60)?;
    assert!(
        text.contains("store-not-local") && !text.contains("recorded="),
        "{text}"
    );
    drop(clone);
    Ok(())
}

#[test]
fn a_head_store_refuses_recorded_max_age() -> TestResult {
    let (w, _) = stored("freshness = \"head\"\nrecorded_max_age = \"30d\"\n")?;
    let refused = writfold_kb::run(&w.request(Verb::Stale {
        scope: None,
        offline: false,
        cached: false,
    }));
    assert!(
        refused.is_err_and(|e| e.to_string().contains("recorded_max_age needs freshness")),
        "a head store with recorded_max_age"
    );
    Ok(())
}
