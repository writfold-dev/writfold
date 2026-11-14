//! Viewer stamps observe local records and subject metadata without an index.
mod support;

use support::{TestResult, git, world};
use writfold_kb::watch::{local, subject};
use writfold_kb::workspace::{GitTool, discover};

#[test]
fn local_stamp_tracks_queue_files_and_pointer_but_not_the_index() -> TestResult {
    let w = world(&[])?;
    let ws = discover(Some(&w.kb), &w.base)?;
    let state = writfold_kb::proposal::state_of(&ws).ok_or("state")?;
    for dir in ["drafts", "proposals"] {
        std::fs::create_dir_all(state.join(dir))?;
    }
    let initial = local(&ws);
    assert_eq!(initial, local(&ws));
    std::fs::write(state.join("unrelated-index.sqlite"), "derived")?;
    assert_eq!(initial, local(&ws));
    for dir in ["drafts", "proposals"] {
        let file = state.join(dir).join("entry.json");
        std::fs::write(&file, "one")?;
        let added = local(&ws);
        assert_ne!(initial, added);
        std::fs::write(&file, "a longer entry")?;
        let changed = local(&ws);
        assert_ne!(added, changed);
        std::fs::File::open(&file)?
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(17))?;
        assert_ne!(changed, local(&ws));
        std::fs::remove_file(file)?;
        assert_eq!(initial, local(&ws));
    }
    std::fs::write(state.join("current-run"), "run-one\n")?;
    let pointed = local(&ws);
    assert_ne!(initial, pointed);
    std::fs::write(state.join("current-run"), "run-two\n")?;
    assert_ne!(pointed, local(&ws));
    std::fs::remove_file(state.join("current-run"))?;
    assert_eq!(initial, local(&ws));
    Ok(())
}

#[test]
fn subject_stamp_changes_for_edits_untracked_paths_and_commits() -> TestResult {
    let w = world(&[])?;
    let ws = discover(Some(&w.kb), &w.base)?;
    let tool = GitTool::locate()?;
    let stamp = || -> TestResult<writfold_kb::watch::Stamp> {
        subject(&ws, &tool, &w.code, None)?.ok_or_else(|| "subject".into())
    };
    let initial = stamp()?;
    assert_eq!(initial, stamp()?);
    assert_eq!(initial.dirty, 0);
    std::fs::write(w.code.join("src/ledger.rs"), "changed\n")?;
    let edited = stamp()?;
    assert_ne!(initial.digest, edited.digest);
    assert_eq!(initial.head, edited.head);
    std::fs::write(w.code.join("new.txt"), "untracked\n")?;
    let untracked = stamp()?;
    assert_ne!(edited.digest, untracked.digest);
    assert_eq!(untracked.dirty, 2);
    git(&w.code, &["add", "-A"])?;
    git(&w.code, &["commit", "-q", "-m", "changed subject"])?;
    let committed = stamp()?;
    assert_ne!(untracked.digest, committed.digest);
    assert_ne!(initial.head, committed.head);
    assert_eq!(committed.dirty, 0);
    assert!(subject(&ws, &tool, &w.base, None)?.is_none());
    let recorded = subject(&ws, &tool, &w.kb, w.code.to_str())?.ok_or("recorded subject")?;
    assert_eq!(recorded, committed);
    assert!(!state_index_exists(&ws));
    Ok(())
}

fn state_index_exists(ws: &writfold_kb::workspace::Workspace) -> bool {
    writfold_kb::proposal::state_of(ws)
        .and_then(|p| std::fs::read_dir(p).ok())
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().starts_with("kb-index-"))
}
