//! `writfold kb hook <event>`: the Claude Code plugin's hooks.
//!
//! Each hook reads the event's JSON on stdin and never fails the session:
//! every problem (no KB, no Git, unreadable input, an error in a verb)
//! leaves stdout empty, puts one diagnostic line on stderr (which Claude
//! Code shows only in its debug log) and exits 0. Nothing here commits or
//! writes a tracked file; drafts are local ([`crate::draft`]).
//!
//! - `session-start` prints JSON whose `additionalContext` holds `brief`,
//!   `resume` when a current run exists, and the unconfirmed drafts, cut
//!   to [`CONTEXT_LIMIT`] characters (Claude Code caps it at 10,000).
//! - `pre-compact` records where the work is (repository, branch, HEAD,
//!   dirty count) in the session's draft and prints plain instructions,
//!   which Claude Code adds to the compaction prompt.
//! - `post-compact` keeps the compaction summary as the draft's body.
//! - `stop` keeps the last assistant message (no Git spawn: it runs every
//!   turn).
//! - `session-end` records the end reason (no Git spawn: the budget is
//!   1.5 s).
use crate::draft;
use crate::workspace::{GitTool, Workspace, discover};
use crate::{Request, Verb, runs, time};
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};

/// The most characters `session-start` injects.
pub const CONTEXT_LIMIT: usize = 9000;
/// Token budgets of the two verbs `session-start` runs.
pub const BRIEF_BUDGET: usize = 1200;
pub const RESUME_BUDGET: usize = 500;
/// Drafts listed at session start.
const DRAFTS_SHOWN: usize = 3;

pub const EVENTS: [&str; 5] = [
    "session-start",
    "pre-compact",
    "post-compact",
    "stop",
    "session-end",
];

/// Where the hook runs: the KB override (the plugin's `kb_root` option),
/// the process working directory (used when the input has no `cwd`), and
/// a fixed clock for tests.
#[derive(Clone, Debug, Default)]
pub struct HookEnv {
    pub kb: Option<PathBuf>,
    pub cwd: PathBuf,
    pub now: Option<i64>,
}

/// What to print. The exit code is always 0.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HookOutput {
    pub stdout: String,
    pub stderr: String,
}

fn quiet(why: impl Into<String>) -> HookOutput {
    HookOutput {
        stdout: String::new(),
        stderr: format!("writfold kb hook: {}\n", why.into()),
    }
}

/// Run one hook over its stdin text.
pub fn run(event: &str, input: &str, env: &HookEnv) -> HookOutput {
    if !EVENTS.contains(&event) {
        return quiet(format!("unknown event {event:?}"));
    }
    let Ok(Value::Object(input)) = serde_json::from_str::<Value>(input) else {
        return quiet("stdin is not a JSON object");
    };
    let named = input.get("hook_event_name").and_then(Value::as_str);
    if named.is_some_and(|n| camel(event) != n) {
        return quiet(format!("{event} was given a {named:?} event"));
    }
    let cwd = input
        .get("cwd")
        .and_then(Value::as_str)
        .map_or_else(|| env.cwd.clone(), PathBuf::from);
    let ws = match discover(env.kb.as_deref(), &cwd) {
        Ok(ws) => ws,
        Err(error) => return quiet(error.to_string()),
    };
    let now = env.now.unwrap_or_else(time::now);
    let result = if event == "session-start" {
        session_start(&ws, &cwd, now)
    } else {
        record(event, &ws, &input, &cwd, now)
    };
    result.unwrap_or_else(quiet)
}

/// `session-start` → `SessionStart`.
fn camel(event: &str) -> String {
    event
        .split('-')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_ascii_uppercase().to_string() + chars.as_str()
            })
        })
        .collect()
}

fn state(ws: &Workspace) -> Result<PathBuf, String> {
    ws.repo
        .as_ref()
        .map(|repo| repo.git_dir.join("writfold"))
        .ok_or_else(|| "the KB is not under Git; drafts need its Git directory".into())
}

/// One verb's text, if it answered (exit 0 or 2).
fn verb(ws: &Workspace, cwd: &Path, now: i64, verb: Verb, budget: usize) -> Option<String> {
    let request = Request {
        verb,
        kb: Some(ws.root.clone()),
        cwd: cwd.to_path_buf(),
        budget,
        cursor: None,
        json: false,
        now: Some(now),
        probe: crate::write::no_probe,
    };
    crate::run(&request)
        .ok()
        .filter(|output| matches!(output.exit, 0 | 2))
        .map(|output| output.text)
}

