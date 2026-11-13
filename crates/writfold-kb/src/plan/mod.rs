//! The run graph (`writfold kb plan set|use|status|advance|skip`).
//!
//! Writfold records the graph and the agent walks it; nothing here
//! schedules anything. A plan is a template's steps spliced at the run's
//! root ([`template`]); the run keeps the template's bytes as a snapshot
//! and an append-only log of events ([`event`]); the steps, the current
//! step and what it owes are derived from that log on every read
//! ([`fold`], [`judge`]), never stored. The decisions themselves (which
//! step is current, whether it may advance, whether a done step is stale)
//! are the pure kernel [`writfold_core::plan`]. Gate checks over the run's
//! own records live in [`records`].
//!
//! `plan use` splices a template before the current step, as its
//! children ([`splice`]); `use=` includes are expanded when a template is
//! spliced; required start and end templates are spliced into every new
//! plan ([`required`]); a spliced template whose file has since changed
//! is shown `!template-moved` ([`moved`]).
pub mod disorder;
pub mod event;
pub mod fold;
pub mod judge;
pub mod moved;
mod place;
pub mod recheck;
pub mod records;
pub mod required;
mod show;
mod splice;
pub mod template;
mod write;

use crate::emit::{Answer, Output, Row};
use crate::error::{KbError, Result};
use crate::gate::status::Owed;
use crate::gate::subject::Subject;
use crate::index::query::Rec;
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::{Request, runs};
use fold::Graph;
use records::Records;
use std::path::{Path, PathBuf};

/// One `plan` verb.
#[derive(Clone, Debug)]
pub enum PlanVerb {
    /// Splice a template at the run's root: by name from
    /// `.writfold/plans/`, or from a file.
    Set {
        template: Option<String>,
        from: Option<PathBuf>,
        run: Option<String>,
        repo: Option<PathBuf>,
    },
    /// Splice a template before the current step, as its children (at
    /// the root when the run has no plan); `again` splices another copy.
    Use {
        template: String,
        again: bool,
        run: Option<String>,
        repo: Option<PathBuf>,
    },
    Status {
        run: Option<String>,
        repo: Option<PathBuf>,
    },
    Advance {
        run: Option<String>,
        repo: Option<PathBuf>,
    },
    Skip {
        reason: String,
        by: String,
        run: Option<String>,
        repo: Option<PathBuf>,
    },
}

/// Answer one plan verb.
pub fn execute(ws: &Workspace, request: &Request, verb: &PlanVerb, now: i64) -> Result<Output> {
    if ws.repo.is_none() {
        return Err(KbError::Refused(
            "plans need a KB under Git (their events are tracked files)".into(),
        ));
    }
    let tool = GitTool::locate()?;
    match verb {
        PlanVerb::Status { run, repo } => {
            let session = Session::open(ws, Some(&tool), false, now)?;
            let loaded = Loaded::open(&session, run.as_deref(), repo.as_deref(), &request.cwd)?;
            let answer = status(&session, &loaded)?;
            let frame = crate::gate::frame(request, "plan-status", vec![loaded.rec.id.clone()]);
            session.emit(&answer, &frame)
        }
        PlanVerb::Set { .. } => splice::set(ws, &tool, request, verb, now),
        PlanVerb::Use { .. } => splice::use_template(ws, &tool, request, verb, now),
        PlanVerb::Advance { run, repo } => {
            write::advance(ws, &tool, request, (run.as_deref(), repo.as_deref()), now)
        }
        PlanVerb::Skip { .. } => write::skip(ws, &tool, request, verb, now),
    }
}

/// A run with its folded plan, its records and its subject repository.
pub(crate) struct Loaded {
    pub rec: Rec,
    pub graph: Graph,
    pub records: Records,
    pub subject: Option<Subject>,
    /// Spliced templates whose file has changed since.
    pub moved: Vec<moved::Moved>,
}

impl Loaded {
    pub fn open(
        session: &Session<'_>,
        run: Option<&str>,
        repo: Option<&Path>,
        cwd: &Path,
    ) -> Result<Self> {
        let rec = crate::gate::target(session, run)?;
        let graph = fold::read(&session.ws.root, &runs::dir(&rec), &rec.id);
        let records = records::observe(session, &rec)?;
        let subject = crate::gate::locate(session, &rec, repo, cwd)?;
        let top = subject.as_ref().map(|s| (s.top.as_path(), s.name.as_str()));
        let moved = moved::find(&session.ws.root, top, &graph);
        Ok(Self {
            rec,
            graph,
            records,
            subject,
            moved,
        })
    }

