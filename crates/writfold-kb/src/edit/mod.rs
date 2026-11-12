//! The Splicery adapter (`writfold kb edit`). Splicery stays frozen as a
//! language; this adapter replays recipes through the pinned kernel
//! directly in a KB run's recorded worktree and keeps the recipe, the
//! splice sites and the preview as a receipt in the run. The result is an
//! ordinary uncommitted Git change, judged afterwards by gates (G1): an
//! edit inside a gate's scope makes its receipts stale like any other
//! change. The edit trial's signing, approval and own-worktree flow is not
//! used. Nothing here stops other tools from editing the worktree; the
//! claims are a pinned replay, exact preconditions, and per-file atomic
//! writes with a journal that makes a crash between files recoverable.
mod apply;
pub mod diff;
mod journal;
pub mod plan;
pub mod receipt;
pub mod target;

use crate::emit::{Answer, Frame, Output, Row};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::{Request, runs, time};
use journal::{Entry, Journal};
use plan::{FileEdit, Recipe};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// One `edit` verb.
#[derive(Clone, Debug)]
pub enum EditVerb {
    /// `edit apply RECIPE.spl… [--run ID] [--worktree DIR] [--dry-run]`.
    Apply {
        recipes: Vec<PathBuf>,
        run: Option<String>,
        worktree: Option<PathBuf>,
        dry_run: bool,
    },
    /// `edit recover [--run ID] [--abandon]`.
    Recover { run: Option<String>, abandon: bool },
}

fn frame(request: &Request, verb: &str, args: Vec<String>) -> Frame {
    Frame {
        verb: verb.to_owned(),
        args,
        budget: request.budget,
        cursor: request.cursor.clone(),
        json: request.json,
    }
}

/// What every edit verb needs: the KB, the request, Git, the local state
/// directory and the clock.
struct Ctx<'a> {
    ws: &'a Workspace,
    request: &'a Request,
    tool: GitTool,
    state: PathBuf,
    now: i64,
}

/// Answer one edit verb.
pub fn execute(ws: &Workspace, request: &Request, verb: &EditVerb, now: i64) -> Result<Output> {
    let Some(repo) = &ws.repo else {
        return Err(KbError::Refused(
            "edits need a KB under Git (receipts are tracked files)".into(),
        ));
    };
    let ctx = Ctx {
        ws,
        request,
        tool: GitTool::locate()?,
        state: repo.git_dir.join("writfold"),
        now,
    };
    match verb {
        EditVerb::Apply {
            recipes,
            run,
            worktree,
            dry_run,
        } => apply_verb(&ctx, recipes, run.as_deref(), worktree.as_deref(), *dry_run),
        EditVerb::Recover { run, abandon } => recover_verb(&ctx, run.as_deref(), *abandon),
    }
}

fn apply_verb(
    ctx: &Ctx<'_>,
    recipes: &[PathBuf],
    run: Option<&str>,
    worktree: Option<&std::path::Path>,
    dry_run: bool,
) -> Result<Output> {
    let (recipes, paths) = plan::recipes(recipes, &ctx.request.cwd)?;
    let session = Session::open(ctx.ws, Some(&ctx.tool), false, ctx.now)?;
    let rec = crate::write::run::target(&session, run)?;
    if runs::state(&session, &rec) == "closed" {
        return Err(KbError::Refused(format!(
            "run {} is closed; reopen it with status {} --set open",
            rec.id, rec.id
        )));
    }
    if Journal::load(&ctx.state, &rec.id)?.is_some() {
        return Err(apply::pending(&rec.id));
    }
    let found = target::resolve(&session, &rec, worktree, &ctx.request.cwd)?;
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("edits need Git".into()))?;
    let root = &ctx.ws.root;
    let files = plan::replay_all(git, &found.top, &found.git_dir, root, &recipes, &paths)?;
    let args = recipes.iter().map(|r| r.sha256.clone()).collect();
    if dry_run {
        let answer = answer(&files, &recipes, "preview", None);
        let mut out = session.emit(&answer, &frame(ctx.request, "edit-preview", args))?;
        let diff: String = files.iter().map(|f| f.diff.as_str()).collect();
        out.text = format!("{diff}{}", out.text);
        return Ok(out);
    }
    let (journal, lines) = journal_for(&rec, &found, &recipes, &files, ctx.now);
    drop(session);
    let befores: Vec<Vec<u8>> = files.iter().map(|f| f.before.clone()).collect();
    apply::apply(root, &ctx.state, &journal, &befores, ctx.request.probe)?;
    let receipt = Some((&journal.receipt, &lines));
    let mut answer = answer(&files, &recipes, "applied", receipt);
    notes(&mut answer, &journal, found.record.is_some());
    let session = Session::open(ctx.ws, Some(&ctx.tool), false, ctx.now)?;
    session.emit(&answer, &frame(ctx.request, "edit-apply", args))
}

