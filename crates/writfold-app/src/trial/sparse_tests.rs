//! The trial worktree holds only the edited path; the rest of the repository
//! is skip-worktree, so status, diff and the user's configuration stay clean.
use super::{Result, Trial, apply, approve, prepare};
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Root(PathBuf);
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args(args)
        .output()?;
    if out.status.success() {
        Ok(String::from_utf8(out.stdout)?)
    } else {
        Err(format!("git {args:?} failed").into())
    }
}

/// A repository with the edited file plus unrelated files, one of whose
/// names needs sparse-pattern escaping; returns (root, repo).
fn fixture(edited: &str) -> Result<(Root, PathBuf)> {
    let root = std::env::temp_dir().join(format!(
        ".writfold-sparse-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    let repo = root.join("repo");
    for (path, text) in [
        (edited, "fn main() {\n    run(1);\n}\n"),
        ("other/a.rs", "fn a() {}\n"),
        ("other/b [x]*.rs", "fn b() {}\n"),
    ] {
        let file = repo.join(path);
        fs::create_dir_all(file.parent().ok_or("parent")?)?;
        fs::write(file, text)?;
    }
    git(&repo, &["init", "-q"])?;
    git(&repo, &["add", "-A"])?;
    git(&repo, &["commit", "-qm", "fixture"])?;
    let recipe = format!(
        "edit \"{edited}\" {{\n  @n := one text \"run(1)\"\n  replace @n with \"run(2)\"\n}}\n"
    );
    fs::write(root.join("r.spl"), recipe)?;
    Ok((Root(root), repo))
}

fn materialized(worktree: &Path) -> Result<Vec<String>> {
    let listed = git(worktree, &["ls-files", "-t"])?;
    Ok(listed
        .lines()
        .filter_map(|l| l.strip_prefix("H "))
        .map(str::to_owned)
        .collect())
}

fn sparse_run(edited: &str) -> Result<()> {
    let (root, repo) = fixture(edited)?;
    let config = fs::read(repo.join(".git/config"))?;
    let state = root.0.join("state");
    Trial::init(&state)?;
    let p = prepare(&state, &repo, edited, &root.0.join("r.spl"), None)?;
    ensure(
        materialized(&p.worktree)? == [edited.to_owned()],
        "only the edited path is checked out",
    )?;
    ensure(!p.worktree.join("other/a.rs").exists(), "no other file")?;
    ensure(
        git(&p.worktree, &["status", "--porcelain"])?.is_empty(),
        "a clean status before apply",
    )?;
    approve(&state, &p.run)?;
    apply(&state, &p.run)?;
    ensure(
        git(&p.worktree, &["status", "--porcelain"])? == format!(" M {}\n", git_quoted(edited)),
        "only the edited path is modified after apply",
    )?;
    ensure(
        fs::read(repo.join(".git/config"))? == config,
        "the repository configuration is unchanged",
    )
}

fn ensure(condition: bool, what: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(format!("expected {what}").into())
    }
}

fn git_quoted(path: &str) -> String {
    if path.contains(' ') || path.contains('"') {
        format!("\"{path}\"")
    } else {
        path.to_owned()
    }
}

#[test]
fn only_the_edited_path_is_checked_out() -> Result<()> {
    sparse_run("src/main.rs")
}

#[test]
fn an_edited_path_with_pattern_characters_matches_only_itself() -> Result<()> {
    sparse_run("src/m[a]in *.rs")
}
