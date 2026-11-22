//! #90: a cited commit this clone does not hold is not evidence of change.
//! The same citations are checked against a full-history repository and a
//! `--depth 1` clone of it: what the bytes at the target decide (`blob=`,
//! `span=`) reads the same in both; what needs the cited commit is
//! `unverifiable history-not-local` in the shallow clone and the true
//! verdict in the full one; after `git fetch --unshallow` they agree.
mod support;

use support::{TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_sources::citations::normalize_span;

fn body(name: &str) -> String {
    format!(
        "pub fn {name}_open() -> u32 {{\n    let {name} = 1;\n    {name} + 1\n}}\n\npub fn {name}_close() {{\n    drop({name}_open());\n}}\n"
    )
}

fn put(dir: &std::path::Path, path: &str, text: &str) -> TestResult {
    let file = dir.join(path);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, text)?;
    Ok(())
}

/// A token citing lines 1-3 of `path` at `rev`, with `blob=` and `span=`
/// only when asked.
fn token(w: &World, rev: &str, path: &str, blob: bool, span: bool) -> TestResult<String> {
    let mut out = format!("⟦cite kind=code repo=demo-service rev={rev} path={path} L=1-3");
    if blob {
        let id = git(&w.code, &["rev-parse", &format!("{rev}:{path}")])?;
        out.push_str(&format!(" blob={id}"));
    }
    if span {
        let text = git(&w.code, &["show", &format!("{rev}:{path}")])?;
        let lines: Vec<&str> = text.lines().take(3).collect();
        out.push_str(&format!(" span={}", normalize_span(&lines)));
    }
    out.push('⟧');
    Ok(out)
}

/// Cite five files at one commit, then change them in the next:
/// line 3 `same.rs` unchanged (blob=); line 4 `shift.rs` shifted (span=,
/// no blob=); line 5 `edit.rs` edited (neither); line 6 `moved.rs` renamed
/// with its cited lines shifted (blob=, span=); line 7 `drop.rs` deleted.
fn history() -> TestResult<World> {
    let w = world(&[])?;
    for name in ["same", "shift", "edit", "moved", "drop"] {
        put(&w.code, &format!("src/{name}.rs"), &body(name))?;
    }
    git(&w.code, &["add", "-A"])?;
    git(&w.code, &["commit", "-q", "-m", "cited"])?;
    let rev = git(&w.code, &["rev-parse", "HEAD"])?;
    let mut note = String::from("# Shallow\n\n");
    for (path, blob, span) in [
        ("src/same.rs", true, false),
        ("src/shift.rs", false, true),
        ("src/edit.rs", false, false),
        ("src/moved.rs", true, true),
        ("src/drop.rs", true, false),
    ] {
        note.push_str(&token(&w, &rev, path, blob, span)?);
        note.push('\n');
    }
    put(
        &w.code,
        "src/shift.rs",
        &format!("// new\n{}", body("shift")),
    )?;
    put(&w.code, "src/edit.rs", &body("edit").replace("+ 1", "+ 2"))?;
    std::fs::remove_file(w.code.join("src/moved.rs"))?;
    put(
        &w.code,
        "src/io/moved.rs",
        &format!("// here\n{}", body("moved")),
    )?;
    std::fs::remove_file(w.code.join("src/drop.rs"))?;
    git(&w.code, &["add", "-A"])?;
    git(&w.code, &["commit", "-q", "-m", "changed"])?;
    put(&w.kb, "lantern/shallow.md", &note)?;
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", "shallow"])?;
    Ok(w)
}

fn stale(w: &World) -> TestResult<String> {
    let (text, _) = w.run(&w.request(Verb::Stale {
        scope: None,
        offline: false,
        cached: false,
    }))?;
    Ok(text)
}

/// The row for `line`; `kb stale` prints no row for a fresh citation. Both
/// clones' heads are the same commit (named `origin/HEAD` in the clone), so
/// rows compare whole.
fn row(text: &str, line: u32) -> TestResult<String> {
    let at = format!("@lantern/shallow.md:{line} ");
    Ok(text.lines().find(|l| l.contains(&at)).map_or_else(
        || "[fresh]".to_owned(),
        |l| l.replace("(origin/HEAD)", "(HEAD)"),
    ))
}

/// Point the KB's local repository map at `dir`.
fn map_to(w: &World, dir: &std::path::Path) -> TestResult {
    let map = w.kb.join(".git/writfold/repos");
    std::fs::write(
        map,
        format!("# local repository map\ndemo-service {}\n", dir.display()),
    )?;
    Ok(())
}

#[test]
fn a_shallow_clone_never_reads_missing_history_as_change() -> TestResult {
    let w = history()?;
    let full = stale(&w)?;
    let shallow_dir = w.kb.parent().ok_or("no base")?.join("demo-shallow");
    let url = format!("file://{}", w.code.display());
    git(
        w.kb.parent().ok_or("no base")?,
        &[
            "clone",
            "-q",
            "--depth",
            "1",
            &url,
            &shallow_dir.display().to_string(),
        ],
    )?;
    map_to(&w, &shallow_dir)?;
    let shallow = stale(&w)?;

    // Decided from the bytes at the target: the same in both.
    for line in [3, 4] {
        assert_eq!(row(&full, line)?, row(&shallow, line)?, "line {line}");
    }
    assert!(row(&full, 3)?.contains("[fresh]"), "{full}");
    assert!(row(&full, 4)?.contains("[moved]"), "{full}");
    // Needs the cited commit: the truth in the full clone, unverifiable in
    // the shallow one, never changed or gone there.
    assert!(row(&full, 5)?.contains("[changed]"), "{full}");
    assert!(row(&full, 6)?.contains("[moved]"), "{full}");
    assert!(row(&full, 6)?.contains("src/io/moved.rs"), "{full}");
    assert!(row(&full, 7)?.contains("[gone]"), "{full}");
    for line in [5, 6, 7] {
        let got = row(&shallow, line)?;
        assert!(got.contains("[unverifiable]"), "line {line}: {got}");
        assert!(got.contains("history-not-local"), "line {line}: {got}");
        assert!(got.contains("shallow clone"), "line {line}: {got}");
    }

    // `kb repos` names the shallow clone; its status stays local.
    let (repos, _) = w.run(&w.request(Verb::Repos { missing: false }))?;
    assert!(repos.contains("history: shallow"), "{repos}");
    assert!(repos.contains("shallow=1"), "{repos}");
    assert!(repos.contains("[local]"), "{repos}");

    // With the history fetched, the shallow clone agrees with the full one.
    git(&shallow_dir, &["fetch", "-q", "--unshallow"])?;
    let deepened = stale(&w)?;
    for line in 3..=7 {
        assert_eq!(row(&full, line)?, row(&deepened, line)?, "line {line}");
    }
    Ok(())
}