    /// Every judged step's gates; `None` without a subject repository.
    pub fn judged(&self, session: &Session<'_>) -> Result<Option<Vec<Vec<judge::Gate>>>> {
        let Some(subject) = &self.subject else {
            return Ok(None);
        };
        let run_dir = runs::dir(&self.rec);
        judge::steps(
            session,
            &run_dir,
            &self.rec.id,
            &self.graph,
            subject,
            &self.records,
        )
        .map(Some)
    }
}

/// The `plan status` answer.
fn status(session: &Session<'_>, loaded: &Loaded) -> Result<Answer> {
    let mut answer = Answer::default();
    let graph = &loaded.graph;
    if !graph.planned() {
        answer.unknown = Some("no-plan");
        answer.notes.push(format!(
            "run {} has no plan: writfold kb plan set <template> (templates in {}/)",
            loaded.rec.id,
            template::DIR
        ));
    }
    let judged = loaded.judged(session)?;
    if judged.is_none() && graph.planned() {
        answer
            .notes
            .push("no subject repository here: gates are not judged; pass --repo DIR".into());
    }
    answer.rows = show::rows(graph, judged.as_deref(), &loaded.moved);
    let disorder = disorder::find(graph);
    disorder.mark(&mut answer.rows);
    let count = |status: &str| {
        answer
            .rows
            .iter()
            .filter(|r| r.status.as_deref() == Some(status))
            .count()
    };
    let stale = answer
        .rows
        .iter()
        .filter(|r| r.flags.iter().any(|f| f == "stale"))
        .count();
    let flagged = |flag: &str| {
        answer
            .rows
            .iter()
            .filter(|r| r.flags.iter().any(|f| f == flag))
            .count()
    };
    let moved = flagged("template-moved");
    answer.counts = vec![
        ("steps", graph.steps.len()),
        ("done", count("done")),
        ("skipped", count("skipped")),
        ("stale", stale),
        ("moved", moved),
        ("invalid", graph.invalid.len()),
    ];
    if disorder.count() > 0 {
        answer.counts.push(("disorder", disorder.count()));
    }
    let spliced: Vec<String> = graph
        .splices
        .iter()
        .map(|(_, s, t)| {
            let under = match s.parent.as_str() {
                "" => String::new(),
                parent => format!(" under {parent}"),
            };
            let short = moved::short(&s.digest);
            format!("{} @{short}{under} from {}", t.name, s.source)
        })
        .collect();
    if !spliced.is_empty() {
        answer.notes.push(format!(
            "plan {}; steps are derived from runs/{}/plan/ on every read, never stored",
            spliced.join(", "),
            loaded.rec.id
        ));
    }
    answer
        .notes
        .extend(loaded.moved.iter().map(moved::Moved::note));
    answer.notes.extend(disorder.notes);
    Ok(answer)
}

/// What a run owes for `gate status`: the plan's gates (every defined gate
/// without a plan) and the current step's run-record digest.
pub fn owed(session: &Session<'_>, rec: &Rec) -> Owed {
    let graph = fold::read(&session.ws.root, &runs::dir(rec), &rec.id);
    let records = records::observe(session, rec).ok();
    Owed {
        gates: graph.planned().then(|| graph.gates()),
        records: records.map(|r| {
            let (begin, end) = window(&graph, &r);
            records::digest(begin.as_ref(), &end, &r)
        }),
    }
}

/// The window run-record gates are judged over now: the current step's
/// (from where it began to now), the last step's once every step is
/// closed, the whole run without a plan.
pub fn window(graph: &Graph, records: &Records) -> (Option<event::Baseline>, event::Baseline) {
    let now = records.baseline();
    match graph.current().or(graph.steps.len().checked_sub(1)) {
        Some(i) => {
            let step = &graph.steps[i];
            (Some(step.begin.clone()), step.end().cloned().unwrap_or(now))
        }
        None => (None, now),
    }
}

/// The one plan line `resume` (and so `writfold watch`) shows; `None`
/// when the run has no plan.
pub fn resume_row(session: &Session<'_>, rec: &Rec, cwd: &Path) -> Option<Row> {
    let loaded = Loaded::open(session, Some(&rec.id), None, cwd).ok()?;
    if !loaded.graph.planned() {
        return None;
    }
    let judged = loaded.judged(session).ok().flatten();
    let mut row = show::resume_row(&loaded.graph, judged.as_deref(), &loaded.moved);
    let kinds = disorder::find(&loaded.graph).kinds();
    row.flags.extend(kinds.into_iter().map(str::to_owned));
    Some(row)
}