fn recover_verb(ctx: &Ctx<'_>, run: Option<&str>, abandon: bool) -> Result<Output> {
    let session = Session::open(ctx.ws, Some(&ctx.tool), false, ctx.now)?;
    let rec = crate::write::run::target(&session, run)?;
    drop(session);
    let (journal, found) = apply::recover(
        &ctx.ws.root,
        &ctx.state,
        &rec.id,
        abandon,
        ctx.request.probe,
    )?;
    let mut answer = Answer::default();
    for r in found {
        answer.rows.push(Row {
            kind: "edit",
            id: r.path,
            status: Some(r.state.into()),
            ..Row::default()
        });
    }
    answer.notes.push(match abandon {
        true => "journal dropped; no receipt was written and no file was changed".into(),
        false => format!("recorded {}", journal.receipt),
    });
    let session = Session::open(ctx.ws, Some(&ctx.tool), false, ctx.now)?;
    session.emit(&answer, &frame(ctx.request, "edit-recover", vec![rec.id]))
}

/// The journal of an apply: the files, the receipt and a recorded
/// `--worktree`; with the receipt line of each file.
fn journal_for(
    rec: &crate::index::query::Rec,
    found: &target::Target,
    recipes: &[Recipe],
    files: &[FileEdit],
    now: i64,
) -> (Journal, Vec<u32>) {
    let stamp = time::stamp(now);
    let compact: String = stamp.chars().filter(|c| *c != '-' && *c != ':').collect();
    let mut salt = Sha256::new();
    salt.update(format!("{stamp}\0{}\0{}", std::process::id(), rec.id).as_bytes());
    for r in recipes {
        salt.update(r.sha256.as_bytes());
    }
    salt.update(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
            .to_le_bytes(),
    );
    let name = format!("{compact}-{}", &crate::index::hex(&salt.finalize())[..6]);
    let receipt = format!("{}/{}/{name}.md", runs::dir(rec), receipt::DIR);
    let worktree = found.top.to_string_lossy();
    let facts = receipt::Facts {
        run: &rec.id,
        repo: &found.repo,
        worktree: &worktree,
        base: found.base.as_deref(),
        head: found.head.as_deref(),
        recorded: &stamp,
    };
    let (lines, bytes) = receipt::bytes(&facts, recipes, files);
    let mut kb = vec![crate::write::Op::Create {
        path: receipt.clone(),
        bytes: bytes.into_bytes(),
    }];
    kb.extend(found.record.clone());
    let entries = files
        .iter()
        .map(|f| Entry {
            path: f.path.clone(),
            before: f.before_sha.clone(),
            after: f.after_sha.clone(),
            temp: apply::temp_name(),
            bytes: f.after.clone(),
        })
        .collect();
    let journal = Journal {
        run: rec.id.clone(),
        worktree: found.top.clone(),
        receipt,
        files: entries,
        kb,
    };
    (journal, lines)
}

/// One row per file: the recipe, Splicery's splice summary and the blobs;
/// one `site:` line per splice (the first eight).
fn answer(
    files: &[FileEdit],
    recipes: &[Recipe],
    status: &str,
    receipt: Option<(&String, &Vec<u32>)>,
) -> Answer {
    let short = |t: &str| t.get(..12).unwrap_or(t).to_owned();
    let mut answer = Answer::default();
    for (index, f) in files.iter().enumerate() {
        let recipe = recipes.get(f.recipe).map_or("?", |r| r.name.as_str());
        let mut row = Row {
            kind: "edit",
            id: f.path.clone(),
            status: Some(status.to_owned()),
            text: format!(
                "recipe={recipe} splices={} -{} +{} bytes before={} after={}",
                f.sites.len(),
                f.removed(),
                f.inserted(),
                short(&f.before_blob),
                short(&f.after_blob)
            ),
            path: receipt.map(|(p, _)| p.clone()),
            line: receipt.and_then(|(_, l)| l.get(index).copied()),
            ..Row::default()
        };
        for s in f.sites.iter().take(8) {
            row.more.push(crate::emit::More {
                label: "site",
                text: receipt::site(s),
                ..crate::emit::More::default()
            });
        }
        if f.sites.len() > 8 {
            row.more.push(crate::emit::More {
                label: "site",
                text: format!("{} more sites (all are in the receipt)", f.sites.len() - 8),
                ..crate::emit::More::default()
            });
        }
        answer.rows.push(row);
    }
    if status == "preview" {
        answer
            .notes
            .push("dry run: nothing was written; the diff above is the preview".into());
    }
    answer
}

fn notes(answer: &mut Answer, journal: &Journal, recorded: bool) {
    answer.notes.push(format!("recorded {}", journal.receipt));
    if recorded {
        answer.notes.push(format!(
            "recorded the worktree {} in the run",
            journal.worktree.display()
        ));
    }
    answer.notes.push(format!(
        "applied in {}: an ordinary uncommitted change; review it, run its gates (writfold kb gate status), commit it",
        journal.worktree.display()
    ));
    answer
        .notes
        .push("the receipt is written to the KB work tree; commit it to share it".into());
}
