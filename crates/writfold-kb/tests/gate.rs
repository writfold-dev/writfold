//! G1 gates over the fixture world: definitions in the KB root and in the
//! subject repository (the fixture's `demo-service`), receipts recorded by
//! `gate run`, state derived by `gate status` and `resume`, `gate verify`,
//! and `gate lines`. Commands are trivial (`true`, `sh -c …`, `sleep`).
mod support;

use std::path::{Path, PathBuf};
use support::{NOW, TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::gate::GateVerb;

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn define(dir: &Path, id: &str, argv: &[&str], scope: &[&str], timeout: u32) -> TestResult {
    let gates = dir.join(".writfold/gates");
    std::fs::create_dir_all(&gates)?;
    let list = |items: &[&str]| serde_json::to_string(items).unwrap_or_default();
    let text = format!(
        "# a test gate\nschema = \"writfold-gate/1\"\nid = \"{id}\"\nversion = 1\nclass = \"mechanical\"\nargv = {}\ntimeout_seconds = {timeout}\nscope = {}\n",
        list(argv),
        list(scope)
    );
    std::fs::write(gates.join(format!("{id}.toml")), text)?;
    Ok(())
}

/// Receipt names end in six random hex digits; durations vary.
fn stable(text: &str) -> String {
    let text = text.replace(RUN, "<RUN>");
    let names = regex::Regex::new(r"-(\d{8}T\d{6}Z)-[0-9a-f]{6}").ok();
    let text = names.map_or(text.clone(), |re| {
        re.replace_all(&text, "-$1-XXXXXX").into_owned()
    });
    let text = text.replace("<RUN>", RUN);
    let ms = regex::Regex::new(r"in \d+ ms").ok();
    ms.map_or(text.clone(), |re| {
        re.replace_all(&text, "in N ms").into_owned()
    })
}

fn golden(name: &str, text: &str, exit: u8) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let actual = format!("{}# exit={exit}\n", stable(text));
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(&file, &actual)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&file).map_err(|e| format!("{name}: {e}"))?;
    if expected != actual {
        return Err(
            format!("golden {name} differs\n--- expected\n{expected}--- actual\n{actual}").into(),
        );
    }
    Ok(())
}

fn gate(w: &World, verb: GateVerb, at: i64) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Gate(verb));
    request.now = Some(at);
    w.run(&request)
}

fn run(w: &World, id: &str, at: i64) -> TestResult<(String, u8)> {
    gate(
        w,
        GateVerb::Run {
            id: id.into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
        at,
    )
}

fn status(w: &World) -> TestResult<(String, u8)> {
    gate(
        w,
        GateVerb::Status {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
        NOW + 3600,
    )
}

/// The state `gate status` gives one gate.
fn state_of(w: &World, id: &str) -> TestResult<String> {
    let (text, _) = status(w)?;
    let prefix = format!("gate {id} [");
    let line = text
        .lines()
        .find(|l| l.starts_with(&prefix))
        .ok_or(format!("no row for {id}:\n{text}"))?;
    Ok(line[prefix.len()..]
        .split(']')
        .next()
        .unwrap_or_default()
        .to_owned())
}

/// The receipts of the run, oldest name first.
fn receipts(w: &World) -> TestResult<Vec<PathBuf>> {
    let dir = w.kb.join(format!("runs/{RUN}/gates"));
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .collect();
    out.sort();
    Ok(out)
}

fn receipt_of(w: &World, id: &str) -> TestResult<PathBuf> {
    receipts(w)?
        .into_iter()
        .rfind(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(&format!("{id}-")))
        })
        .ok_or_else(|| format!("no receipt for {id}").into())
}

fn commit_all(dir: &Path, message: &str) -> TestResult {
    git(dir, &["add", "-A"])?;
    git(dir, &["commit", "-q", "-m", message])?;
    Ok(())
}

