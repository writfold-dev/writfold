//! A watch world: the invented fixture KB committed with fixed dates, a
//! second run that is closed, and four gates over the KB repository
//! itself (satisfied, failed, stale and open) with receipts recorded by
//! `gate run`.
#![allow(dead_code)]
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use writfold_kb::gate::GateVerb;
use writfold_kb::{Request, Verb};
use writfold_tui::source::Source;

pub type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// 2026-11-11T00:00:00Z.
pub const NOW: i64 = 1_794_355_200;
pub const RUN: &str = "run-20261101T090000Z-a1b2c3";
pub const CLOSED: &str = "run-20261104T080000Z-d4e5f6";

pub struct World {
    pub base: PathBuf,
    pub kb: PathBuf,
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

pub fn git(dir: &Path, args: &[&str]) -> TestResult {
    let output = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("GIT_AUTHOR_DATE", "2026-11-05T12:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-11-05T12:00:00Z")
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
    Ok(())
}

fn copy(from: &Path, to: &Path) -> TestResult {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn write(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn define(kb: &Path, id: &str, argv: &str, scope: &str) -> TestResult {
    write(
        &kb.join(format!(".writfold/gates/{id}.toml")),
        &format!(
            "schema = \"writfold-gate/1\"\nid = \"{id}\"\nversion = 1\nclass = \"mechanical\"\nargv = {argv}\ntimeout_seconds = 60\nscope = {scope}\n"
        ),
    )
}

impl World {
    pub fn source(&self) -> Source {
        Source {
            kb: Some(self.kb.clone()),
            cwd: self.kb.clone(),
            now: Some(NOW),
        }
    }

    pub fn request(&self, verb: Verb, json: bool, at: i64) -> Request {
        Request {
            verb,
            kb: Some(self.kb.clone()),
            cwd: self.kb.clone(),
            budget: writfold_kb::MAX_BUDGET,
            cursor: None,
            json,
            now: Some(at),
            probe: writfold_kb::write::no_probe,
        }
    }

    /// A verb's text (or JSON) output and exit code, as the CLI prints it.
    pub fn verb(&self, verb: Verb, json: bool) -> TestResult<(String, u8)> {
        let output = writfold_kb::run(&self.request(verb, json, NOW))?;
        Ok((output.text, output.exit))
    }

    fn gate(&self, id: &str, at: i64) -> TestResult {
        let verb = Verb::Gate(GateVerb::Run {
            id: id.into(),
            run: Some(RUN.into()),
            repo: None,
        });
        writfold_kb::run(&self.request(verb, false, at))?;
        Ok(())
    }
}

/// The world, with `pass` satisfied, `fail` failed, `moved` stale (its
/// scope edited after its receipt) and `todo` open.
pub fn world(name: &str) -> TestResult<World> {
    let base = std::env::temp_dir().join(format!("writfold-tui-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let kb = base.join("kb");
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/kb/basic"),
        &kb,
    )?;
    let closed = kb.join("runs").join(CLOSED);
    write(
        &closed.join("manifest.json"),
        &format!(
            "{{\n  \"run_id\": \"{CLOSED}\",\n  \"task\": \"Retire the legacy flush path\",\n  \"project\": \"harbor\",\n  \"mode\": \"review\",\n  \"created\": \"2026-11-04T08:00:00Z\",\n  \"state\": \"open\"\n}}\n"
        ),
    )?;
    write(
        &closed.join("checkpoint.md"),
        "\n## 2026-11-04T09:00:00Z\n\nLegacy path removed; nothing left to do.\n",
    )?;
    write(
        &closed.join("writfold-run.json"),
        "{\n  \"state\": \"closed\",\n  \"closed\": \"2026-11-04T09:05:00Z\"\n}\n",
    )?;
    define(&kb, "pass", "[\"true\"]", "[\"inventory/**\"]")?;
    define(&kb, "fail", "[\"false\"]", "[\"inventory/**\"]")?;
    define(&kb, "moved", "[\"true\"]", "[\"harbor/**\"]")?;
    define(&kb, "todo", "[\"true\"]", "[\"signatures/**\"]")?;
    git(&kb, &["init", "-q"])?;
    git(&kb, &["add", "-A"])?;
    git(&kb, &["commit", "-q", "-m", "fixture KB"])?;
    let world = World {
        base: std::fs::canonicalize(&base)?,
        kb: std::fs::canonicalize(&kb)?,
    };
    for (n, id) in ["pass", "fail", "moved"].iter().enumerate() {
        world.gate(id, NOW + 60 * i64::try_from(n)?)?;
    }
    let status = world.kb.join("harbor/STATUS.md");
    let text = std::fs::read_to_string(&status)?;
    std::fs::write(&status, format!("{text}\nA later note.\n"))?;
    Ok(world)
}

/// Receipt names end in a UTC time and six random hex digits.
pub fn stable(text: &str) -> String {
    let names = regex::Regex::new(r"-(\d{8}T\d{6}Z)-[0-9a-f]{6}\.json");
    match names {
        Ok(re) => re.replace_all(text, "-$1-XXXXXX.json").into_owned(),
        Err(_) => text.to_owned(),
    }
}
