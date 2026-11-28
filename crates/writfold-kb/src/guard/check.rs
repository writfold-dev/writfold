//! `writfold kb guard check` (#183): the guard site's decision for any
//! harness. It reads one event (from flags, or a harness's own event
//! through its [`binding`]), runs the delivery guard and the rules exactly
//! as `kb hook pre-tool-use` does ([`site::decide`]), writes the guard log
//! entry the hook would (`via: check`), and exits by the decision: 0
//! allow, 2 ask, 3 deny, 1 usage.
use super::binding::Binding;
use super::event::{Kind, SiteEvent};
use super::site::{self, Sited};
use crate::hook::HookEnv;
use crate::workspace::{Workspace, discover};
use serde_json::{Map, Value, json};
use std::path::PathBuf;
use writfold_core::guard::Decision;
use writfold_core::site::Basis;
pub use writfold_core::site::Posture;

/// The verb's arguments, as the command line gave them.
#[derive(Clone, Debug, Default)]
pub struct Args {
    pub kind: Option<String>,
    pub command: Option<String>,
    pub path: Option<String>,
    pub name: Option<String>,
    pub tool: Option<String>,
    pub cwd: Option<String>,
    pub session: Option<String>,
    pub actor: Option<String>,
    pub actor_type: Option<String>,
    /// A harness's own event (the text of `--from-event FILE|-`).
    pub from_event: Option<String>,
    pub harness: Option<String>,
    /// The registration's posture (`--posture`).
    pub posture: Option<String>,
    /// `--event stop`: the end of a turn, not a tool call.
    pub stop: bool,
    /// `--stop-active`: this Stop follows a block in the same turn.
    pub stop_active: bool,
    /// `--message`: the agent's last message, kept in the session's draft
    /// as the hook keeps it.
    pub message: Option<String>,
    pub json: bool,
}

/// What to print and the exit code.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Checked {
    pub stdout: String,
    pub stderr: String,
    pub exit: u8,
}

const NAME: &str = "writfold kb guard check";

/// A usage error: exit 1, or 3 under a closed posture, so "anything but
/// 0 means do not proceed" holds there (#184).
pub fn usage_under(posture: Posture, why: &str) -> Checked {
    Checked {
        stdout: String::new(),
        stderr: format!("{NAME}: {why}\n(see: writfold kb guard check --help)\n"),
        exit: match posture {
            Posture::Open => 1,
            _ => 3,
        },
    }
}

pub fn usage(why: &str) -> Checked {
    usage_under(Posture::Open, why)
}

/// The registration's posture from `--posture`; an unknown word is
/// refused as `deny` would refuse it.
pub fn posture_of(written: Option<&str>) -> Result<Posture, Checked> {
    match written {
        None => Ok(Posture::Open),
        Some(text) => Posture::parse(text).ok_or_else(|| {
            usage_under(
                Posture::Deny,
                &format!("unknown posture {text:?}: one of open, ask, deny"),
            )
        }),
    }
}

/// The event the flags describe.
fn from_flags(args: &Args) -> Result<SiteEvent, String> {
    let written = args
        .kind
        .as_deref()
        .ok_or("needs --kind (or --from-event FILE)")?;
    let kind = Kind::parse(written).ok_or_else(|| {
        format!(
            "unknown kind {written:?}: one of {}",
            super::event::KINDS.join(", ")
        )
    })?;
    let (flag, subject) = match kind {
        Kind::Shell => ("--command", &args.command),
        Kind::FileWrite | Kind::FileRead => ("--path", &args.path),
        Kind::Skill | Kind::Spawn => ("--name", &args.name),
        Kind::Tool => ("--tool", &args.tool),
    };
    if subject.is_none() {
        return Err(format!("--kind {written} needs {flag}"));
    }
    let mut event = SiteEvent::new(kind, "check");
    event.subject = (kind != Kind::Tool).then(|| subject.clone()).flatten();
    event.tool = args.tool.clone().unwrap_or_default();
    event.cwd = args.cwd.clone();
    event.session = args.session.clone();
    if let Some(actor) = &args.actor {
        event.actor = actor.clone();
    }
    event.actor_type = args.actor_type.clone();
    Ok(event)
}

