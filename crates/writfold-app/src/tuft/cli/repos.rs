use super::Context;
use super::args::Args;
use super::program::{Program, diagnostics, full_commit};
use crate::trial::Result;
use std::collections::BTreeSet;
use std::path::PathBuf;
use writfold_kb::stale::repos;
use writfold_kb::workspace::{CANCELLED, GitTool, budgets};
use writfold_runtime::git::{KbRepository, kb_commit};

pub(super) struct Prepared {
    pub repo: PathBuf,
    pub name: String,
    pub requested: String,
    pub source: Vec<u8>,
    pub echo: bool,
}

pub(super) fn prepare(args: &Args, program: &Program, context: &Context) -> Result<Vec<Prepared>> {
    let workspace = args.workspace(context)?;
    let selectors: Vec<&str> = args
        .options
        .iter()
        .filter(|(k, _)| k == "repo")
        .map(|(_, v)| v.as_str())
        .collect();
    let selectors = if selectors.is_empty() {
        vec!["."]
    } else {
        selectors
    };
    if selectors.len() > 1 && args.option("cursor").is_some() {
        return Err(
            "--cursor resumes one repository; use --repo NAME with that repository's cursor".into(),
        );
    }
    let tool = GitTool::locate()?;
    let git = tool.admit()?;
    let mut seen = BTreeSet::new();
    let mut prepared = Vec::new();
    for selector in selectors {
        let direct = repos::locate(workspace.as_ref(), selector, &context.cwd);
        let (name, requested) = if direct.is_some() {
            (selector, program.plan.find.commit.as_str())
        } else {
            selector
                .rsplit_once('@')
                .unwrap_or((selector, &program.plan.find.commit))
        };
        if program.strict && !full_commit(requested) {
            return Err(format!("saved programs and --pinned require a full commit ID for {name}; pin it with git rev-parse '{}^{{commit}}'", requested.replace('\'', "'\\''")).into());
        }
        let repo = direct.or_else(|| repos::locate(workspace.as_ref(), name, &context.cwd))
            .ok_or_else(|| format!("no local repository {name:?}; map it in <KB git dir>/writfold/repos or pass --repo DIR"))?;
        if !seen.insert(repo.clone()) {
            return Err(format!("repository {} selected twice", repo.display()).into());
        }
        let git_dir = repos::git_dir(&repo).ok_or("repository has no Git directory")?;
        let reads = KbRepository {
            git_dir: &git_dir,
            work_tree: Some(&repo),
        };
        let commit = kb_commit(&git, reads, requested, budgets()?, &CANCELLED)?
            .ok_or_else(|| format!("at {requested:?} names no local commit in {name}; check the ref with git rev-parse --verify"))?;
        if full_commit(requested) && commit.as_str() != requested {
            return Err(
                format!("at {requested} must name the commit itself, not a tag object").into(),
            );
        }
        let source = diagnostics(
            tuft::pin(&program.source, commit.as_str()),
            &program.source,
            &program.name,
        )?;
        let mapped = repos::mapped_name(workspace.as_ref(), &repo);
        let name = mapped.unwrap_or_else(|| {
            repo.file_name()
                .map_or_else(|| name.to_owned(), |n| n.to_string_lossy().into_owned())
        });
        prepared.push(Prepared {
            repo,
            name,
            requested: requested.to_owned(),
            source,
            echo: !full_commit(requested),
        });
    }
    prepared.sort_by(|a, b| a.name.cmp(&b.name).then(a.repo.cmp(&b.repo)));
    Ok(prepared)
}