#[test]
fn status_golden_covers_every_state() -> TestResult {
    let w = world(&[])?;
    define(&w.kb, "pass", &["true"], &["src/ledger.rs"], 60)?;
    define(
        &w.kb,
        "fail",
        &["sh", "-c", "echo bad >&2; exit 3"],
        &["src/batch.rs"],
        60,
    )?;
    define(&w.kb, "slow", &["sleep", "30"], &["src/batch.rs"], 1)?;
    define(&w.kb, "edits", &["true"], &["src/retry.rs"], 60)?;
    define(&w.kb, "fresh", &["true"], &["**"], 60)?;
    define(
        &w.code,
        "crash",
        &["sh", "-c", "kill -SEGV $$"],
        &["src/batch.rs"],
        60,
    )?;
    for (n, id) in ["pass", "fail", "slow", "edits", "crash"]
        .iter()
        .enumerate()
    {
        let at = NOW + 60 * i64::try_from(n)?;
        let (text, exit) = run(&w, id, at)?;
        let want = if *id == "pass" || *id == "edits" {
            0
        } else {
            2
        };
        assert_eq!(exit, want, "{id}:\n{text}");
    }
    // A hand-written receipt claiming that `fresh` passed.
    let fake = format!("runs/{RUN}/gates/fresh-20261111T010000Z-abcdef.json");
    let bytes = std::fs::read_to_string(receipt_of(&w, "pass")?)?.replace("\"pass\"", "\"fresh\"");
    std::fs::write(w.kb.join(&fake), bytes)?;
    std::fs::write(w.code.join("src/retry.rs"), "pub fn retry() {}\n")?;
    let (text, exit) = status(&w)?;
    golden("gate-status.txt", &text, exit)?;
    for (id, state) in [
        ("pass", "satisfied"),
        ("fail", "failed"),
        ("slow", "unknown"),
        ("edits", "stale"),
        ("fresh", "open"),
        ("crash", "unknown"),
    ] {
        assert_eq!(state_of(&w, id)?, state, "{text}");
    }
    let (lines, exit) = gate(
        &w,
        GateVerb::Lines {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
        NOW + 3600,
    )?;
    golden("gate-lines.txt", &lines, exit)?;
    // `resume` from inside the subject shows the same rows.
    let mut request = w.request(Verb::Resume {
        run: Some(RUN.into()),
    });
    request.cwd = w.code.clone();
    let (resumed, _) = w.run(&request)?;
    assert!(
        resumed.contains("gate edits [stale] mechanical pass exit=0"),
        "{resumed}"
    );
    assert!(
        resumed.contains("  next: writfold kb gate run fresh"),
        "{resumed}"
    );
    let (verify, exit) = w.run(&w.request(Verb::IndexVerify))?;
    assert!(exit == 0 && !verify.contains("[differs]"), "{verify}");
    Ok(())
}

#[test]
fn a_docs_change_keeps_unit_satisfied_and_a_code_change_makes_it_stale() -> TestResult {
    let w = world(&[])?;
    define(&w.kb, "unit", &["true"], &["**", "!docs/**", "!*.md"], 60)?;
    run(&w, "unit", NOW)?;
    assert_eq!(state_of(&w, "unit")?, "satisfied");
    std::fs::create_dir_all(w.code.join("docs"))?;
    std::fs::write(w.code.join("docs/notes.md"), "# Notes\n")?;
    std::fs::write(w.code.join("README.md"), "# Demo\n")?;
    assert_eq!(state_of(&w, "unit")?, "satisfied", "dirty docs");
    commit_all(&w.code, "docs")?;
    assert_eq!(state_of(&w, "unit")?, "satisfied", "committed docs");
    let original = std::fs::read_to_string(w.code.join("src/ledger.rs"))?;
    std::fs::write(w.code.join("src/ledger.rs"), format!("{original}// more\n"))?;
    assert_eq!(state_of(&w, "unit")?, "stale", "dirty code");
    commit_all(&w.code, "code")?;
    let (text, exit) = status(&w)?;
    assert!(text.contains("gate unit [stale]") && exit == 2, "{text}");
    assert!(
        text.contains("  why: changed in scope: src/ledger.rs"),
        "{text}"
    );
    // Back to the receipt's bytes: the same content is the same subject.
    std::fs::write(w.code.join("src/ledger.rs"), original)?;
    commit_all(&w.code, "revert")?;
    assert_eq!(state_of(&w, "unit")?, "satisfied", "reverted");
    // A run on a dirty tree stays satisfied once those bytes are committed.
    std::fs::write(w.code.join("src/new.rs"), "pub fn new() {}\n")?;
    run(&w, "unit", NOW + 60)?;
    assert_eq!(state_of(&w, "unit")?, "satisfied", "dirty run");
    commit_all(&w.code, "new")?;
    assert_eq!(state_of(&w, "unit")?, "satisfied", "dirty run committed");
    // A changed definition makes every earlier receipt stale.
    define(&w.kb, "unit", &["true"], &["**", "!docs/**"], 60)?;
    let (text, _) = status(&w)?;
    assert!(
        text.contains("  why: the definition changed since this receipt"),
        "{text}"
    );
    Ok(())
}

#[test]
fn timeouts_crashes_and_errors_are_never_satisfied() -> TestResult {
    let w = world(&[])?;
    define(
        &w.kb,
        "slow",
        &["sh", "-c", "sleep 30 & sleep 30"],
        &["src/**"],
        1,
    )?;
    define(
        &w.kb,
        "crash",
        &["sh", "-c", "kill -KILL $$"],
        &["src/**"],
        60,
    )?;
    define(&w.kb, "missing", &["./no-such-program"], &["src/**"], 60)?;
    define(&w.kb, "fail", &["false"], &["src/**"], 60)?;
    let started = std::time::Instant::now();
    for (id, field, state) in [
        ("slow", "\"timed_out\": true", "unknown"),
        ("crash", "\"signal\": 9", "unknown"),
        ("missing", "\"error\": \"could not run", "unknown"),
        ("fail", "\"exit_code\": 1", "failed"),
    ] {
        let (text, exit) = run(&w, id, NOW)?;
        assert_eq!(exit, 2, "{text}");
        let bytes = std::fs::read_to_string(receipt_of(&w, id)?)?;
        assert!(bytes.contains(field), "{id}: {bytes}");
        assert!(!bytes.contains("\"outcome\": \"pass\""), "{bytes}");
        assert_eq!(state_of(&w, id)?, state);
    }
    // The timed-out group (with its background sleep) was stopped.
    assert!(started.elapsed() < std::time::Duration::from_secs(20));
    Ok(())
}

#[test]
fn hand_written_receipts_are_invalid_never_satisfied() -> TestResult {
    let w = world(&[])?;
    define(&w.kb, "fail", &["false"], &["src/**"], 60)?;
    define(&w.kb, "other", &["true"], &["src/**"], 60)?;
    run(&w, "fail", NOW)?;
    let real = std::fs::read_to_string(receipt_of(&w, "fail")?)?;
    // Edited outcome: the digest no longer matches.
    let edited = real
        .replace("\"outcome\": \"fail\"", "\"outcome\": \"pass\"")
        .replace("\"exit_code\": 1", "\"exit_code\": 0");
    let dir = w.kb.join(format!("runs/{RUN}/gates"));
    std::fs::write(dir.join("fail-20261111T020000Z-000001.json"), edited)?;
    // Written by hand, with no digest at all.
    let mut map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&real)?;
    map.remove("digest");
    map.insert("gate".into(), "other".into());
    std::fs::write(
        dir.join("other-20261111T020000Z-000002.json"),
        serde_json::to_string(&map)?,
    )?;
    // A real receipt filed under another gate's name.
    std::fs::write(dir.join("other-20261111T020000Z-000003.json"), &real)?;
    let (text, exit) = status(&w)?;
    assert_eq!(exit, 2);
    assert!(
        text.contains("gate fail [failed]") && text.contains("gate other [open]"),
        "{text}"
    );
    assert_eq!(
        text.matches("[invalid] not written by writfold gate run")
            .count(),
        2,
        "{text}"
    );
    assert!(
        text.contains("[invalid] its run or gate does not match where it is filed"),
        "{text}"
    );
    assert!(text.contains(" invalid=3 "), "{text}");
    Ok(())
}

