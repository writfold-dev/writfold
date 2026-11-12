//! The subject of a gate: a repository, its HEAD commit, and the content of
//! the paths in a gate's scope. The scope digest is the blob comparison of
//! `stale` (design 6.2, step 3) applied to a set of paths: the blob ID of
//! every in-scope path, taking a dirty file's work-tree bytes as the blob
//! Git would store, so committing those bytes later leaves the digest
//! unchanged, and any edit, addition or removal in scope changes it.
use crate::error::{KbError, Result};
use crate::stale::repos;
use crate::workspace::{CANCELLED, Workspace, budgets, find_repo};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use writfold_core::gate::in_scope;
use writfold_domain::GitObjectId;
use writfold_runtime::git::{
    GitExecutable, KbRepository, gate_hash_paths, gate_ls_tree, kb_config, kb_diff_tree, kb_status,
};

/// A located subject repository.
#[derive(Clone, Debug)]
pub struct Subject {
    /// The top of its work tree.
    pub top: PathBuf,
    pub git_dir: PathBuf,
    /// The KB map's name for it, else its origin's last path segment, else
    /// its directory name: stable across worktrees of one repository.
    pub name: String,
}

impl Subject {
    fn reads(&self) -> KbRepository<'_> {
        KbRepository {
            git_dir: &self.git_dir,
            work_tree: Some(&self.top),
        }
    }
}

/// The subject: `--repo`, else the repository enclosing the working
/// directory unless it is the KB's own, else the run's recorded worktree
/// when it exists here, else the KB's repository. `None` outside any.
pub fn locate(
    ws: &Workspace,
    git: &GitExecutable<'_>,
    explicit: Option<&Path>,
    cwd: &Path,
    recorded: Option<&str>,
) -> Result<Option<Subject>> {
    let found = |dir: &Path| -> Result<Option<crate::workspace::Repo>> {
        match std::fs::canonicalize(dir) {
            Ok(dir) => find_repo(&dir),
            Err(_) => Ok(None),
        }
    };
    let repo = if let Some(dir) = explicit {
        let dir = cwd.join(dir);
        Some(found(&dir)?.ok_or_else(|| {
            KbError::Refused(format!(
                "--repo {} is not in a Git work tree",
                dir.display()
            ))
        })?)
    } else {
        let here = found(cwd)?;
        let kb_top = ws.repo.as_ref().map(|r| r.work_tree.clone());
        match here {
            Some(r) if Some(&r.work_tree) != kb_top.as_ref() => Some(r),
            here => match recorded.map(Path::new).filter(|p| p.is_dir()) {
                Some(dir) => found(dir)?.or(here),
                None => here,
            },
        }
    };
    let Some(repo) = repo else {
        return Ok(None);
    };
    let reads = KbRepository {
        git_dir: &repo.git_dir,
        work_tree: None,
    };
    let origin = kb_config(git, reads, "remote.origin.url", budgets()?, &CANCELLED)?;
    let name = repos::mapped_name(Some(ws), &repo.work_tree)
        .or_else(|| {
            origin
                .as_deref()
                .and_then(|url| url.trim_end_matches('/').rsplit(['/', ':']).next())
                .map(|n| n.trim_end_matches(".git").to_owned())
                .filter(|n| !n.is_empty())
        })
        .or_else(|| {
            repo.work_tree
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "repo".into());
    Ok(Some(Subject {
        top: repo.work_tree,
        git_dir: repo.git_dir,
        name,
    }))
}

/// The commit, its tree, and the work tree's dirty paths with the blob ID
/// each would have (`None`: deleted).
pub struct Snapshot {
    pub commit: GitObjectId,
    tree: Vec<(String, String)>,
    dirty: Vec<(String, Option<String>)>,
}

/// One gate's view of a snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scoped {
    /// `sha256:<hex>` over the in-scope paths and their blob IDs.
    pub digest: String,
    pub paths: usize,
    pub dirty: usize,
    /// `sha256:<hex>` over the dirty in-scope paths, when there are any.
    pub dirty_digest: Option<String>,
}

