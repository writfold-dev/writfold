//! Writfold KB v1: Git-native project continuity.
//!
//! The KB is plain records in Git (the prototype's layout, read unchanged).
//! A local SQLite file under the repository's Git directory is a derived
//! index, rebuilt in full whenever the observed snapshot (HEAD plus the
//! digests of dirty KB files) differs from the one it was built at. Every
//! verb returns budgeted rows, each ending in its source position, and a
//! trailer stating coverage, the KB pin and a resume cursor. Unreadable
//! files are counted and listed, never dropped. Nothing here writes a
//! tracked file.
#![forbid(unsafe_code)]

pub mod emit;
pub mod error;
pub mod index;
pub mod layout;
pub mod model;
pub mod parse;
pub mod session;
pub mod stale;
pub mod time;
pub mod verbs;
pub mod workspace;

use emit::{Answer, Frame, Output};
use error::Result;
use session::Session;
use std::path::PathBuf;
use verbs::brief::Home;
use workspace::{GitTool, discover};

/// Default and bounds of `--budget-tokens`.
pub const DEFAULT_BUDGET: usize = 1000;
pub const MAX_BUDGET: usize = 1_000_000;

/// One verb with its arguments.
#[derive(Clone, Debug)]
pub enum Verb {
    Brief {
        project: Option<String>,
        all: bool,
    },
    Context {
        target: String,
        depth: usize,
    },
    Why {
        text: String,
        sections: Vec<String>,
    },
    Stale {
        scope: Option<String>,
        offline: bool,
    },
    IndexStatus,
    IndexRebuild,
}

/// One invocation.
#[derive(Clone, Debug)]
pub struct Request {
    pub verb: Verb,
    pub kb: Option<PathBuf>,
    pub cwd: PathBuf,
    pub budget: usize,
    pub cursor: Option<String>,
    pub json: bool,
    /// Seconds since the epoch; `None` reads the clock.
    pub now: Option<i64>,
}

/// Locate the KB, open its index at the current snapshot, answer, render.
pub fn run(request: &Request) -> Result<Output> {
    let ws = discover(request.kb.as_deref(), &request.cwd)?;
    let tool = match &ws.repo {
        Some(_) => Some(GitTool::locate()?),
        None => None,
    };
    let rebuild = matches!(request.verb, Verb::IndexRebuild);
    let now = request.now.unwrap_or_else(time::now);
    let session = Session::open(&ws, tool.as_ref(), rebuild, now)?;
    let (answer, name, args) = answer(&session, request)?;
    let frame = Frame {
        verb: name.to_owned(),
        args,
        budget: request.budget,
        cursor: request.cursor.clone(),
        json: request.json,
    };
    session.emit(&answer, &frame)
}

fn answer(session: &Session<'_>, request: &Request) -> Result<(Answer, &'static str, Vec<String>)> {
    Ok(match &request.verb {
        Verb::Brief { project, all } => {
            let (home, inferred) = match (project, all) {
                (_, true) => (Home::All, true),
                (Some(name), false) => (Home::Project(name.clone()), true),
                (None, false) => match verbs::brief::infer(session, &request.cwd)? {
                    Some(name) => (Home::Project(name), true),
                    None => (Home::All, false),
                },
            };
            let mut answer = verbs::brief::brief(session, &home, request.budget)?;
            if !inferred {
                answer.unknown = Some("no-home");
                answer.notes.push("no project matched the working directory; showing a KB-wide brief (pass a project, or --all)".into());
            }
            let scope = match home {
                Home::Project(name) => name,
                Home::All => "--all".into(),
            };
            (answer, "brief", vec![scope])
        }
        Verb::Context { target, depth } => (
            verbs::context::context(session, target, *depth)?,
            "context",
            vec![target.clone(), format!("depth={depth}")],
        ),
        Verb::Why { text, sections } => (
            verbs::why::why(session, text, sections)?,
            "why",
            vec![text.clone()],
        ),
        Verb::Stale { scope, offline } => (
            verbs::stale::stale(session, scope.as_deref(), *offline)?,
            "stale",
            vec![
                scope.clone().unwrap_or_default(),
                format!("offline={offline}"),
            ],
        ),
        Verb::IndexStatus => (verbs::index::status(session)?, "index-status", Vec::new()),
        Verb::IndexRebuild => (verbs::index::status(session)?, "index-rebuild", Vec::new()),
    })
}
