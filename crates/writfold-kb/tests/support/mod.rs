//! A synthetic world for KB tests: the invented fixture KB committed to a
//! fresh repository, and a companion code repository whose history
//! exercises every citation verdict. Authors, committers and dates are
//! fixed, so commit IDs (and so outputs) are stable.
#![allow(dead_code)]
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use writfold_kb::{Request, Verb};
use writfold_sources::citations::normalize_span;

pub type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// 2026-11-11T00:00:00Z, the fixed clock of every test.
pub const NOW: i64 = 1_794_355_200;

static NEXT: AtomicUsize = AtomicUsize::new(0);

pub struct World {
    pub base: PathBuf,
    pub kb: PathBuf,
    pub code: PathBuf,
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

pub fn git(dir: &Path, args: &[&str]) -> TestResult<String> {
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
    Ok(String::from_utf8(output.stdout)?.trim_end().to_owned())
}

fn write(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

const LEDGER: &str = "// ledger writes\npub fn write(rows: &[Row]) {\n    for row in rows {\n        append(row);\n    }\n}\n";
const BATCH: &str = "pub fn batch(rows: Vec<Row>) -> Vec<Vec<Row>> {\n    rows.chunks(500)\n        .map(<[Row]>::to_vec)\n        .collect()\n}\n";
const RETRY: &str = "pub fn retry() {\n    let attempts = 3;\n    backoff(attempts);\n}\n";
const LEGACY: &str = "pub fn legacy() {}\npub fn old() {}\n";
const DB: &str = "pub struct Db;\nimpl Db {\n    pub fn open() -> Self { Db }\n}\n";

/// The code repository: one commit cited by the KB, then one that shifts,
/// edits, deletes and renames. Returns the cited commit.
fn code_repo(dir: &Path) -> TestResult<String> {
    std::fs::create_dir_all(dir)?;
    git(dir, &["init", "-q"])?;
    for (path, text) in [
        ("src/ledger.rs", LEDGER),
        ("src/batch.rs", BATCH),
        ("src/retry.rs", RETRY),
        ("src/legacy.rs", LEGACY),
        ("src/db.rs", DB),
    ] {
        write(&dir.join(path), text)?;
    }
    git(dir, &["add", "-A"])?;
    git(dir, &["commit", "-q", "-m", "first"])?;
    let cited = git(dir, &["rev-parse", "HEAD"])?;
    write(
        &dir.join("src/batch.rs"),
        &format!("// batching\n// limit 500\n\n{BATCH}"),
    )?;
    write(&dir.join("src/retry.rs"), &RETRY.replace("3", "5"))?;
    std::fs::remove_file(dir.join("src/legacy.rs"))?;
    std::fs::create_dir_all(dir.join("src/store"))?;
    std::fs::rename(dir.join("src/db.rs"), dir.join("src/store/db.rs"))?;
    git(dir, &["add", "-A"])?;
    git(dir, &["commit", "-q", "-m", "second"])?;
    Ok(cited)
}

/// Fill `{{REV}}`, `{{BLOB:path}}` and `{{SPAN:path:a-b}}` from the cited
/// commit of the code repository.
fn fill(text: &str, code: &Path, rev: &str) -> TestResult<String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let end = rest[start..].find("}}").ok_or("unclosed placeholder")? + start;
        let key = &rest[start + 2..end];
        let value = if key == "REV" {
            rev.to_owned()
        } else if let Some(path) = key.strip_prefix("BLOB:") {
            git(code, &["rev-parse", &format!("{rev}:{path}")])?
        } else if let Some(spec) = key.strip_prefix("SPAN:") {
            let (path, range) = spec.rsplit_once(':').ok_or("span placeholder")?;
            let (a, b) = range.split_once('-').ok_or("span range")?;
            let (a, b): (usize, usize) = (a.parse()?, b.parse()?);
            let content = git(code, &["show", &format!("{rev}:{path}")])?;
            let lines: Vec<&str> = content.lines().skip(a - 1).take(b - a + 1).collect();
            normalize_span(&lines)
        } else {
            return Err(format!("unknown placeholder {key}").into());
        };
        out.push_str(&value);
        rest = &rest[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

fn copy_tree(from: &Path, to: &Path, code: &Path, rev: &str) -> TestResult {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target, code, rev)?;
        } else {
            write(
                &target,
                &fill(&std::fs::read_to_string(entry.path())?, code, rev)?,
            )?;
        }
    }
    Ok(())
}

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/kb")
        .join(name)
}

/// A fresh world; `overlays` are extra fixture trees copied over `basic`.
pub fn world(overlays: &[&str]) -> TestResult<World> {
    let base = std::env::temp_dir().join(format!(
        "writfold-kb-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&base);
    let (kb, code) = (base.join("kb"), base.join("demo-service"));
    let rev = code_repo(&code)?;
    for name in std::iter::once("basic").chain(overlays.iter().copied()) {
        copy_tree(&fixture(name), &kb, &code, &rev)?;
    }
    git(&kb, &["init", "-q"])?;
    git(&kb, &["add", "-A"])?;
    git(&kb, &["commit", "-q", "-m", "fixture KB"])?;
    let map = kb.join(".git/writfold/repos");
    write(
        &map,
        &format!("# local repository map\ndemo-service {}\n", code.display()),
    )?;
    Ok(World {
        base: std::fs::canonicalize(&base)?,
        kb: std::fs::canonicalize(&kb)?,
        code,
    })
}

impl World {
    pub fn request(&self, verb: Verb) -> Request {
        Request {
            verb,
            kb: Some(self.kb.clone()),
            cwd: self.base.clone(),
            budget: writfold_kb::DEFAULT_BUDGET,
            cursor: None,
            json: false,
            now: Some(NOW),
            probe: writfold_kb::write::no_probe,
        }
    }

    /// Run, and return the output with this world's paths made stable.
    pub fn run(&self, request: &Request) -> TestResult<(String, u8)> {
        let output = writfold_kb::run(request)?;
        let text = output
            .text
            .replace(&self.base.display().to_string(), "<WORLD>");
        Ok((text, output.exit))
    }
}
