//! `writfold kb check [<project>|<path>]`: one command over the checks that
//! already exist. It runs `stale` (citations, for the scope given),
//! `index verify`, `gate status` for the current run when there is one,
//! and the plan's status when a plan exists ([`plan`]), and prints one
//! `check` row per area carrying that verb's own trailer. The exit code is
//! the worst of theirs (1 error, then 3 refused, then 2, then 0). Nothing
//! is written beyond what `index verify` already writes (the rebuilt
//! comparison index).
use crate::emit::{Answer, Frame, More, Output, Row};
use crate::error::Result;
use crate::gate::GateVerb;
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use crate::{Request, Verb, runs};

/// One composed check: its area, the command a person would run, and the
/// verb's output (or error) when it ran.
pub struct Part {
    pub area: &'static str,
    pub command: String,
    /// `None`: skipped, with the reason.
    pub outcome: Option<(String, u8)>,
    pub skipped: Option<String>,
    /// Extra flags for its row (the plan's merge disorder).
    pub flags: Vec<String>,
}

/// Run every part, then answer at the KB's snapshot.
pub fn execute(ws: &Workspace, request: &Request, scope: Option<&str>, now: i64) -> Result<Output> {
    let parts = parts(ws, request, scope);
    let tool = match &ws.repo {
        Some(_) => Some(GitTool::locate()?),
        None => None,
    };
    let session = Session::open(ws, tool.as_ref(), false, now)?;
    let worst = parts
        .iter()
        .filter_map(|p| p.outcome.as_ref().map(|(_, code)| *code))
        .max_by_key(|code| severity(*code))
        .unwrap_or(0);
    let ran = parts.iter().filter(|p| p.outcome.is_some()).count();
    let not_clean = parts
        .iter()
        .filter(|p| p.outcome.as_ref().is_some_and(|(_, c)| *c != 0))
        .count();
    let answer = Answer {
        counts: vec![("checks", ran), ("not_clean", not_clean)],
        rows: parts.iter().map(row).collect(),
        ..Answer::default()
    };
    let frame = Frame {
        verb: "check".into(),
        args: scope.map(str::to_owned).into_iter().collect(),
        budget: request.budget,
        cursor: request.cursor.clone(),
        json: request.json,
    };
    let mut output = session.emit(&answer, &frame)?;
    if severity(worst) > severity(output.exit) {
        output.exit = worst;
    }
    Ok(output)
}

/// `stale`, `index verify`, the current run's `gate status`, the plan.
fn parts(ws: &Workspace, request: &Request, scope: Option<&str>) -> Vec<Part> {
    let stale = Verb::Stale {
        scope: scope.map(str::to_owned),
        offline: false,
    };
    let mut parts = vec![
        sub(request, "stale", stale_command(scope), stale),
        sub(
            request,
            "index",
            "writfold kb index verify".into(),
            Verb::IndexVerify,
        ),
    ];
    let current = crate::proposal::state_of(ws).and_then(|state| runs::current(&state));
    parts.push(match current {
        Some(run) => sub(
            request,
            "gates",
            format!("writfold kb gate status --run {run}"),
            Verb::Gate(GateVerb::Status {
                run: Some(run),
                repo: None,
            }),
        ),
        None => Part {
            area: "gates",
            command: "writfold kb gate status".into(),
            outcome: None,
            skipped: Some("no current run".into()),
            flags: Vec::new(),
        },
    });
    parts.extend(plan(ws, request));
    parts
}

/// The plan's part: `plan status` of the current run when that run holds
/// a plan (a `plan/` directory of events), its row flagged with any merge
/// disorder ([`crate::plan::disorder`]); `None` otherwise.
pub fn plan(ws: &Workspace, request: &Request) -> Option<Part> {
    let run = crate::proposal::state_of(ws).and_then(|state| runs::current(&state))?;
    let events = ws
        .root
        .join("runs")
        .join(&run)
        .join(crate::plan::event::DIR);
    if !events.is_dir() {
        return None;
    }
    let verb = Verb::Plan(crate::plan::PlanVerb::Status {
        run: Some(run.clone()),
        repo: None,
    });
    let mut part = sub(
        request,
        "plan",
        format!("writfold kb plan status --run {run}"),
        verb,
    );
    let graph = crate::plan::fold::read(&ws.root, &format!("runs/{run}"), &run);
    let kinds = crate::plan::disorder::find(&graph).kinds();
    part.flags = kinds.into_iter().map(str::to_owned).collect();
    Some(part)
}

/// 0 best; then 2 (stale, partial, owed), 3 (refused), 1 (error) worst.
pub fn severity(code: u8) -> u8 {
    match code {
        0 => 0,
        2 => 1,
        3 => 2,
        _ => 3,
    }
}

fn stale_command(scope: Option<&str>) -> String {
    match scope {
        Some(scope) => format!("writfold kb stale {scope}"),
        None => "writfold kb stale".into(),
    }
}

/// Run one verb as its own invocation (plain text, default budget).
pub fn sub(request: &Request, area: &'static str, command: String, verb: Verb) -> Part {
    let inner = Request {
        verb,
        cursor: None,
        json: false,
        budget: crate::DEFAULT_BUDGET,
        ..request.clone()
    };
    let outcome = match crate::run(&inner) {
        Ok(output) => {
            let trailer = output.text.lines().last().unwrap_or_default().to_owned();
            (trailer, output.exit)
        }
        Err(error) => (format!("{error}"), error.exit_code()),
    };
    Part {
        area,
        command,
        outcome: Some(outcome),
        skipped: None,
        flags: Vec::new(),
    }
}

fn row(part: &Part) -> Row {
    let (status, text, flags) = match (&part.outcome, &part.skipped) {
        (Some((text, code)), _) => {
            let status = match code {
                0 => "clean",
                2 => "attention",
                3 => "refused",
                _ => "error",
            };
            let flags = if *code == 0 {
                Vec::new()
            } else {
                vec![status.to_owned()]
            };
            (status, text.clone(), flags)
        }
        (None, why) => ("skipped", why.clone().unwrap_or_default(), Vec::new()),
    };
    let mut row = Row {
        kind: "check",
        id: part.area.to_owned(),
        status: Some(status.into()),
        text,
        flags,
        ..Row::default()
    };
    row.flags.extend(part.flags.iter().cloned());
    if status != "clean" {
        row.more.push(More {
            label: "next",
            text: part.command.clone(),
            ..More::default()
        });
    }
    row
}
