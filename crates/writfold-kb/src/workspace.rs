//! Finding the KB root, its repository, and the Git executable.
use crate::error::{KbError, Result};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use writfold_domain::{ByteCount, ContentDigest};
use writfold_runtime::fs::{ContainedRoot, FsError, PinnedFile, RelativePath};
use writfold_runtime::git::{GitBudgets, GitExecutable, KbRepository};
use writfold_runtime::process::{ProcessError, ProcessSupervisor};

pub static CANCELLED: AtomicBool = AtomicBool::new(false);
static SUPERVISOR: LazyLock<std::result::Result<ProcessSupervisor, ProcessError>> =
    LazyLock::new(ProcessSupervisor::enable);
const EXECUTABLE_LIMIT: u64 = 256 << 20;

/// The process-wide supervisor every spawned command runs under.
pub fn supervisor() -> Result<&'static ProcessSupervisor> {
    SUPERVISOR
        .as_ref()
        .map_err(|error| KbError::Usage(format!("process supervision unavailable: {error}")))
}

/// The repository a KB root lives in. `prefix` is the root's path inside the
/// work tree, empty or ending in `/`.
#[derive(Clone, Debug)]
pub struct Repo {
    pub work_tree: PathBuf,
    pub git_dir: PathBuf,
    pub prefix: String,
}

impl Repo {
    pub fn reads(&self) -> KbRepository<'_> {
        KbRepository {
            git_dir: &self.git_dir,
            work_tree: Some(&self.work_tree),
        }
    }

    /// The pathspec naming the whole KB root.
    pub fn pathspec(&self) -> &str {
        if self.prefix.is_empty() {
            "."
        } else {
            self.prefix.trim_end_matches('/')
        }
    }
}

/// A located KB: its root and, when it is under Git, its repository.
#[derive(Clone, Debug)]
pub struct Workspace {
    pub root: PathBuf,
    pub repo: Option<Repo>,
}

fn is_root(path: &Path) -> bool {
    path.join("inventory").join("assets").is_dir()
}

/// `--kb DIR`, then `$WRITFOLD_KB`, then the nearest ancestor of `cwd`
/// holding `inventory/assets/`, then the prototype configuration's `root`.
pub fn discover(explicit: Option<&Path>, cwd: &Path) -> Result<Workspace> {
    let named = explicit
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os("WRITFOLD_KB").map(PathBuf::from));
    let root = match named {
        Some(path) if is_root(&path) => path,
        Some(path) => {
            return Err(KbError::NoKb(format!(
                "{} holds no inventory/assets/ directory",
                path.display()
            )));
        }
        None => cwd
            .ancestors()
            .find(|dir| is_root(dir))
            .map(Path::to_path_buf)
            .or_else(configured_root)
            .ok_or_else(|| {
                KbError::NoKb(
                    "no KB found: pass --kb DIR, set WRITFOLD_KB, or run inside a KB".into(),
                )
            })?,
    };
    let root = std::fs::canonicalize(root)?;
    let repo = find_repo(&root)?;
    Ok(Workspace { root, repo })
}

/// `root = "<path>"` from the prototype's configuration file, if present:
/// `$KCTX_CONFIG`, else `$XDG_CONFIG_HOME` or `~/.config`, then
/// `kctx/config.toml`. Every other key is ignored.
fn configured_root() -> Option<PathBuf> {
    let file = std::env::var_os("KCTX_CONFIG")
        .map(PathBuf::from)
        .or_else(|| {
            let base = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
            Some(base.join("kctx").join("config.toml"))
        })?;
    let text = std::fs::read_to_string(file).ok()?;
    let value = text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "root").then(|| value.trim().to_owned())
    })?;
    let path: String = serde_json::from_str(&value).ok()?;
    let path = match path.strip_prefix("~/") {
        Some(rest) => PathBuf::from(std::env::var_os("HOME")?).join(rest),
        None => PathBuf::from(path),
    };
    is_root(&path).then_some(path)
}

/// The nearest enclosing work tree and its Git directory (a `.git`
/// directory, or the one a `.git` file points at).
pub fn find_repo(root: &Path) -> Result<Option<Repo>> {
    for dir in root.ancestors() {
        let dot = dir.join(".git");
        let git_dir = if dot.is_dir() {
            dot
        } else if dot.is_file() {
            let text = std::fs::read_to_string(&dot)?;
            let target = text
                .lines()
                .find_map(|line| line.strip_prefix("gitdir: "))
                .ok_or_else(|| KbError::NoKb(format!("{} is not a gitdir file", dot.display())))?;
            dir.join(target.trim())
        } else {
            continue;
        };
        let relative = root
            .strip_prefix(dir)
            .map_err(|_| KbError::NoKb("root outside work tree".into()))?;
        let mut prefix = String::new();
        for part in relative.components() {
            let part = part
                .as_os_str()
                .to_str()
                .ok_or_else(|| KbError::NoKb("non-UTF-8 KB path".into()))?;
            prefix.push_str(part);
            prefix.push('/');
        }
        return Ok(Some(Repo {
            work_tree: dir.to_path_buf(),
            git_dir: std::fs::canonicalize(git_dir)?,
            prefix,
        }));
    }
    Ok(None)
}

/// The Git executable this process uses: `$WRITFOLD_GIT`, else `git` on
/// `PATH`. Its bytes are observed once and then admitted unchanged.
pub struct GitTool {
    file: PinnedFile,
    digest: ContentDigest,
}

impl GitTool {
    pub fn locate() -> Result<Self> {
        let path = match std::env::var_os("WRITFOLD_GIT") {
            Some(path) => PathBuf::from(path),
            None => {
                let search = std::env::var_os("PATH")
                    .ok_or_else(|| KbError::Usage("PATH is not set; cannot find git".into()))?;
                std::env::split_paths(&search)
                    .map(|dir| dir.join("git"))
                    .find(|candidate| candidate.is_file())
                    .ok_or_else(|| {
                        KbError::Usage("git was not found on PATH (or set WRITFOLD_GIT)".into())
                    })?
            }
        };
        let path = std::fs::canonicalize(path)?;
        let (Some(dir), Some(name)) = (path.parent(), path.file_name().and_then(|n| n.to_str()))
        else {
            return Err(KbError::Usage("unusable git path".into()));
        };
        let name = RelativePath::new(name).map_err(FsError::from)?;
        let file = ContainedRoot::open(dir)?.pin(&name)?;
        let digest = GitExecutable::observe(&file, ByteCount::new(EXECUTABLE_LIMIT)?)?;
        Ok(Self { file, digest })
    }

    pub fn admit(&self) -> Result<GitExecutable<'_>> {
        let supervisor = supervisor()?;
        Ok(GitExecutable::admit(
            &self.file,
            &self.digest,
            ByteCount::new(EXECUTABLE_LIMIT)?,
            supervisor,
        )?)
    }
}

/// Output limits and the deadline for one KB Git read.
pub fn budgets() -> Result<GitBudgets> {
    Ok(GitBudgets::new(
        ByteCount::new(512 << 20)?,
        ByteCount::new(1 << 16)?,
        ByteCount::new(64 << 20)?,
        Duration::from_secs(60),
    ))
}
