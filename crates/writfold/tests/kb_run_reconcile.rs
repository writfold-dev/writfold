//! The run reconciliation recipe of `docs/guide/recipes/` (#97) runs as
//! published, against a KB with each way a run can be left out: more runs
//! than one page holds, a run directory without `manifest.json`, a manifest
//! outside `runs/<id>/`, one that does not parse, a duplicate `run_id`, a
//! state other than `open` or `closed`, and IDs the other tool disagrees on.
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Demo(PathBuf);

impl Drop for Demo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn put(path: &Path, text: &str) -> Result {
    std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
    std::fs::write(path, text)?;
    Ok(())
}

fn run(dir: &Path, demo: &Demo, program: &str, args: &[&str]) -> Result<(String, i32)> {
    let path = format!(
        "{}:{}",
        demo.0.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new(program)
        .args(args)
        .current_dir(dir)
        .env("PATH", path)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .env_remove("WRITFOLD_KB")
        .env_remove("WRITFOLD_SESSION")
        .output()?;
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    Ok((text, out.status.code().unwrap_or(-1)))
}

fn manifest(id: &str, state: &str) -> String {
    format!(
        "{{\"run_id\": \"{id}\", \"task\": \"{id}\", \"created\": \"2026-11-01T09:00:00Z\", \"state\": \"{state}\"}}\n"
    )
}

/// 80 good runs (more than one page of `kb runs`), plus each cause.
fn demo(name: &str, causes: bool) -> Result<Demo> {
    let dir = std::env::temp_dir().join(format!("writfold-runs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let d = Demo(dir.clone());
    std::fs::create_dir_all(dir.join("bin"))?;
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_writfold"), dir.join("bin/writfold"))?;
    let kb = dir.join("kb");
    put(
        &kb.join("inventory/assets/demo.json"),
        "{\"id\": \"demo\", \"kind\": \"service\", \"name\": \"Demo\"}\n",
    )?;
    for n in 0..80 {
        let state = if n % 4 == 0 { "closed" } else { "open" };
        let id = format!("run-{n:03}");
        put(&kb.join(format!("runs/{id}/manifest.json")), &manifest(&id, state))?;
    }
    if causes {
        put(&kb.join("runs/run-nomanifest/checkpoint.md"), "# notes\n")?;
        put(
            &kb.join("archive/runs/run-nested/manifest.json"),
            &manifest("run-nested", "open"),
        )?;
        put(&kb.join("runs/run-bad/manifest.json"), "{\"run_id\": \"run-bad\", \"task\": ")?;
        put(&kb.join("runs/run-dup1/manifest.json"), &manifest("run-dup", "open"))?;
        put(&kb.join("runs/run-dup2/manifest.json"), &manifest("run-dup", "open"))?;
        put(&kb.join("runs/run-done/manifest.json"), &manifest("run-done", "done"))?;
    }
    let (text, exit) = run(
        &kb,
        &d,
        "sh",
        &["-c", "git init -q -b main && git add -A && git commit -qm kb"],
    )?;
    assert_eq!(exit, 0, "{text}");
    Ok(d)
}

fn recipe() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/guide/recipes/reconcile-runs.py")
        .display()
        .to_string()
}

#[test]
fn every_silent_cause_is_named() -> Result {
    let d = demo("causes", true)?;
    let kb = d.0.join("kb");
    put(
        &d.0.join("other.txt"),
        "run-001 open\nrun-004 open\nrun-elsewhere open\n",
    )?;
    let other = d.0.join("other.txt").display().to_string();
    let (text, exit) = run(&kb, &d, "python3", &[&recipe(), "--other", &other])?;
    assert_eq!(exit, 1, "{text}");
    // Every page was read: 80 good runs, the duplicate's first and the done run.
    assert!(text.contains("writfold lists 82 run(s) in "), "{text}");
    assert!(!text.contains(" in 1 page(s)"), "one page would hide the cursor: {text}");
    for expected in [
        "directories without manifest.json (1)",
        "  run-nomanifest",
        "manifest.json outside runs/<id>/ (1)",
        "  archive/runs/run-nested/manifest.json",
        "manifests writfold does not list (2)",
        "  runs/run-bad/manifest.json",
        "  runs/run-dup2/manifest.json",
        "duplicate run_id (1)",
        "state other than open or closed (1)",
        "  run-done [done]",
        "only in the other tool (1)",
        "  run-elsewhere",
        "state differs (1)",
        "  run-004: writfold closed, other open",
        "coverage=partial unreadable=2",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in\n{text}");
    }
    Ok(())
}

#[test]
fn a_clean_inventory_agrees() -> Result {
    let d = demo("clean", false)?;
    let (text, exit) = run(&d.0.join("kb"), &d, "python3", &[&recipe()])?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("writfold lists 80 run(s) in "), "{text}");
    assert!(text.contains("agree: every run directory is a listed run"), "{text}");
    Ok(())
}
