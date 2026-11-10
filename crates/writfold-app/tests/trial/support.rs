//! A throwaway Git repository and trial state for each test.
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use writfold_app::trial::Trial;

pub type Result<T = (), E = Box<dyn std::error::Error + Send + Sync>> = std::result::Result<T, E>;
static NEXT: AtomicU64 = AtomicU64::new(0);

pub const CLIENT: &str = "use crate::transport::{open, Conn, Error};

pub fn connect(url: &str) -> Result<Conn, Error> {
    let conn = open(url)?;
    conn.handshake()?;
    Ok(conn)
}

pub fn probe(url: &str) -> bool {
    open(url).is_ok()
}

pub fn reconnect(conn: &mut Conn, url: &str) -> Result<(), Error> {
    let fresh = open(url)?;
    *conn = fresh;
    Ok(())
}
";

const WRAP: &str = r#"edit "src/client.rs" {
  @binding := one line "    let conn = open(url)?;"
  @call := one text "open(url)" within @binding
  replace @call with ["with_retry(3, || ", keep(@call), ")"]
}
"#;
const AMBIGUOUS: &str = r#"edit "src/client.rs" {
  @call := one text "open(url)"
  replace @call with ["with_retry(3, || ", keep(@call), ")"]
}
"#;

// `whole @line` selects the line with its terminator, so deleting it leaves no blank line.
const WHOLE: &str = r#"edit "src/client.rs" {
  @handshake := one line "    conn.handshake()?;"
  delete whole @handshake
}
"#;
// `from @a to @b` spans both captures and the bytes between them.
const FROM_TO: &str = r#"edit "src/client.rs" {
  @first := one text "let fresh = open(url)?;"
  @last := one text "*conn = fresh;"
  @swap := from @first to @last
  replace @swap with ["*conn = open(url)?;"]
}
"#;

pub fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args(args)
        .output()?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned().into());
    }
    Ok(String::from_utf8(out.stdout)?)
}

pub struct Setup {
    root: PathBuf,
    pub repo: PathBuf,
    pub state: PathBuf,
}

impl Setup {
    pub fn new() -> Result<Self> {
        let root = std::env::temp_dir().join(format!(
            ".writfold-trial-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&root)?;
        let repo = root.join("repo");
        fs::create_dir_all(repo.join("src"))?;
        fs::write(repo.join("src/client.rs"), CLIENT)?;
        git(&repo, &["init", "-q"])?;
        git(&repo, &["add", "-A"])?;
        git(&repo, &["commit", "-qm", "fixture"])?;
        let state = root.join("state");
        Trial::init(&state)?;
        Ok(Self { root, repo, state })
    }

    pub fn recipe(&self, name: &str) -> Result<PathBuf> {
        let path = self.root.join(format!("{name}.spl"));
        let body = match name {
            "wrap" => WRAP,
            "whole" => WHOLE,
            "from-to" => FROM_TO,
            _ => AMBIGUOUS,
        };
        fs::write(&path, body)?;
        Ok(path)
    }
}

impl Drop for Setup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
