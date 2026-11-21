//! Read-only rule judgments for the current run, as if every rule matched.
use super::require::{self, Owed};
use super::rules::{self, Event, Require, Rule};
use crate::Request;
use crate::emit::{Answer, Frame, More, Output, Row};
use crate::error::Result;
use crate::session::Session;
use crate::workspace::{GitTool, Workspace};
use writfold_core::guard::{Decision, Fact, Mode, decide, nudge};

/// Project the hook's facts without running a hook, gate or checker.
pub fn execute(ws: &Workspace, request: &Request, now: i64) -> Result<Output> {
    let tool = match &ws.repo {
        Some(_) => Some(GitTool::locate()?),
        None => None,
    };
    let session = Session::open(ws, tool.as_ref(), false, now)?;
    let (mut rules, problems) = rules::all(&ws.root, &request.cwd);
    rules.sort_by(|a, b| a.id.cmp(&b.id));
    let mut answer = Answer::default();
    notes(&mut answer, &session, request);
    if rules.is_empty() && problems.is_empty() {
        answer.unknown = Some("no-guards");
        answer
            .notes
            .push("no guard rule files in .writfold/guards/".into());
    }
    let count = rules.len();
    if !rules.is_empty() {
        let refs: Vec<&Rule> = rules.iter().collect();
        let evaluated = require::evaluate(ws, &request.cwd, now, &refs);
        answer.rows.extend(
            rules
                .into_iter()
                .zip(evaluated)
                .map(|(rule, owed)| row(rule, owed)),
        );
    }
    let invalid = problems.len();
    let mut report = writfold_core::diag::Report::default();
    for problem in problems {
        let id = crate::defs::stem(&problem.path).to_owned();
        answer
            .rows
            .push(crate::defs::invalid_row("guard-rule", &id, &problem, None));
        report.extend(problem.report);
    }
    answer.report = Some(report);
    answer.counts = counts(&answer.rows, count, invalid);
    let exit = if answer.counts.iter().skip(1).any(|(_, n)| *n > 0) {
        2
    } else {
        0
    };
    let frame = Frame {
        verb: "guard status".into(),
        args: vec![request.cwd.to_string_lossy().into_owned()],
        budget: request.budget,
        cursor: request.cursor.clone(),
        json: request.json,
    };
    let mut output = session.emit(&answer, &frame)?;
    // This verb's policy includes deny/ask/nudge and makes no-guards exit 0.
    output.exit = exit;
    Ok(output)
}

fn notes(answer: &mut Answer, session: &Session<'_>, request: &Request) {
    let current = crate::seat::current(session.ws, &session.ws.seat);
    answer.notes.push(format!(
        "judged for the current run {} in {} as if each rule had matched; a guard, not enforcement; nothing was run",
        current.as_ref().map_or("none", |(found, _)| &found.id),
        request.cwd.display(),
    ));
    // Said only when a narrower pointer than the clone's shared one decided.
    if let Some((found, _)) = current.filter(|(found, _)| found.level != "shared") {
        let seat = &session.ws.seat;
        let whose = match found.level {
            "session" => format!("session {}", seat.session.as_deref().unwrap_or_default()),
            _ => format!(
                "work tree {}",
                seat.tree
                    .as_deref()
                    .map_or_else(String::new, |t| t.display().to_string())
            ),
        };
        answer.notes.push(format!(
            "the current run comes from the {} pointer ({whose}), read before the clone's shared pointer",
            found.level
        ));
    }
    if session
        .ws
        .root
        .join(".writfold/deliveries/guard.toml")
        .exists()
    {
        answer.notes.push(
            "the delivery guard (.writfold/deliveries/guard.toml) is a separate mechanism and is not covered".into(),
        );
    }
}

fn counts(rows: &[Row], rules: usize, invalid: usize) -> Vec<(&'static str, usize)> {
    let flagged = |flag: &str| {
        rows.iter()
            .filter(|r| r.flags.iter().any(|f| f == flag))
            .count()
    };
    let audit = rows
        .iter()
        .filter(|r| r.flags.iter().any(|f| f == "audit"))
        .filter(|r| matches!(r.status.as_deref(), Some("would-deny" | "would-ask")))
        .count();
    let mut counts = vec![
        ("rules", rules),
        ("deny", flagged("deny")),
        ("ask", flagged("ask")),
        ("nudge", flagged("nudge")),
        ("invalid", invalid),
    ];
    // Shown only when an audit rule would refuse, so other answers keep
    // their trailer.
    if audit > 0 {
        counts.push(("audit", audit));
    }
    counts
}

fn row(rule: Rule, owed: Option<Vec<Owed>>) -> Row {
    let (status, flag) = standing(&rule, owed.as_deref());
    let text = description(&rule);
    let mut flags: Vec<String> = match rule.mode {
        Mode::Audit => vec!["audit".into()],
        Mode::Enforce => flag.into_iter().map(str::to_owned).collect(),
    };
    let mut more: Vec<More> = owed
        .iter()
        .flatten()
        .filter(|o| o.fact != Fact::Met)
        .cloned()
        .map(|o| More {
            label: "owes",
            text: o.text,
            flags: if o.fact == Fact::Unknown {
                vec!["unknown".into()]
            } else {
                Vec::new()
            },
            ..More::default()
        })
        .collect();
    if status == "would-pass" {
        for w in owed.iter().flatten().flat_map(|o| &o.waived) {
            if !flags.iter().any(|f| f == "waived") {
                flags.push("waived".into());
            }
            more.push(More {
                label: "waived",
                text: format!("gate {}: {}", w.gate, w.text),
                path: Some(w.path.clone()),
                line: Some(1),
                ..More::default()
            });
        }
    }
    if rule.mode == Mode::Audit {
        more.push(More {
            label: "mode",
            text: "audit: a matched call is let through and logged".into(),
            ..More::default()
        });
    }
    Row {
        kind: "guard",
        id: rule.id,
        status: Some(status.into()),
        text,
        path: Some(rule.path),
        line: Some(1),
        flags,
        more,
        ..Row::default()
    }
}

fn standing(rule: &Rule, owed: Option<&[Owed]>) -> (&'static str, Option<&'static str>) {
    let Some(owed) = owed else {
        return ("not-here", None);
    };
    let facts: Vec<Fact> = owed.iter().map(|o| o.fact).collect();
    match rule.event {
        Event::PreToolUse => match decide(true, rule.action, &facts) {
            Decision::Pass => ("would-pass", None),
            Decision::Deny => ("would-deny", Some("deny")),
            Decision::Ask => ("would-ask", Some("ask")),
        },
        Event::Stop if nudge(false, false, &facts) => ("would-nudge", Some("nudge")),
        Event::Stop => ("quiet", None),
    }
}

fn description(rule: &Rule) -> String {
    let mut text = match rule.event {
        Event::PreToolUse => "pre-tool-use".to_owned(),
        Event::Stop => "stop".to_owned(),
    };
    for word in rule.commands.iter().flatten().chain(rule.tools.iter()) {
        text.push(' ');
        text.push_str(word);
    }
    text.push_str(" requires");
    for require in &rule.require {
        text.push(' ');
        match require {
            Require::RunCurrent => text.push_str("run.current"),
            Require::GatesMet => text.push_str("gates.met"),
            Require::StepMet => text.push_str("step.met"),
            Require::StepSettled => text.push_str("step.settled"),
            Require::Gate(id) => {
                text.push_str("gate:");
                text.push_str(id);
            }
        }
    }
    text
}
