//! A real binary tour: every pane is read-only and starts no checker.
mod checker;

use checker::{RUN, Result, Root, cached_rows, kb, run, world};
use std::path::{Path, PathBuf};

const TOKEN: &str = "⟦cite kind=src depot=main path=lib/fresh.c L=3-9 want=fresh⟧";
const B: &str = "run-20261102T090000Z-b1c2d3";
const C: &str = "run-20261103T090000Z-c1d2e3";
const VIEWS: [&str; 8] = [
    "progress", "gates", "timeline", "inbox", "guards", "stale", "brief", "resume",
];

fn put(path: &Path, text: &str) -> Result {
    std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
    std::fs::write(path, text)?;
    Ok(())
}

fn prepare(w: &Root) -> Result {
    put(
        &w.kb().join("harbor/README.md"),
        &format!("# Harbor\n\n{TOKEN}\n"),
    )?;
    for id in [B, C] {
        let manifest = serde_json::json!({"run_id":id, "task":"Read the records", "project":"harbor", "mode":"authoring", "created":"2026-11-02T09:00:00Z", "state":"open"});
        put(
            &w.kb().join(format!("runs/{id}/manifest.json")),
            &manifest.to_string(),
        )?;
    }
    put(
        &w.kb().join(format!("runs/{C}/writfold-run.json")),
        "{\"state\":\"closed\"}",
    )?;
    put(
        &w.kb().join(".writfold/plans/watch.toml"),
        "schema = \"writfold-plan/1\"\nname = \"watch\"\nsteps = [\"review: review\"]\n",
    )?;
    put(
        &w.kb().join(".writfold/guards/review.toml"),
        "schema = \"writfold-guard-rule/1\"\nid = \"review\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\"]\nrequire = [\"gates.met\"]\naction = \"deny\"\n",
    )?;
    let (text, exit) = kb(w, &["plan", "set", "watch", "--run", RUN], &[])?;
    assert_eq!(exit, 0, "{text}");
    put(&w.kb().join(".git/writfold/current-run"), RUN)?;
    put(
        &w.kb().join(".git/writfold/drafts/watch.json"),
        &format!("{{\"run\":\"{RUN}\",\"summary\":\"Review the evidence\"}}"),
    )?;
    let (text, exit) = kb(
        w,
        &[
            "propose",
            "ticket",
            "WATCH-1",
            "--title",
            "Review evidence",
            "--run",
            RUN,
        ],
        &[],
    )?;
    assert_eq!(exit, 0, "{text}");
    kb(w, &["index", "status"], &[])?;
    Ok(())
}

fn files(root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    let mut out = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            if name == ".git" || name.to_string_lossy().starts_with("kb-index-") {
                continue;
            }
            if entry.file_type()?.is_dir() {
                pending.push(entry.path());
            } else {
                out.push((
                    entry.path().strip_prefix(root)?.to_path_buf(),
                    std::fs::read(entry.path())?,
                ));
            }
        }
    }
    out.sort();
    Ok(out)
}

fn frame(w: &Root, id: &str, view: &str) -> Result {
    let out = run(
        w,
        &["watch", "--run", id, "--view", view, "--once"],
        &[],
        "",
    )?;
    let text = String::from_utf8(out.stdout)?;
    assert!(
        out.status.success(),
        "{view}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains(&format!("== {view} ")), "{text}");
    assert!(!text.contains('\u{1b}'), "{text}");
    if view == "stale" {
        assert!(text.contains("not-checked"), "{text}");
        assert!(text.contains("no checker was started"), "{text}");
    }
    Ok(())
}

#[test]
fn all_watch_panes_leave_records_local_state_and_checker_cache_unchanged() -> Result {
    let w = world("watch-tour", &[TOKEN], true)?;
    prepare(&w)?;
    let records = files(&w.kb())?;
    let local = files(&w.kb().join(".git/writfold"))?;
    let cache = cached_rows(&w)?;
    for _ in 0..3 {
        for id in [RUN, B, C] {
            for view in VIEWS {
                frame(&w, id, view)?;
            }
        }
    }
    assert_eq!(w.spawns(), 0);
    assert_eq!(records, files(&w.kb())?);
    assert_eq!(local, files(&w.kb().join(".git/writfold"))?);
    assert_eq!(cache, cached_rows(&w)?);
    let (text, exit) = kb(&w, &["stale"], &[])?;
    assert_eq!((exit, w.spawns()), (0, 1), "{text}");
    Ok(())
}
