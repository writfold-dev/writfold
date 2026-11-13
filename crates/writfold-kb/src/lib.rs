//! Writfold KB v1: Git-native project continuity.
//!
//! The KB is plain records in Git (the prototype's layout, read unchanged).
//! A local SQLite file under the repository's Git directory is a derived
//! index, rebuilt in full whenever the observed snapshot (HEAD plus the
//! digests of dirty KB files) differs from the one it was built at. Every
//! verb returns budgeted rows, each ending in its source position, and a
//! trailer stating coverage, the KB pin and a resume cursor. Unreadable
//! files are counted and listed, never dropped. The write verbs (`write`)
//! change tracked files only through the atomic protocol and leave the
//! work tree dirty for the user to commit.
#![forbid(unsafe_code)]

pub mod check;
pub mod deliver;
pub mod draft;
pub mod edit;
pub mod emit;
pub mod error;
pub mod gate;
pub mod guard;
pub mod hook;
pub mod index;
pub mod layout;
pub mod model;
pub mod parse;
pub mod plan;
pub mod proposal;
pub mod runs;
pub mod session;
pub mod shell;
pub mod stale;
pub mod time;
pub mod verbs;
pub mod workspace;
pub mod write;

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
    Find {
        text: String,
        filter: verbs::find::Filter,
        signatures: bool,
    },
    Status {
        target: String,
    },
    Impact {
        target: String,
        depth: usize,
        direction: verbs::impact::Direction,
        inferred: bool,
    },
    Uses {
        target: String,
    },
    Timeline {
        target: String,
    },
    IndexStatus,
    IndexRebuild,
    IndexVerify,
    Runs {
        open: bool,
        project: Option<String>,
    },
    /// `repos [--missing]`: the cited repositories and their local clones.
    Repos {
        missing: bool,
    },
    Resume {
        run: Option<String>,
    },
    /// A write verb (`checkpoint`, `record`, `status --set`, `cite make`).
    Write(write::Write),
    /// `gate run|status|verify|lines`.
    Gate(gate::GateVerb),
    /// `edit apply|recover` (the Splicery adapter).
    Edit(edit::EditVerb),
    /// `deliver prepare|check|attest|waive|status` (G2; nothing is sent).
    Deliver(deliver::DeliverVerb),
    /// `plan set|use|status|advance|skip` (the run graph).
    Plan(plan::PlanVerb),
    /// `propose …` and `proposals list|show|accept|drop` (the local queue).
    Proposal(proposal::ProposalVerb),
    /// `check [<project>|<path>]`: the existing checks composed.
    Check {
        scope: Option<String>,
    },
    /// `checker list` and `cite make --kind <kind> key=value …`.
    Checker(stale::checker::verb::CheckerVerb),
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
    /// Called after each step of the write protocol; fault tests stop the
    /// process there. Production passes [`write::no_probe`].
    pub probe: write::Probe,
}

/// Locate the KB, open its index at the current snapshot, answer, render.
pub fn run(request: &Request) -> Result<Output> {
    let ws = discover(request.kb.as_deref(), &request.cwd)?;
    let now = request.now.unwrap_or_else(time::now);
    if let Verb::Write(write) = &request.verb {
        return write::execute(&ws, request, write, now);
    }
    if let Verb::Gate(verb) = &request.verb {
        return gate::execute(&ws, request, verb, now);
    }
    if let Verb::Edit(verb) = &request.verb {
        return edit::execute(&ws, request, verb, now);
    }
    if let Verb::Deliver(verb) = &request.verb {
        return deliver::execute(&ws, request, verb, now);
    }
    if let Verb::Plan(verb) = &request.verb {
        return plan::execute(&ws, request, verb, now);
    }
    if let Verb::Proposal(verb) = &request.verb {
        return proposal::execute(&ws, request, verb, now);
    }
    if let Verb::Checker(verb) = &request.verb {
        return stale::checker::verb::execute(&ws, request, verb, now);
    }
    if let Verb::Check { scope } = &request.verb {
        return check::execute(&ws, request, scope.as_deref(), now);
    }
    let tool = match &ws.repo {
        Some(_) => Some(GitTool::locate()?),
        None => None,
    };
    let rebuild = matches!(request.verb, Verb::IndexRebuild);
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
        Verb::Resume { run } => {
            let (answer, id) = verbs::runs::resume(session, run.as_deref(), &request.cwd)?;
            (answer, "resume", vec![id])
        }
        other => graph(session, other)?,
    })
}

/// The search, graph and housekeeping verbs.
fn graph(session: &Session<'_>, verb: &Verb) -> Result<(Answer, &'static str, Vec<String>)> {
    Ok(match verb {
        Verb::Find {
            text,
            filter,
            signatures,
        } => {
            let mut args = vec![text.clone(), format!("signatures={signatures}")];
            args.extend(filter.kinds.iter().map(|k| format!("kind={k}")));
            args.extend(filter.project.iter().map(|p| format!("project={p}")));
            (
                verbs::find::find(session, text, filter, *signatures)?,
                "find",
                args,
            )
        }
        Verb::Status { target } => (
            verbs::status::status(session, target)?,
            "status",
            vec![target.clone()],
        ),
        Verb::Impact {
            target,
            depth,
            direction,
            inferred,
        } => (
            verbs::impact::impact(session, target, *depth, *direction, *inferred)?,
            "impact",
            vec![
                target.clone(),
                format!("depth={depth}"),
                format!("direction={}", direction.name()),
                format!("inferred={inferred}"),
            ],
        ),
        Verb::Uses { target } => (
            verbs::uses::uses(session, target)?,
            "uses",
            vec![target.clone()],
        ),
        Verb::Timeline { target } => (
            verbs::timeline::timeline(session, target)?,
            "timeline",
            vec![target.clone()],
        ),
        Verb::IndexVerify => (verbs::index::verify(session)?, "index-verify", Vec::new()),
        Verb::Runs { open, project } => {
            let mut args = vec![format!("open={open}")];
            args.extend(project.iter().map(|p| format!("project={p}")));
            (
                verbs::runs::list(session, *open, project.as_deref())?,
                "runs",
                args,
            )
        }
        Verb::Repos { missing } => (
            verbs::repos::repos(session, *missing)?,
            "repos",
            vec![format!("missing={missing}")],
        ),
        Verb::IndexRebuild => (verbs::index::status(session)?, "index-rebuild", Vec::new()),
        _ => (verbs::index::status(session)?, "index-status", Vec::new()),
    })
}