/// Observe HEAD, its tree and the dirty files of the subject. Only dirty
/// files `wanted` accepts are hashed.
pub fn snapshot(
    git: &GitExecutable<'_>,
    subject: &Subject,
    wanted: &dyn Fn(&str) -> bool,
) -> Result<Snapshot> {
    let status = kb_status(git, subject.reads(), ".", budgets()?, &CANCELLED)?;
    let commit = status
        .head
        .ok_or_else(|| KbError::Refused(format!("{} has no commit yet", subject.top.display())))?;
    let tree = gate_ls_tree(git, &subject.git_dir, &commit, budgets()?, &CANCELLED)?
        .into_iter()
        .map(|e| (e.path, e.id))
        .collect();
    let mut present = Vec::new();
    let mut dirty = Vec::new();
    for entry in status.dirty {
        if !wanted(&entry.path) {
            continue;
        }
        let file = subject.top.join(&entry.path);
        match std::fs::symlink_metadata(&file) {
            Ok(meta) if meta.is_file() || meta.file_type().is_symlink() => present.push(entry.path),
            Ok(_) => dirty.push((entry.path, Some("not-a-file".to_owned()))),
            Err(_) => dirty.push((entry.path, None)),
        }
    }
    let ids = gate_hash_paths(
        git,
        &subject.git_dir,
        &subject.top,
        &present,
        budgets()?,
        &CANCELLED,
    )?;
    dirty.extend(present.into_iter().zip(ids.into_iter().map(Some)));
    dirty.sort();
    Ok(Snapshot {
        commit,
        tree,
        dirty,
    })
}

impl Snapshot {
    /// The scope digest and counts for one gate's patterns.
    pub fn scoped(&self, patterns: &[&str]) -> Scoped {
        let mut paths: BTreeMap<&str, &str> = self
            .tree
            .iter()
            .filter(|(p, _)| in_scope(patterns, p))
            .map(|(p, id)| (p.as_str(), id.as_str()))
            .collect();
        let mut dirty = Sha256::new();
        let mut count = 0;
        for (path, id) in self.dirty.iter().filter(|(p, _)| in_scope(patterns, p)) {
            count += 1;
            dirty.update(format!("{path}\0{}\n", id.as_deref().unwrap_or("deleted")).as_bytes());
            match id {
                Some(id) => paths.insert(path, id),
                None => paths.remove(path.as_str()),
            };
        }
        let mut all = Sha256::new();
        all.update(b"writfold-gate-scope/1\n");
        for (path, id) in &paths {
            all.update(format!("{path}\0{id}\n").as_bytes());
        }
        let hex = |d: Sha256| format!("sha256:{}", crate::index::hex(&d.finalize()));
        Scoped {
            digest: hex(all),
            paths: paths.len(),
            dirty: count,
            dirty_digest: (count > 0).then(|| hex(dirty)),
        }
    }

    /// The in-scope paths that differ from `commit` (committed changes,
    /// then dirty files), or `None` when that commit is not in this clone.
    pub fn changed_since(
        &self,
        git: &GitExecutable<'_>,
        subject: &Subject,
        commit: &str,
        patterns: &[&str],
    ) -> Result<Option<Vec<String>>> {
        let reads = subject.reads();
        let head = self.commit.as_str();
        let Some(mut changed) =
            kb_diff_tree(git, reads, commit, head, ".", budgets()?, &CANCELLED)?
        else {
            return Ok(None);
        };
        changed.retain(|p| in_scope(patterns, p));
        for (path, _) in self.dirty.iter().filter(|(p, _)| in_scope(patterns, p)) {
            if !changed.contains(path) {
                changed.push(path.clone());
            }
        }
        Ok(Some(changed))
    }
}