/// The event a harness reported, read through its binding.
fn from_harness(args: &Args, text: &str, kb: Option<&Workspace>) -> Result<SiteEvent, String> {
    let Ok(Value::Object(raw)) = serde_json::from_str::<Value>(text) else {
        return Err("the event is not a JSON object".into());
    };
    let id = args.harness.as_deref().unwrap_or("claude-code");
    let binding = match (id, kb) {
        ("claude-code", _) => Binding::claude_code(),
        (_, Some(ws)) => Binding::named(&ws.root, id)?,
        (_, None) => return Err(format!("no KB to read the binding {id:?} from")),
    };
    let mut event = binding.read(&raw, "check");
    if let Some(cwd) = &args.cwd {
        event.cwd = Some(cwd.clone());
    }
    Ok(event)
}

fn basis_name(basis: Basis) -> &'static str {
    match basis {
        Basis::NoRules => "no-rules",
        Basis::NoMatch => "no-match",
        Basis::Judged(_) => "judged",
        Basis::Undecided => "undecided",
    }
}

fn decision_name(d: Decision) -> &'static str {
    match d {
        Decision::Pass => "allow",
        Decision::Ask => "ask",
        Decision::Deny => "deny",
    }
}

fn exit_of(d: Decision) -> u8 {
    match d {
        Decision::Pass => 0,
        Decision::Ask => 2,
        Decision::Deny => 3,
    }
}

fn text_of(rows: &[Value], key: &str) -> String {
    rows.iter().filter(|r| r[key] != "pass").count().to_string()
}

/// The human form: one line per judged rule, the delivery guard's reason,
/// then the summary.
fn render(sited: &Sited, posture: Posture) -> String {
    let mut out = String::new();
    if sited.delivery {
        for reason in &sited.reasons {
            out.push_str(&format!("delivery [deny] {reason}\n"));
        }
    }
    for row in &sited.rules {
        let owes: Vec<&str> = row["owes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let mode = if row["mode"] == "audit" {
            " (audit)"
        } else {
            ""
        };
        let command = row["command"]
            .as_str()
            .filter(|c| !c.is_empty())
            .map(|c| format!(" {c}"))
            .unwrap_or_default();
        let owed = if owes.is_empty() {
            String::new()
        } else {
            format!(" owes: {}", owes.join("; "))
        };
        out.push_str(&format!(
            "guard {} [{}{mode}]{command}{owed} @{}\n",
            row["id"].as_str().unwrap_or(""),
            row["decided"].as_str().unwrap_or(""),
            row["path"].as_str().unwrap_or(""),
        ));
    }
    let deny = sited.rules.iter().filter(|r| r["effect"] == "deny").count();
    let ask = sited.rules.iter().filter(|r| r["effect"] == "ask").count();
    out.push_str("# note: a guard, not enforcement: it decides only the calls it is asked about\n");
    out.push_str(&format!(
        "# decision={} basis={} posture={} rules={} deny={deny} ask={ask} not-pass={}\n",
        decision_name(sited.decision),
        basis_name(sited.basis),
        posture.name(),
        sited.rules.len(),
        text_of(&sited.rules, "decided"),
    ));
    out
}

fn json_of(sited: &Sited, posture: Posture, event: &SiteEvent) -> String {
    let mut owes: Vec<Value> = Vec::new();
    for o in sited
        .rules
        .iter()
        .flat_map(|r| r["owes"].as_array().into_iter().flatten())
    {
        if !owes.contains(o) {
            owes.push(o.clone());
        }
    }
    let value = json!({"writfold_guard": 1, "decision": decision_name(sited.decision),
        "basis": basis_name(sited.basis), "posture": posture.name(),
        "delivery": sited.delivery, "reasons": sited.reasons, "owes": owes,
        "rules": sited.rules, "event": event.json()});
    format!("{value}\n")
}

