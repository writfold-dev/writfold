//! `index verify` repairs only a confirmed mismatch at a still-current pin
//! (#58): a corrupted index is found at its first differing key (early and
//! late in a table) and replaced by the rebuild; when the KB is edited, or
//! another process commits to the live index, between the comparison and
//! the repair, nothing is replaced and nothing is claimed. A killed
//! verify's temporary file is swept.
mod support;

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use support::{TestResult, World, world};
use writfold_kb::Verb;

/// The live index file of the world's KB.
fn index_file(w: &World) -> TestResult<PathBuf> {
    let dir = w.kb.join(".git/writfold");
    let found = std::fs::read_dir(&dir)?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            name.starts_with("kb-index-") && name.ends_with(".sqlite")
        });
    found.ok_or_else(|| format!("no index in {}", dir.display()).into())
}

fn built() -> TestResult<(World, PathBuf)> {
    let w = world(&[])?;
    let (text, exit) = w.run(&w.request(Verb::IndexStatus))?;
    assert_eq!(exit, 0, "{text}");
    let file = index_file(&w)?;
    Ok((w, file))
}

fn execute(file: &Path, sql: &str) -> TestResult {
    let conn = Connection::open(file)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch(sql)?;
    Ok(())
}

/// The file whose `files` row the tests corrupt: not one they edit.
const VICTIM: &str = "(SELECT min(path) FROM files WHERE path <> 'harbor/README.md')";

/// The corrupted marker of the victim's row, if still there.
fn marker(file: &Path) -> TestResult<String> {
    let conn = Connection::open(file)?;
    let sql = format!("SELECT coalesce(parse_error, '') FROM files WHERE path = {VICTIM}");
    Ok(conn.query_row(&sql, [], |r| r.get(0))?)
}

/// Corrupt a Phase A row of the live index (a refresh leaves it alone).
fn corrupt_first(file: &Path) -> TestResult {
    execute(
        file,
        &format!("UPDATE files SET parse_error = 'corrupt' WHERE path = {VICTIM}"),
    )
}

fn verify(w: &World) -> TestResult<(String, u8)> {
    w.run(&w.request(Verb::IndexVerify))
}

fn verify_with(w: &World, between: &dyn Fn()) -> TestResult<(String, u8)> {
    let output = writfold_kb::index_verify_with(&w.request(Verb::IndexVerify), between)?;
    Ok((output.text, output.exit))
}

#[test]
fn a_corrupt_index_is_found_at_its_first_key_and_replaced() -> TestResult {
    let (w, file) = built()?;
    let sql = format!("SELECT path FROM files WHERE path = {VICTIM}");
    let first: String = Connection::open(&file)?.query_row(&sql, [], |r| r.get(0))?;
    corrupt_first(&file)?;
    let (text, exit) = verify(&w)?;
    assert_eq!(exit, 1, "{text}");
    assert!(
        text.contains(&format!("first-difference: {first}")),
        "early: {text}"
    );
    assert!(text.contains("the rebuild replaced it"), "{text}");
    assert_ne!(marker(&file)?, "corrupt");
    let (text, exit) = verify(&w)?;
    assert_eq!(exit, 0, "repaired: {text}");
    // The last edge in canonical order: found there, not earlier.
    let conn = Connection::open(&file)?;
    let key: (String, String, String, String, String, i64) = conn.query_row(
        "SELECT src, rel, dst, origin, prov_path, prov_line FROM edges ORDER BY src DESC, rel DESC, dst DESC, origin DESC, prov_path DESC, prov_line DESC LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
    )?;
    drop(conn);
    let src = key.0.clone();
    let update = Connection::open(&file)?;
    update.execute(
        "UPDATE edges SET status = 'corrupt' WHERE src = ?1 AND rel = ?2 AND dst = ?3 AND origin = ?4 AND prov_path = ?5 AND prov_line = ?6",
        rusqlite::params![key.0, key.1, key.2, key.3, key.4, key.5],
    )?;
    drop(update);
    let (text, exit) = verify(&w)?;
    assert_eq!(exit, 1, "{text}");
    let row = text
        .lines()
        .find(|l| l.starts_with("table edges [differs]"))
        .ok_or("edges row")?;
    assert!(row.contains("differs"), "{text}");
    assert!(
        text.contains(&format!("first-difference: {src} ")),
        "late: {text}"
    );
    Ok(())
}

#[test]
fn an_edit_between_comparison_and_repair_replaces_nothing() -> TestResult {
    let (w, file) = built()?;
    corrupt_first(&file)?;
    let edited = w.kb.join("harbor/README.md");
    let between = || {
        let _ = std::fs::write(&edited, "# Harbor\n\nedited while verify ran\n");
    };
    let (text, exit) = verify_with(&w, &between)?;
    assert_eq!(exit, 2, "{text}");
    assert!(
        text.contains("moved while it was compared: nothing was replaced"),
        "{text}"
    );
    assert_eq!(marker(&file)?, "corrupt", "the index was left alone");
    // Observed again, the corruption is still found and now repaired.
    let (text, exit) = verify(&w)?;
    assert_eq!(exit, 1, "{text}");
    assert_ne!(marker(&file)?, "corrupt");
    Ok(())
}

#[test]
fn a_commit_to_the_live_index_between_comparison_and_repair_replaces_nothing() -> TestResult {
    let (w, file) = built()?;
    corrupt_first(&file)?;
    // Another process commits to the live index (a refresh, say); the
    // comparison's read transaction kept it out until the comparison ended.
    let between = || {
        let _ = execute(
            &file,
            "UPDATE records SET summary = 'written by another process' WHERE id = (SELECT max(id) FROM records)",
        );
    };
    let (text, exit) = verify_with(&w, &between)?;
    assert_eq!(exit, 2, "{text}");
    assert!(text.contains("nothing was replaced"), "{text}");
    assert_eq!(marker(&file)?, "corrupt");
    Ok(())
}

#[test]
fn a_dead_process_temporary_file_is_swept() -> TestResult {
    let (w, file) = built()?;
    let dir = file.parent().ok_or("parent")?;
    let dead = dir.join(".kb-verify-4194999.tmp");
    let other = dir.join(".kb-verify-notapid.tmp");
    std::fs::write(&dead, "left by a killed verify")?;
    std::fs::write(&other, "not ours")?;
    let (text, exit) = verify(&w)?;
    assert_eq!(exit, 0, "{text}");
    assert!(!dead.exists(), "a dead process's file is swept");
    assert!(other.exists(), "a name that is not ours is left");
    Ok(())
}