fn session_start(ws: &Workspace, cwd: &Path, now: i64) -> Result<HookOutput, String> {
    let brief = Verb::Brief {
        project: None,
        all: false,
    };
    let mut text = String::new();
    if let Some(found) = verb(ws, cwd, now, brief, BRIEF_BUDGET) {
        text.push_str("Writfold KB brief (`writfold kb brief`). Rows end in their source; re-check any row with a ! flag before relying on it.\n");
        text.push_str(&found);
    }
    let local = state(ws).ok();
    let current = local.as_deref().and_then(runs::current);
    if let Some(run) = &current {
        let resume = Verb::Resume {
            run: Some(run.clone()),
        };
        if let Some(found) = verb(ws, cwd, now, resume, RESUME_BUDGET) {
            text.push_str("\nCurrent run (`writfold kb resume`):\n");
            text.push_str(&found);
        }
    }
    // `resume` already lists the drafts of the current run and of no run.
    let shown_by_resume = |d: &Map<String, Value>| {
        current.is_some() && draft::text(d, "run").is_none_or(|run| Some(run) == current)
    };
    let drafts: Vec<_> = local
        .as_deref()
        .map(draft::all)
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, d)| !shown_by_resume(d))
        .collect();
    if !drafts.is_empty() {
        text.push_str("\nUnconfirmed checkpoint drafts (local only; confirm one with `writfold kb checkpoint --from-draft SESSION`):\n");
        for (session, d) in drafts.iter().take(DRAFTS_SHOWN) {
            text.push_str(&draft_line(session, d));
        }
        if drafts.len() > DRAFTS_SHOWN {
            text.push_str(&format!("(+{} more)\n", drafts.len() - DRAFTS_SHOWN));
        }
    }
    if text.is_empty() {
        return Err("no brief, run or draft to inject".into());
    }
    let context = cap(&text, CONTEXT_LIMIT);
    let out = json!({"hookSpecificOutput": {
        "hookEventName": "SessionStart",
        "additionalContext": context,
    }});
    Ok(HookOutput {
        stdout: format!("{out}\n"),
        stderr: String::new(),
    })
}

/// `draft <session> <updated> [run=<id>] <first line>`.
pub fn draft_line(session: &str, d: &Map<String, Value>) -> String {
    let mut line = format!(
        "draft {session} {}",
        draft::text(d, "updated").unwrap_or_default()
    );
    if let Some(run) = draft::text(d, "run") {
        line.push_str(&format!(" run={run}"));
    }
    let first = draft::body(d)
        .and_then(|b| b.lines().find(|l| !l.trim().is_empty()).map(str::to_owned))
        .unwrap_or_else(|| "(no text yet)".into());
    let first: String = first.trim().chars().take(120).collect();
    format!("{line} {first}\n")
}

/// At most `limit` characters, cut at a line break, with a closing note.
pub fn cap(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let marker = "[… cut to fit the session-start limit; run the verbs above for the rest]\n";
    let keep = limit.saturating_sub(marker.chars().count());
    let cut = text.char_indices().nth(keep).map_or(text.len(), |(i, _)| i);
    let head = &text[..cut];
    let head = head.rfind('\n').map_or(head, |at| &head[..=at]);
    format!("{head}{marker}")
}

/// The draft-writing events.
fn record(
    event: &str,
    ws: &Workspace,
    input: &Map<String, Value>,
    cwd: &Path,
    now: i64,
) -> Result<HookOutput, String> {
    let local = state(ws)?;
    let session = input
        .get("session_id")
        .and_then(Value::as_str)
        .filter(|s| draft::valid_session(s))
        .ok_or("no usable session_id in the input")?;
    let field = |key: &str| {
        input
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let existing = draft::load(&local, session).map_err(|e| e.to_string())?;
    let (key, value) = match event {
        "pre-compact" => ("place", place(cwd)?),
        "post-compact" => match field("compact_summary") {
            Some(text) => (
                "summary",
                Value::String(draft::clip(draft::summary_block(text))),
            ),
            None => return Err("no compact_summary".into()),
        },
        "stop" => match field("last_assistant_message") {
            Some(text) => ("last_message", Value::String(draft::clip(text))),
            None => return Err("no last_assistant_message".into()),
        },
        _ if existing.is_none() => return Err("no draft to end".into()),
        _ => (
            "ended",
            Value::String(field("reason").unwrap_or("other").into()),
        ),
    };
    let stamp = time::stamp(now);
    let mut d = existing.unwrap_or_else(|| {
        let mut d = Map::new();
        d.insert("schema".into(), draft::SCHEMA.into());
        d.insert("session".into(), session.into());
        d.insert("created".into(), stamp.clone().into());
        d.insert("cwd".into(), cwd.to_string_lossy().into_owned().into());
        d
    });
    if let Some(run) = runs::current(&local) {
        d.insert("run".into(), run.into());
    }
    if let Some(trigger) = field("trigger") {
        d.insert("trigger".into(), trigger.into());
    }
    d.insert(key.into(), value);
    d.insert("updated".into(), stamp.into());
    draft::save(&local, session, &d).map_err(|e| e.to_string())?;
    let stdout = if event == "pre-compact" {
        instructions(session)
    } else {
        String::new()
    };
    Ok(HookOutput {
        stdout,
        stderr: String::new(),
    })
}

/// The repository facts of `cwd`, as a JSON object (empty outside one).
fn place(cwd: &Path) -> Result<Value, String> {
    let tool = GitTool::locate().map_err(|e| e.to_string())?;
    let git = tool.admit().map_err(|e| e.to_string())?;
    let mut map = Map::new();
    if let Some(found) = crate::write::run::observe_place(&git, cwd).map_err(|e| e.to_string())? {
        for (key, value) in found.fields {
            map.insert(
                if key == "base" { "head" } else { key }.into(),
                value.into(),
            );
        }
        map.insert("dirty".into(), found.dirty.into());
    }
    Ok(Value::Object(map))
}

/// What `pre-compact` asks the compaction summary to keep.
pub fn instructions(session: &str) -> String {
    format!(
        "Writfold keeps this summary as a local checkpoint draft for session {session}; \
         it is committed to the project KB only if someone confirms it with \
         `writfold kb checkpoint --from-draft {session}`. So that it can stand as a \
         checkpoint, keep: the task and its goal; each decision made, with its reason; \
         what was tried and failed; open questions; the exact next step; and the files, \
         commits and KB record IDs involved.\n"
    )
}