fn report(sited: &Sited, event: &SiteEvent, json: bool) -> Checked {
    let posture = sited.posture;
    Checked {
        stdout: match json {
            true => json_of(sited, posture, event),
            false => render(sited, posture),
        },
        stderr: sited
            .stderr
            .replace("writfold kb hook: ", &format!("{NAME}: ")),
        exit: exit_of(sited.decision),
    }
}

/// Run the verb: the KB is found from `--cwd` (else the process's
/// directory), or from the event's own `cwd` when it names another.
pub fn run(args: &Args, env: &HookEnv) -> Checked {
    let posture = match posture_of(args.posture.as_deref()) {
        Ok(posture) => posture,
        Err(refused) => return refused,
    };
    if args.stop {
        return stop(args, env, posture);
    }
    let here = args
        .cwd
        .as_deref()
        .map_or_else(|| env.cwd.clone(), PathBuf::from);
    let found = discover(env.kb.as_deref(), &here).map_err(|e| e.to_string());
    let event = match &args.from_event {
        Some(text) => from_harness(args, text, found.as_ref().ok()),
        None => from_flags(args),
    };
    let event = match event {
        Ok(event) => event,
        Err(why) => return usage_under(posture, &why),
    };
    let cwd = event
        .cwd
        .as_deref()
        .map_or_else(|| here.clone(), PathBuf::from);
    let ws = match cwd == here {
        true => found,
        false => discover(env.kb.as_deref(), &cwd).map_err(|e| e.to_string()),
    };
    let mut ws = match ws {
        Ok(ws) => ws,
        Err(why) => {
            let sited = Sited::undecided(posture, why.clone(), format!("{NAME}: {why}\n"));
            return report(&sited, &event, args.json);
        }
    };
    if let Some(id) = crate::seat::session_id(event.session.as_deref()) {
        ws.seat.session = Some(id);
    }
    let now = env.now.unwrap_or_else(crate::time::now);
    let input: Map<String, Value> = event.canonical();
    let sited = site::decide(&ws, &input, (&cwd, env.home.as_deref()), now, posture);
    report(&sited, &event, args.json)
}

/// `--event stop`: the stop rules exactly as `kb hook stop` runs them (the
/// session's draft recorded, a blocking rule counted, a nudge once); exit
/// 3 when a rule blocks the end of the turn, else 0 with any nudge.
fn stop(args: &Args, env: &HookEnv, posture: Posture) -> Checked {
    let Some(session) = &args.session else {
        return usage_under(posture, "--event stop needs --session");
    };
    let cwd = args
        .cwd
        .as_deref()
        .map_or_else(|| env.cwd.clone(), PathBuf::from);
    let event = json!({"hook_event_name": "Stop", "session_id": session,
        "cwd": cwd.display().to_string(), "stop_hook_active": args.stop_active,
        "last_assistant_message": args.message.as_deref().unwrap_or("(reported through writfold kb guard check --event stop)")});
    let out = crate::hook::run_with("stop", &event.to_string(), env, posture);
    let value: Value = serde_json::from_str(&out.stdout).unwrap_or(Value::Null);
    let stderr = out
        .stderr
        .replace("writfold kb hook: ", &format!("{NAME}: "));
    let note = "# note: a guard, not enforcement: it decides only the calls it is asked about\n";
    if let Some(reason) = value["reason"]
        .as_str()
        .filter(|_| value["decision"] == "block")
    {
        return Checked {
            stdout: format!("stop [block] {reason}\n{note}# decision=block event=stop\n"),
            stderr,
            exit: 3,
        };
    }
    let nudge = value["hookSpecificOutput"]["additionalContext"].as_str();
    let said = nudge
        .map(|n| format!("stop [nudge] {n}\n"))
        .unwrap_or_default();
    Checked {
        stdout: format!("{said}{note}# decision=allow event=stop\n"),
        stderr,
        exit: 0,
    }
}
