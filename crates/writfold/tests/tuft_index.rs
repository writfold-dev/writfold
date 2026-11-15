//! `writfold tuft index` through the binary with a fake `ctags` (a shell
//! script that answers `--version` and `--list-features` and writes one
//! JSON tag per `fn` line): the sidecar binds the index to the commit and
//! its digest, uncommitted bytes never reach it, the user's status and
//! worktree list are unchanged, a non-universal, JSON-less, absent or hung
//! ctags is refused and leaves nothing behind, and `find def` accepts the
//! pair.
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

const FAKE: &str = r##"#!/bin/sh
case "$1" in
--version)
  if [ "$FAKE_CTAGS" = exuberant ]; then echo "Exuberant Ctags 5.8"; else echo "Universal Ctags 6.1.0, Copyright (C) 2015-2024 Universal Ctags Team"; fi
  exit 0 ;;
--list-features)
  echo "#NAME DESCRIPTION"
  if [ "$FAKE_CTAGS" != nojson ]; then echo "json        supports json format output"; fi
  echo "wildcards   can use glob matching"
  exit 0 ;;
esac
if [ "$FAKE_CTAGS" = hang ]; then sleep 600 & echo $! > "$FAKE_PID"; wait; fi
out=""
while [ $# -gt 0 ]; do case "$1" in -f) out="$2"; shift 2 ;; *) shift ;; esac; done
find . -type f | sort | while read -r f; do
  p=${f#./}
  grep -n '^fn ' "$f" | while IFS=: read -r n rest; do
    name=$(printf '%s' "$rest" | sed 's/^fn \([a-z_]*\).*/\1/')
    printf '{"_type": "tag", "name": "%s", "path": "%s", "line": %s, "roles": "def"}\n' "$name" "$p" "$n"
  done
done > "$out"
"##;

struct World(PathBuf);

impl Drop for World {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl World {
    fn repo(&self) -> PathBuf {
        self.0.join("repo")
    }
    fn state(&self) -> PathBuf {
        self.0.join("state")
    }
    fn fake(&self) -> PathBuf {
        self.0.join("bin/ctags")
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .output()?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned().into());
    }
    Ok(String::from_utf8(out.stdout)?)
}

/// A committed `src/lib.rs` with `alpha` and `beta`, then an uncommitted
/// `dirty_only` and an untracked file; a trial state; the fake ctags.
fn world(name: &str) -> Result<World> {
    let dir =
        std::env::temp_dir().join(format!("writfold-tuft-index-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let w = World(dir);
    fs::create_dir_all(w.repo().join("src"))?;
    fs::create_dir_all(w.0.join("bin"))?;
    fs::write(w.repo().join("src/lib.rs"), "fn alpha() {}\nfn beta() {}\n")?;
    git(&w.repo(), &["init", "-q"])?;
    git(&w.repo(), &["add", "-A"])?;
    git(&w.repo(), &["commit", "-q", "-m", "code"])?;
    fs::write(
        w.repo().join("src/lib.rs"),
        "fn alpha() {}\nfn beta() {}\nfn dirty_only() {}\n",
    )?;
    fs::write(w.repo().join("src/new.rs"), "fn untracked_only() {}\n")?;
    fs::write(w.fake(), FAKE)?;
    fs::set_permissions(w.fake(), fs::Permissions::from_mode(0o755))?;
    let state = w.state().display().to_string();
    let init = call(&w, &["trial", "init", "--state", &state], &[], "")?;
    if !init.status.success() {
        return Err(String::from_utf8_lossy(&init.stderr).into_owned().into());
    }
    Ok(w)
}

fn call(w: &World, args: &[&str], env: &[(&str, &str)], stdin: &str) -> Result<Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_writfold"));
    command
        .current_dir(&w.0)
        .env_remove("WRITFOLD_STATE")
        .env_remove("WRITFOLD_KB")
        .env_remove("WRITFOLD_CTAGS_TIMEOUT")
        .env_remove("FAKE_CTAGS")
        .env("HOME", &w.0)
        .env("XDG_CONFIG_HOME", &w.0)
        .env("WRITFOLD_GIT", "/usr/bin/git")
        .env("WRITFOLD_CTAGS", w.fake())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        command.env(k, v);
    }
    let mut child = command.spawn()?;
    if let Some(mut input) = child.stdin.take() {
        std::io::Write::write_all(&mut input, stdin.as_bytes())?;
    }
    Ok(child.wait_with_output()?)
}

fn index(w: &World, extra: &[&str], env: &[(&str, &str)]) -> Result<Output> {
    let (repo, state) = (
        w.repo().display().to_string(),
        w.state().display().to_string(),
    );
    let mut args = vec!["tuft", "index", "--repo", &repo, "--state", &state];
    args.extend_from_slice(extra);
    call(w, &args, env, "")
}

fn text(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

/// What is left in `<state>/symbols` besides finished indexes.
fn leftovers(w: &World) -> Vec<String> {
    fs::read_dir(w.state().join("symbols"))
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.starts_with('.') || n.ends_with(".tmp"))
        .collect()
}

#[test]
fn an_index_of_the_commit_bound_by_its_sidecar() -> Result {
    let w = world("bound")?;
    let status = git(&w.repo(), &["status", "--porcelain"])?;
    let worktrees = git(&w.repo(), &["worktree", "list"])?;
    let out = index(&w, &[], &[])?;
    assert!(out.status.success(), "{}", text(&out));
    let path = PathBuf::from(String::from_utf8(out.stdout.clone())?.trim());
    let head = git(&w.repo(), &["rev-parse", "HEAD"])?.trim().to_owned();
    assert_eq!(path, w.state().join(format!("symbols/repo-{head}.jsonl")));
    let bytes = fs::read(&path)?;
    let tags = String::from_utf8(bytes.clone())?;
    assert!(
        tags.contains("\"alpha\"") && tags.contains("\"beta\""),
        "{tags}"
    );
    assert!(
        !tags.contains("dirty_only") && !tags.contains("untracked_only"),
        "{tags}"
    );
    let meta: serde_json::Value =
        serde_json::from_slice(&fs::read(format!("{}.meta.json", path.display()))?)?;
    assert_eq!(meta["commit"], head.as_str());
    let digest = Command::new("sha256sum").arg(&path).output()?.stdout;
    let digest = String::from_utf8(digest)?;
    assert_eq!(
        meta["sha256"],
        digest.split_whitespace().next().unwrap_or_default()
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("2 tags"),
        "{}",
        text(&out)
    );
    assert_eq!(git(&w.repo(), &["status", "--porcelain"])?, status);
    assert_eq!(git(&w.repo(), &["worktree", "list"])?, worktrees);
    assert!(leftovers(&w).is_empty(), "{:?}", leftovers(&w));
    // The pair is what find def reads.
    let program = "@h := find def \"alpha\" in \"src/\" at \"HEAD\"; @e := retain @h around lines 0; @v := project @e [path, line]; emit @v budget_tokens 1000;";
    let (repo, state) = (
        w.repo().display().to_string(),
        w.state().display().to_string(),
    );
    let shown = path.display().to_string();
    let run = [
        "tuft", "run", "-", "--repo", &repo, "--state", &state, "--index", &shown,
    ];
    let found = call(&w, &run, &[], program)?;
    assert!(found.status.success(), "{}", text(&found));
    assert!(
        text(&found).contains("src/lib.rs") && text(&found).contains("coverage=partial"),
        "{}",
        text(&found)
    );
    // --json, and --out.
    let out = index(&w, &["--json", "--out", "tags.jsonl"], &[])?;
    let body: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(
        (body["commit"].as_str(), body["tags"].as_u64()),
        (Some(head.as_str()), Some(2))
    );
    assert!(
        body["ctags"]
            .as_str()
            .is_some_and(|c| c.starts_with("Universal Ctags")),
        "{body}"
    );
    assert_eq!(body["path"], w.0.join("tags.jsonl").display().to_string());
    Ok(())
}

#[test]
fn only_a_universal_ctags_with_json_is_run() -> Result {
    let w = world("refused")?;
    for (env, want) in [
        (
            vec![("FAKE_CTAGS", "exuberant")],
            "is not universal-ctags (--version says \"Exuberant Ctags 5.8\")",
        ),
        (
            vec![("FAKE_CTAGS", "nojson")],
            "does not list the json feature",
        ),
        (
            vec![("WRITFOLD_CTAGS", "/nonexistent/ctags")],
            "is not an executable file",
        ),
    ] {
        let out = index(&w, &[], &env)?;
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains(want), "{want}: {}", text(&out));
    }
    let empty = w.0.join("empty");
    fs::create_dir_all(&empty)?;
    let path = empty.display().to_string();
    let out = index(&w, &[], &[("WRITFOLD_CTAGS", ""), ("PATH", &path)])?;
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("no ctags on PATH"), "{}", text(&out));
    assert!(leftovers(&w).is_empty(), "{:?}", leftovers(&w));
    Ok(())
}

#[test]
fn a_hung_ctags_is_stopped_and_leaves_nothing() -> Result {
    let w = world("hang")?;
    let pid = w.0.join("pid").display().to_string();
    let started = Instant::now();
    let env = [
        ("FAKE_CTAGS", "hang"),
        ("FAKE_PID", pid.as_str()),
        ("WRITFOLD_CTAGS_TIMEOUT", "1"),
    ];
    let out = index(&w, &[], &env)?;
    assert!(started.elapsed() < Duration::from_secs(20));
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("timed out after 1 s"), "{}", text(&out));
    assert!(leftovers(&w).is_empty(), "{:?}", leftovers(&w));
    let child = fs::read_to_string(&pid)?;
    assert!(
        !Path::new("/proc").join(child.trim()).join("stat").exists(),
        "{child} outlived ctags"
    );
    Ok(())
}
