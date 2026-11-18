//! Two clones of one repository in one temporary directory, for
//! `kb_second_clone.rs`.
#![allow(
    dead_code,
    reason = "shared by several test binaries; each one uses only part of it"
)]
use std::path::{Path, PathBuf};
use std::process::Command;

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

pub const WRITFOLD: &str = env!("CARGO_BIN_EXE_writfold");

pub struct Base(pub PathBuf);

impl Drop for Base {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Base {
    pub fn new(name: &str) -> Result<Self> {
        let dir = std::env::temp_dir().join(format!("writfold-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        Ok(Self(std::fs::canonicalize(&dir)?))
    }

    pub fn command(&self, program: &str, dir: &Path) -> Command {
        let mut c = Command::new(program);
        c.current_dir(dir)
            .env("HOME", &self.0)
            .env("XDG_CONFIG_HOME", &self.0)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
            .env_remove("WRITFOLD_KB")
            .env_remove("WRITFOLD_RUN");
        c
    }

    pub fn git(&self, dir: &Path, args: &[&str]) -> Result<String> {
        let out = self
            .command("git", dir)
            .args([
                "-c",
                "init.defaultBranch=main",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(format!("git {args:?}: {err}").into());
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_owned())
    }

    /// `writfold <args>` in `dir`: stdout and stderr, and the exit code.
    pub fn run(&self, dir: &Path, args: &[&str]) -> Result<(String, i32)> {
        let out = self.command(WRITFOLD, dir).args(args).output()?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        Ok((text, out.status.code().unwrap_or(-1)))
    }

    pub fn kb(&self, dir: &Path, args: &[&str]) -> Result<(String, i32)> {
        let mut all = vec!["kb"];
        all.extend_from_slice(args);
        self.run(dir, &all)
    }

    /// Where the gate's check last ran.
    pub fn ran_in(&self) -> Result<PathBuf> {
        Ok(PathBuf::from(
            std::fs::read_to_string(self.0.join("ran-in"))?.trim(),
        ))
    }
}

pub fn put(path: &Path, text: &str) -> Result {
    std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Clone A's files: the source, the check (recording where it ran in
/// `ran`), the `unit` gate, the `change` plan and the `review` guard.
fn seed(a: &Path, ran: &Path) -> Result {
    put(&a.join("src/l.rs"), "500\n")?;
    put(
        &a.join("check.sh"),
        &format!(
            "#!/bin/sh\npwd > '{}'\ngrep -q 500 src/l.rs\n",
            ran.display()
        ),
    )?;
    put(
        &a.join(".writfold/gates/unit.toml"),
        "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"sh\", \"check.sh\"]\nworkdir = \".\"\ntimeout_seconds = 60\nscope = [\"src/**\"]\n",
    )?;
    put(
        &a.join(".writfold/plans/change.toml"),
        "schema = \"writfold-plan/1\"\nname = \"change\"\nsteps = [\"build: unit\"]\n",
    )?;
    put(
        &a.join(".writfold/guards/review.toml"),
        "schema = \"writfold-guard-rule/1\"\nid = \"review\"\nevent = \"pre-tool-use\"\ncommands = [\"git commit\"]\nrequire = [\"gates.met\"]\naction = \"deny\"\n",
    )?;
    Ok(())
}

/// Clone A of a fresh origin: `src/l.rs` holds 500, the `unit` gate (at
/// the repository root) checks for it and records where it ran, a `change`
/// plan owes it and a guard requires it. The KB is at `kb` inside the
/// repository; a run is started there, the plan set and the receipts
/// pushed. Returns clone A, clone B (cloned afterwards) and the run.
pub fn two_clones(base: &Base, kb: &str, run_gate: bool) -> Result<(PathBuf, PathBuf, String)> {
    let origin = base.0.join("o.git");
    base.git(&base.0, &["init", "-q", "--bare", "o.git"])?;
    let a = base.0.join("a/demo");
    std::fs::create_dir_all(&a)?;
    base.git(&a, &["init", "-q"])?;
    base.git(&a, &["remote", "add", "origin", &origin.to_string_lossy()])?;
    seed(&a, &base.0.join("ran-in"))?;
    let akb = a.join(kb);
    std::fs::create_dir_all(&akb)?;
    let (text, code) = base.kb(&akb, &["init"])?;
    assert_eq!(code, 0, "{text}");
    base.git(&a, &["add", "-A"])?;
    base.git(&a, &["commit", "-q", "-m", "start"])?;
    base.git(&a, &["push", "-q", "-u", "origin", "main"])?;
    let args = [
        "checkpoint",
        "--new-run",
        "--project",
        "demo",
        "--task",
        "t",
    ];
    let (text, code) = base.kb(&akb, &[&args[..], &["--note", "n"]].concat())?;
    assert_eq!(code, 0, "{text}");
    let run = std::fs::read_dir(akb.join("runs"))?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .next()
        .ok_or("no run")?;
    let (text, code) = base.kb(&akb, &["plan", "set", "change", "--run", &run])?;
    assert_eq!(code, 0, "{text}");
    if run_gate {
        let (text, code) = base.kb(&akb, &["gate", "run", "unit", "--run", &run])?;
        assert_eq!(code, 0, "{text}");
    }
    base.git(&a, &["add", "-A"])?;
    base.git(&a, &["commit", "-q", "-m", "run"])?;
    base.git(&a, &["push", "-q"])?;
    std::fs::create_dir_all(base.0.join("b"))?;
    base.git(
        &base.0.join("b"),
        &["clone", "-q", &origin.to_string_lossy(), "demo"],
    )?;
    let b = base.0.join("b/demo");
    put(&b.join(".git/writfold/current-run"), &run)?;
    Ok((a, b, run))
}
