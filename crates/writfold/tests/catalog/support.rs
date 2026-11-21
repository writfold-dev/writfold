//! Shared by the catalog tests (#86): a scratch Git KB, the binary, and
//! the RFC's example declaration and input.
#![allow(
    dead_code,
    reason = "shared by several test binaries; each one uses only part of it"
)]
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const WRITFOLD: &str = env!("CARGO_BIN_EXE_writfold");

pub const DECL: &str = "schema = \"writfold-asset-catalog/1\"\ninput = \".writfold/catalog-inputs/equipment.json\"\nitems = \"/equipment\"\n\n[fields]\nid = \"/tag\"\nname = \"/label\"\nkind = \"/classification\"\naliases = \"/search/names\"\n";
pub const INPUT: &str = r#"{"equipment":[{"tag":"mix-station","label":"Mix station","classification":"fermenter","search":{"names":["brewdeck"]}},{"tag":"chiller","label":"Chiller","classification":"cooler","search":{"names":[]}}]}"#;

/// A failing case: what, the files, a word of the refusal.
pub type Case = (&'static str, Vec<(&'static str, String)>, &'static str);

pub struct Kb(pub PathBuf);

impl Drop for Kb {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn git(dir: &Path, args: &[&str]) -> Result<Output> {
    Ok(Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args(args)
        .output()?)
}

pub fn put(root: &Path, path: &str, text: &str) -> Result {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().ok_or("parent")?)?;
    std::fs::write(file, text)?;
    Ok(())
}

/// A KB with a hand-owned `demo` asset, a project, and `files`, committed.
pub fn kb(name: &str, files: &[(&str, &str)]) -> Result<Kb> {
    let dir = std::env::temp_dir().join(format!("writfold-catalog-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    put(
        &dir,
        "inventory/assets/demo.json",
        "{\"id\": \"demo\", \"kind\": \"service\", \"name\": \"Demo\"}\n",
    )?;
    put(&dir, "harbor/README.md", "# Harbor\n")?;
    for (path, text) in files {
        put(&dir, path, text)?;
    }
    git(&dir, &["init", "-q", "-b", "main"])?;
    commit(&dir, "kb")?;
    Ok(Kb(std::fs::canonicalize(dir)?))
}

pub fn commit(dir: &Path, message: &str) -> Result {
    git(dir, &["add", "-A"])?;
    git(dir, &["commit", "-q", "--allow-empty", "-m", message])?;
    Ok(())
}

pub fn kb_run(dir: &Path, args: &[&str]) -> Result<(String, String, i32)> {
    let out = Command::new(WRITFOLD)
        .arg("kb")
        .args(args)
        .current_dir(dir)
        .env("WRITFOLD_KB", dir)
        .env("WRITFOLD_SESSION", "s1")
        .env("HOME", dir.join(".home"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    Ok((
        stdout,
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    ))
}

/// `catalog diff NAME`, applied with `git apply` and committed.
pub fn apply(dir: &Path, name: &str) -> Result<String> {
    let (patch, err, exit) = kb_run(dir, &["catalog", "diff", name])?;
    assert_eq!(exit, 0, "{err}");
    let file = dir.join(".git/catalog.patch");
    std::fs::write(&file, &patch)?;
    if !patch.is_empty() {
        let out = git(dir, &["apply", "--check", ".git/catalog.patch"])?;
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        git(dir, &["apply", ".git/catalog.patch"])?;
    }
    commit(dir, "apply")?;
    Ok(patch)
}

pub fn base() -> [(&'static str, &'static str); 2] {
    [
        (".writfold/catalogs/brewery.toml", DECL),
        (".writfold/catalog-inputs/equipment.json", INPUT),
    ]
}
