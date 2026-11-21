//! A refresh rewrites only the graph facts an edit affects (#59): an edit
//! that changes no reference writes no edge row; a changed reference
//! rewrites only its file's edges; a key change (a renamed record) takes
//! the explicit whole-graph fallback, which retargets references in files
//! that did not change; and at a fixed small edit, the edge rows written do
//! not grow with the unrelated edges. After every step the index equals a
//! rebuild (`index verify`).
mod support;

use std::path::Path;
use support::{TestResult, git};
use writfold_kb::{Request, Verb};

/// A KB of `n` decisions, each naming the next three by ID.
fn kb(name: &str, n: usize) -> TestResult<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!("writfold-graph-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("inventory/assets"))?;
    std::fs::create_dir_all(dir.join("proposals/decision"))?;
    std::fs::create_dir_all(dir.join("harbor"))?;
    std::fs::write(dir.join("harbor/README.md"), "# Harbor\n")?;
    std::fs::write(
        dir.join("inventory/assets/demo.json"),
        "{\"id\": \"demo\", \"kind\": \"service\", \"name\": \"Demo\"}\n",
    )?;
    for i in 0..n {
        write(&dir, i, n, "")?;
    }
    git(&dir, &["init", "-q"])?;
    git(&dir, &["add", "-A"])?;
    git(&dir, &["commit", "-q", "-m", "kb"])?;
    Ok(std::fs::canonicalize(dir)?)
}

/// Decision `i`: names the next three; `extra` is appended to the body.
fn write(dir: &Path, i: usize, n: usize, extra: &str) -> TestResult {
    let refs: Vec<String> = (1..=3)
        .map(|k| format!("decision/2026-01-01-d{:04}", (i + k) % n))
        .collect();
    std::fs::write(
        dir.join(format!("proposals/decision/2026-01-01-d{i:04}.md")),
        format!(
            "---\nid: decision/2026-01-01-d{i:04}\nkind: decision\ntitle: Decision {i}\nstatus: accepted\ncreated: 2026-01-01\nupdated: 2026-01-01\nproject: harbor\n---\n\n## Context\n\nWe chose it. See {}.{extra}\n",
            refs.join(" ")
        ),
    )?;
    Ok(())
}

fn request(dir: &Path, verb: Verb) -> Request {
    Request {
        verb,
        kb: Some(dir.to_path_buf()),
        cwd: dir.to_path_buf(),
        budget: writfold_kb::DEFAULT_BUDGET,
        cursor: None,
        json: false,
        now: Some(support::NOW),
        probe: writfold_kb::write::no_probe,
    }
}

/// Refresh by reading `index status`; the last refresh's graph work.
fn refresh(dir: &Path) -> TestResult<String> {
    let out = writfold_kb::run(&request(dir, Verb::IndexStatus))?;
    let line = (out.text.lines())
        .find(|l| l.trim_start().starts_with("last-refresh: "))
        .ok_or_else(|| format!("no last-refresh line:\n{}", out.text))?;
    Ok(line.trim_start()["last-refresh: ".len()..].to_owned())
}

fn equal_to_rebuild(dir: &Path) -> TestResult {
    let out = writfold_kb::run(&request(dir, Verb::IndexVerify))?;
    assert_eq!(out.exit, 0, "{}", out.text);
    Ok(())
}

/// `edges-written=N` of a refresh note.
fn written(note: &str) -> usize {
    note.split_whitespace()
        .find_map(|w| w.strip_prefix("edges-written="))
        .and_then(|n| n.parse().ok())
        .unwrap_or(usize::MAX)
}

#[test]
fn edits_rewrite_only_what_they_affect() -> TestResult {
    let dir = kb("plan", 40)?;
    let out = writfold_kb::run(&request(&dir, Verb::IndexStatus))?;
    assert!(
        out.text.contains("records: decision=40") && out.text.contains("mentions=120"),
        "{}",
        out.text
    );
    // Same lines, no reference changed: no edge row is written.
    let file = dir.join("proposals/decision/2026-01-01-d0005.md");
    let text = std::fs::read_to_string(&file)?.replace("We chose it.", "We picked it.");
    std::fs::write(&file, text)?;
    let note = refresh(&dir)?;
    assert!(note.contains("graph=skip") && written(&note) == 0, "{note}");
    equal_to_rebuild(&dir)?;
    // A new reference in one file: only that file's edges are replaced.
    write(&dir, 5, 40, " Also decision/2026-01-01-d0020.")?;
    let note = refresh(&dir)?;
    assert!(note.contains("graph=delta"), "{note}");
    assert!(
        (1..=6).contains(&written(&note)),
        "one file's edges: {note}"
    );
    equal_to_rebuild(&dir)?;
    // A renamed record is a key change: everything is resolved again, and
    // references in files that did not change (d0017..d0019 name d0020)
    // now dangle; the fallback says so.
    let renamed = std::fs::read_to_string(dir.join("proposals/decision/2026-01-01-d0020.md"))?
        .replace(
            "id: decision/2026-01-01-d0020",
            "id: decision/2026-01-01-renamed",
        );
    std::fs::write(dir.join("proposals/decision/2026-01-01-d0020.md"), renamed)?;
    let note = refresh(&dir)?;
    assert!(note.contains("graph=whole"), "{note}");
    equal_to_rebuild(&dir)?;
    Ok(())
}

#[test]
fn a_small_edit_writes_the_same_edges_however_large_the_graph() -> TestResult {
    let mut counts = Vec::new();
    for n in [30, 300] {
        let dir = kb(&format!("grow{n}"), n)?;
        writfold_kb::run(&request(&dir, Verb::IndexStatus))?;
        write(&dir, 7, n, " Also decision/2026-01-01-d0001.")?;
        let note = refresh(&dir)?;
        assert!(note.contains("graph=delta"), "{note}");
        assert!(written(&note) > 0, "the edit adds an edge: {note}");
        counts.push(written(&note));
        equal_to_rebuild(&dir)?;
    }
    assert_eq!(
        counts[0], counts[1],
        "edge rows written track the edit, not the graph: {counts:?}"
    );
    Ok(())
}