#[test]
fn verify_agrees_then_disagrees() -> TestResult {
    let w = world(&[])?;
    let marker = w.base.join("marker");
    let check = format!("test ! -e {} && test -f src/batch.rs", marker.display());
    define(&w.kb, "probe", &["sh", "-c", &check], &["src/**"], 60)?;
    run(&w, "probe", NOW)?;
    let path = receipt_of(&w, "probe")?;
    let given = path.strip_prefix(&w.kb)?.display().to_string();
    let verify = |at: i64| {
        gate(
            &w,
            GateVerb::Verify {
                receipt: given.clone(),
                repo: None,
            },
            at,
        )
    };
    let (text, exit) = verify(NOW + 60)?;
    assert!(
        exit == 0 && text.contains("[agree] recorded pass exit=0; re-run pass exit=0"),
        "{text}"
    );
    std::fs::write(&marker, "")?;
    let (text, exit) = verify(NOW + 120)?;
    assert!(
        exit == 2 && text.contains("[disagree] recorded pass exit=0; re-run fail exit=1"),
        "{text}"
    );
    assert!(text.contains(" disagree=1 "), "{text}");
    let trees = git(&w.code, &["worktree", "list"])?;
    assert_eq!(
        trees.lines().count(),
        1,
        "a verification worktree was left: {trees}"
    );
    // A receipt of a dirty subject cannot be reproduced from a commit.
    std::fs::remove_file(&marker)?;
    std::fs::write(w.code.join("src/extra.rs"), "\n")?;
    run(&w, "probe", NOW + 180)?;
    let dirty = receipt_of(&w, "probe")?
        .strip_prefix(&w.kb)?
        .display()
        .to_string();
    let (text, exit) = gate(
        &w,
        GateVerb::Verify {
            receipt: dirty,
            repo: None,
        },
        NOW + 240,
    )?;
    assert!(
        exit == 2 && text.contains("[unverifiable] the receipt's subject had 1 dirty"),
        "{text}"
    );
    Ok(())
}
