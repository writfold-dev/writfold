//! The reliability pilot's operational trip: an agent exports
//! `WRITFOLD_STATE`, prepares a run, then calls `approve` from a fresh shell
//! where the export is gone. The refusal must name the directory it searched
//! and point at the right fix, and the printed `next:` commands must work
//! verbatim without the variable.
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
const WRITFOLD: &str = env!("CARGO_BIN_EXE_writfold");
const RECIPE: &str =
    "edit \"src/main.rs\" {\n  @n := one text \"run(1)\"\n  replace @n with \"run(2)\"\n}\n";

struct Root(PathBuf);

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) -> Result {
    let ok = Command::new("git")
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args(args)
        .status()?
        .success();
    if ok {
        Ok(())
    } else {
        Err(format!("git {args:?} failed").into())
    }
}

/// `writfold ARGS` with `HOME` pointing into the fixture, and
/// `WRITFOLD_STATE` set only when given.
fn writfold(root: &Path, env_state: Option<&Path>, args: &[&str]) -> Result<Output> {
    let mut command = Command::new(WRITFOLD);
    command.args(args).env("HOME", root.join("home"));
    match env_state {
        Some(state) => command.env("WRITFOLD_STATE", state),
        None => command.env_remove("WRITFOLD_STATE"),
    };
    Ok(command.output()?)
}

fn stdout(output: &Output) -> Result<String> {
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(String::from_utf8(output.stdout.clone())?)
}

/// The words after `next: writfold ` on the last line.
fn next_command(printed: &str) -> Result<Vec<String>> {
    let line = printed
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("next: writfold "))
        .ok_or("no next: line")?;
    Ok(line.split(' ').map(str::to_owned).collect())
}

#[test]
fn next_lines_survive_a_fresh_shell() -> Result {
    let root =
        Root(std::env::temp_dir().join(format!(".writfold-state-dir-{}", std::process::id())));
    let _ = fs::remove_dir_all(&root.0);
    fs::DirBuilder::new().mode(0o700).create(&root.0)?;
    let (repo, state) = (root.0.join("repo"), root.0.join("state"));
    fs::create_dir_all(repo.join("src"))?;
    fs::write(repo.join("src/main.rs"), "fn main() {\n    run(1);\n}\n")?;
    git(&repo, &["init", "-q"])?;
    git(&repo, &["add", "-A"])?;
    git(&repo, &["commit", "-qm", "fixture"])?;
    let recipe = root.0.join("r.spl");
    fs::write(&recipe, RECIPE)?;
    let (repo_arg, recipe_arg) = (repo.to_str().ok_or("path")?, recipe.to_str().ok_or("path")?);

    stdout(&writfold(&root.0, Some(&state), &["trial", "init"])?)?;
    let prepare = [
        "trial",
        "prepare",
        "--repo",
        repo_arg,
        "--file",
        "src/main.rs",
    ];
    let prepared = stdout(&writfold(
        &root.0,
        Some(&state),
        &[&prepare[..], &["--recipe", recipe_arg]].concat(),
    )?)?;
    let approve = next_command(&prepared)?;
    let run = approve.get(2).ok_or("no run on the next: line")?.clone();

    // The trip: same command, fresh shell, no WRITFOLD_STATE.
    let refused = writfold(&root.0, None, &["trial", "approve", &run])?;
    let message = String::from_utf8(refused.stderr)?;
    assert!(!refused.status.success());
    assert!(
        message.contains("the default; --state and WRITFOLD_STATE are unset"),
        "{message}"
    );
    assert!(
        message.contains("prepared in another state directory"),
        "{message}"
    );

    // A run the state has never seen is named as such, with the state.
    let unknown = writfold(&root.0, Some(&state), &["trial", "approve", "run.0.absent"])?;
    let message = String::from_utf8(unknown.stderr)?;
    assert!(
        message.contains("was not prepared in this trial state"),
        "{message}"
    );
    assert!(message.contains("(from WRITFOLD_STATE)"), "{message}");

    // The printed next: lines work verbatim in a fresh shell.
    let words: Vec<&str> = approve.iter().map(String::as_str).collect();
    let approved = stdout(&writfold(&root.0, None, &words)?)?;
    let apply = next_command(&approved)?;
    let words: Vec<&str> = apply.iter().map(String::as_str).collect();
    let applied = stdout(&writfold(&root.0, None, &words)?)?;
    assert!(
        applied.starts_with(&format!("applied run {run}")),
        "{applied}"
    );
    Ok(())
}
