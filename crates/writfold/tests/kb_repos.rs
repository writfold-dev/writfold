//! `writfold kb repos` through the real binary: clones found under the
//! `$WRITFOLD_KB_REPOS` search roots (as `<name>` or `<org>/<name>`), the
//! `--missing` filter, its exit codes and its argument checks.
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
const WRITFOLD: &str = env!("CARGO_BIN_EXE_writfold");

struct Root(PathBuf);

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args([
            "-c",
            "init.defaultBranch=main",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

/// A one-file repository at `dir`; returns its commit.
fn code(dir: &Path) -> Result<String> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join("a.rs"), "fn a() {}\n")?;
    git(dir, &["init", "-q"])?;
    git(dir, &["add", "-A"])?;
    git(dir, &["commit", "-q", "-m", "a"])?;
    git(dir, &["rev-parse", "HEAD"])
}

/// Search roots holding `org/svc` and `tool`, and a KB citing them and
/// `ghost` (cloned nowhere).
fn world(name: &str) -> Result<Root> {
    let dir = std::env::temp_dir().join(format!("writfold-repos-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let root = Root(fs::canonicalize(&dir)?);
    let svc = code(&root.0.join("roots/org/svc"))?;
    let tool = code(&root.0.join("roots/tool"))?;
    let kb = root.0.join("kb");
    fs::create_dir_all(kb.join("inventory/assets"))?;
    fs::write(
        kb.join("inventory/assets/demo.json"),
        "{\"id\": \"demo\", \"kind\": \"service\", \"name\": \"Demo\"}\n",
    )?;
    let cite =
        |repo: &str, rev: &str| format!("⟦cite kind=code repo={repo} rev={rev} path=a.rs L=1-1⟧\n");
    let note = format!(
        "# Demo\n\n{}{}{}",
        cite("org/svc", &svc),
        cite("tool", &tool),
        cite("ghost", &tool)
    );
    fs::create_dir_all(kb.join("demo"))?;
    fs::write(kb.join("demo/README.md"), note)?;
    git(&kb, &["init", "-q"])?;
    git(&kb, &["add", "-A"])?;
    git(&kb, &["commit", "-q", "-m", "kb"])?;
    Ok(root)
}

fn repos(root: &Path, args: &[&str], roots: Option<&Path>) -> Result<Output> {
    let mut command = Command::new(WRITFOLD);
    command
        .args(["kb", "repos", "--kb"])
        .arg(root.join("kb"))
        .args(args)
        .current_dir(root)
        .env("HOME", root)
        .env_remove("WRITFOLD_KB")
        .env_remove("WRITFOLD_KB_REPOS");
    if let Some(roots) = roots {
        command.env("WRITFOLD_KB_REPOS", roots);
    }
    Ok(command.output()?)
}

fn rows(output: &Output) -> Result<Vec<Value>> {
    let body: Value = serde_json::from_slice(&output.stdout)?;
    Ok(body["rows"].as_array().ok_or("rows")?.clone())
}

fn label<'a>(row: &'a Value, label: &str) -> Option<&'a Value> {
    row["more"].as_array()?.iter().find(|m| m["label"] == label)
}

#[test]
fn search_roots_find_named_and_org_named_clones() -> Result {
    let r = world("roots")?;
    let roots = r.0.join("roots");
    let all = repos(&r.0, &["--json"], Some(&roots))?;
    assert_eq!(all.status.code(), Some(2), "{all:?}");
    let found = rows(&all)?;
    let names: Vec<(&str, &str)> = found
        .iter()
        .map(|row| {
            (
                row["id"].as_str().unwrap_or_default(),
                row["status"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        names,
        [
            ("ghost", "missing"),
            ("org/svc", "local"),
            ("tool", "local")
        ]
    );
    for row in &found[1..] {
        let resolved = label(row, "resolved").ok_or("resolved")?;
        assert_eq!(resolved["text"], "search-root");
        assert_eq!(
            label(row, "target").ok_or("target")?["text"]
                .as_str()
                .map(|t| t.starts_with("HEAD ")),
            Some(true)
        );
    }
    let svc = label(&found[1], "resolved").ok_or("resolved")?;
    assert_eq!(
        svc["path"].as_str(),
        Some(roots.join("org/svc/.git").display().to_string().as_str())
    );
    let missing = repos(&r.0, &["--missing", "--json"], Some(&roots))?;
    assert_eq!(missing.status.code(), Some(2));
    assert_eq!(rows(&missing)?.len(), 1);
    // Without the search roots every repository is missing.
    let none = repos(&r.0, &["--missing"], None)?;
    let text = String::from_utf8(none.stdout)?;
    assert_eq!(text.matches("[missing]").count(), 3, "{text}");
    assert!(text.contains(" repos=3 missing=3 partial=0 "), "{text}");
    Ok(())
}

#[test]
fn arguments_are_checked() -> Result {
    let r = world("args")?;
    let extra = repos(&r.0, &["ghost"], None)?;
    assert_eq!(extra.status.code(), Some(1));
    assert!(String::from_utf8(extra.stderr)?.contains("repos takes no arguments"));
    let output = Command::new(WRITFOLD)
        .args(["kb", "stale", "--missing"])
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)?.contains("--missing does not apply to stale"));
    Ok(())
}
