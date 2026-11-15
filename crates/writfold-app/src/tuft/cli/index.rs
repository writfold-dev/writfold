//! `writfold tuft index [--repo DIR|NAME] [--at REF] [--out FILE] [--json]`:
//! a universal-ctags symbol index of one commit, with the sidecar
//! `find def|refs` verifies. The commit's tree is exported to a new
//! temporary directory under the Tuft state directory with the same pinned
//! Git reads Tuft searches with, so the user's work tree, index and
//! worktree list are never touched and uncommitted bytes cannot reach the
//! index. ctags runs there; its output is renamed into place, then the
//! sidecar (`<out>.meta.json`: the commit and the SHA-256 of the index) is
//! written. The temporary directory is removed whether it succeeds or not.
//! What is claimed: the index was produced from the bytes of that commit;
//! its completeness is ctags's, so symbol coverage stays partial.
use super::Context;
use super::args::Args;
use crate::trial::Result;
use crate::tuft::{ctags, files, state};
use std::path::{Path, PathBuf};
use writfold_kb::stale::repos;
use writfold_kb::workspace::{CANCELLED, GitTool, budgets};
use writfold_runtime::fs::RelativePath;
use writfold_runtime::git::{KbRepository, kb_commit};

/// A directory (or file) removed when dropped.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        if std::fs::remove_dir_all(&self.0).is_err() {
            let _ = std::fs::remove_file(&self.0);
        }
    }
}

/// What one index run produced.
pub(crate) struct Built {
    pub path: PathBuf,
    pub commit: String,
    pub sha256: String,
    pub tags: usize,
    pub ctags: String,
}

/// The repository, its name and the resolved commit.
fn resolve(args: &Args, context: &Context) -> Result<(PathBuf, String, String)> {
    let workspace = args.workspace(context)?;
    let selector = args.option("repo").unwrap_or(".");
    let repo = repos::locate(workspace.as_ref(), selector, &context.cwd).ok_or_else(|| {
        format!("no local repository {selector:?}; map it in <KB git dir>/writfold/repos or pass --repo DIR")
    })?;
    let git_dir = repos::git_dir(&repo).ok_or("repository has no Git directory")?;
    let tool = GitTool::locate()?;
    let git = tool.admit()?;
    let reads = KbRepository {
        git_dir: &git_dir,
        work_tree: Some(&repo),
    };
    let requested = args.option("at").unwrap_or("HEAD");
    let commit = kb_commit(&git, reads, requested, budgets()?, &CANCELLED)?.ok_or_else(|| {
        format!("at {requested:?} names no local commit; check the ref with git rev-parse --verify")
    })?;
    let name = repos::mapped_name(workspace.as_ref(), &repo).unwrap_or_else(|| {
        repo.file_name()
            .map_or_else(|| "repo".into(), |n| n.to_string_lossy().into_owned())
    });
    Ok((repo, name, commit.as_str().to_owned()))
}

/// Write every blob of `commit` under `dir`.
fn export(repo: &Path, commit: &str, dir: &Path) -> Result<usize> {
    let git_dir = repos::git_dir(repo).ok_or("repository has no Git directory")?;
    let tool = GitTool::locate()?;
    let pin = crate::tuft::commit(commit)?;
    let snapshot = files::snapshot(&tool.admit()?, &git_dir, &pin, "")?;
    for (path, bytes) in &snapshot {
        RelativePath::new(path)?;
        let file = dir.join(path);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file, bytes)?;
    }
    Ok(snapshot.len())
}

/// Rename `from` to `to` after syncing it.
fn publish(from: &Path, to: &Path) -> Result<()> {
    std::fs::File::open(from)?.sync_all()?;
    std::fs::rename(from, to)?;
    Ok(())
}

/// Build the index; see the module documentation.
pub(crate) fn build(args: &Args, context: &Context) -> Result<Built> {
    let program = ctags::find()?;
    let version = ctags::probe(&program, &context.cwd)?;
    let (repo, name, commit) = resolve(args, context)?;
    let (state::Chosen::Trial(state) | state::Chosen::Kb(state)) = args.state(context)?;
    let symbols = state.join("symbols");
    std::fs::create_dir_all(&symbols)?;
    let out = match args.option("out") {
        Some(out) => context.cwd.join(out),
        None => symbols.join(format!("{name}-{commit}.jsonl")),
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let scratch = Scratch(symbols.join(format!(".export-{}-{nanos}", std::process::id())));
    std::fs::create_dir(&scratch.0)?;
    export(&repo, &commit, &scratch.0)?;
    let mut partial = out.as_os_str().to_os_string();
    partial.push(".tmp");
    let partial = Scratch(PathBuf::from(partial));
    let written = partial.0.to_string_lossy().into_owned();
    let flags = [
        "--options=NONE",
        "--output-format=json",
        "--fields=+nr",
        "--extras=+r",
    ];
    let mut args_list: Vec<&str> = flags.to_vec();
    args_list.extend(["-R", "-f", &written, "."]);
    ctags::run(&program, &args_list, &scratch.0, ctags::timeout())?;
    let bytes = std::fs::read(&partial.0)?;
    let sha256 = tuft::digest_hex(&bytes);
    let tags = String::from_utf8_lossy(&bytes)
        .lines()
        .filter(|l| l.contains("\"_type\": \"tag\"") || l.contains("\"_type\":\"tag\""))
        .count();
    publish(&partial.0, &out)?;
    let mut meta = out.as_os_str().to_os_string();
    meta.push(".meta.json");
    let meta = PathBuf::from(meta);
    let sidecar = Scratch(meta.with_extension("json.tmp"));
    let body = serde_json::json!({"commit": commit, "sha256": sha256});
    std::fs::write(&sidecar.0, format!("{body}\n"))?;
    publish(&sidecar.0, &meta)?;
    Ok(Built {
        path: out,
        commit,
        sha256,
        tags,
        ctags: version,
    })
}

/// `tuft index`'s stdout (the index path, or the JSON object) and its
/// stderr notes.
pub(crate) fn run(args: &Args, context: &Context) -> Result<(String, Vec<String>)> {
    let built = build(args, context)?;
    let path = built.path.display().to_string();
    if args.format == tuft::emit::Format::Json {
        let body = serde_json::json!({"path": path, "commit": built.commit, "sha256": built.sha256, "tags": built.tags, "ctags": built.ctags});
        return Ok((body.to_string(), Vec::new()));
    }
    let note = format!(
        "indexed {} at {}: {} tags, sha256 {}, by {}; symbol coverage stays partial",
        path, built.commit, built.tags, built.sha256, built.ctags
    );
    Ok((path, vec![note]))
}
