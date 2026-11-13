//! A rename with edits (design section 6.2, step 5): a cited path absent at
//! the target commit whose blob is nowhere in its tree is followed through
//! Git's rename detection from the cited revision. It is `moved` to the new
//! path only when the cited span digest is found there, never `fresh`, and
//! `gone` with the new path named when the span was edited.
mod support;

use support::{TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_sources::citations::normalize_span;

const BODY: &str = "pub fn flush(queue: &mut Queue) {\n    while let Some(page) = queue.pop() {\n        disk.write(page);\n    }\n    disk.sync();\n}\n\npub fn drain(queue: &mut Queue) {\n    queue.clear();\n}\n\npub fn size(queue: &Queue) -> usize {\n    queue.len()\n}\n";

/// Each file's own text, so Git pairs every rename with its source.
fn body(name: &str) -> String {
    BODY.replace("queue", name)
}

fn put(dir: &std::path::Path, path: &str, text: &str) -> TestResult {
    let file = dir.join(path);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, text)?;
    Ok(())
}

/// A token citing `path:first-last` at `rev`, with its blob and span.
fn token(w: &World, rev: &str, path: &str, first: usize, last: usize) -> TestResult<String> {
    let blob = git(&w.code, &["rev-parse", &format!("{rev}:{path}")])?;
    let text = git(&w.code, &["show", &format!("{rev}:{path}")])?;
    let lines: Vec<&str> = text
        .lines()
        .skip(first - 1)
        .take(last - first + 1)
        .collect();
    let span = normalize_span(&lines);
    Ok(format!(
        "⟦cite kind=code repo=demo-service rev={rev} path={path} L={first}-{last} blob={blob} span={span}⟧"
    ))
}

/// Cite four files, then rename each with edits: one keeps the cited lines
/// in place, one shifts them, one edits them, one is deleted outright.
fn renamed() -> TestResult<(World, String)> {
    let w = world(&[])?;
    for name in ["shift", "keep", "edit", "drop"] {
        put(&w.code, &format!("src/{name}.rs"), &body(name))?;
    }
    git(&w.code, &["add", "-A"])?;
    git(&w.code, &["commit", "-q", "-m", "third"])?;
    let rev = git(&w.code, &["rev-parse", "HEAD"])?;
    let mut note = String::from("# Renames\n\n");
    for name in ["shift", "keep", "edit", "drop"] {
        note.push_str(&token(&w, &rev, &format!("src/{name}.rs"), 1, 6)?);
        note.push('\n');
    }
    let elsewhere = token(&w, &rev, "src/shift.rs", 8, 10)?.replace(&rev, &"d".repeat(40));
    note.push_str(&elsewhere);
    note.push('\n');
    for name in ["shift", "keep", "edit", "drop"] {
        std::fs::remove_file(w.code.join(format!("src/{name}.rs")))?;
    }
    let shifted = format!("// moved\n// here\n{}", body("shift"));
    put(&w.code, "src/io/shift.rs", &shifted)?;
    let kept = body("keep").replace(".len()", ".count()");
    put(&w.code, "src/io/keep.rs", &kept)?;
    let edited = body("edit").replace("disk.sync()", "disk.flush()");
    put(&w.code, "src/io/edit.rs", &edited)?;
    git(&w.code, &["add", "-A"])?;
    git(&w.code, &["commit", "-q", "-m", "fourth"])?;
    put(&w.kb, "lantern/renames.md", &note)?;
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", "renames"])?;
    let (text, _) = w.run(&w.request(Verb::Stale {
        scope: None,
        offline: false,
        cached: false,
    }))?;
    Ok((w, text))
}

fn row(text: &str, line: u32) -> TestResult<&str> {
    let at = format!("@lantern/renames.md:{line} ");
    text.lines()
        .find(|l| l.contains(&at))
        .ok_or_else(|| format!("no row for line {line} in\n{text}").into())
}

#[test]
fn renames_with_edits_are_followed() -> TestResult {
    let (w, text) = renamed()?;
    let shift = row(&text, 3)?;
    assert!(shift.contains("[moved]"), "{shift}");
    assert!(shift.ends_with("!moved→src/io/shift.rs:3-8"), "{shift}");
    let keep = row(&text, 4)?;
    assert!(keep.ends_with("!moved→src/io/keep.rs:1-6"), "{keep}");
    let edit = row(&text, 5)?;
    assert!(edit.contains("[gone]"), "{edit}");
    assert!(
        edit.contains("renamed to src/io/edit.rs; span not found"),
        "{edit}"
    );
    let drop = row(&text, 6)?;
    assert!(drop.contains("[gone]"), "{drop}");
    assert!(
        drop.contains("path absent; no rename since the cited rev"),
        "{drop}"
    );
    let elsewhere = row(&text, 7)?;
    assert!(elsewhere.contains("[gone]"), "{elsewhere}");
    assert!(
        elsewhere.contains("cited rev not in repository"),
        "{elsewhere}"
    );
    for line in 3..=7 {
        assert!(!row(&text, line)?.contains("[fresh]"));
    }
    // The reported range at the new path holds the cited span digest.
    let now = std::fs::read_to_string(w.code.join("src/io/shift.rs"))?;
    let lines: Vec<&str> = now.lines().skip(2).take(6).collect();
    let original = body("shift");
    let cited: Vec<&str> = original.lines().take(6).collect();
    assert_eq!(normalize_span(&lines), normalize_span(&cited));
    Ok(())
}
