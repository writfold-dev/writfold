//! Harness bindings (#183): how one harness's own event becomes
//! [`SiteEvent`]. Claude Code's is built in; another harness's is a file
//! in the KB, `.writfold/harness/<id>.toml` (`writfold-harness/1`),
//! checked like every definition, each fault at its line and column.
//!
//! ```toml
//! schema = "writfold-harness/1"
//! id = "example"
//! session = "sessionId"
//! shell = ["bash: command", "terminal: commandLine"]   # tool name: input field
//! file_write = ["edit: path"]
//! reports = ["shell", "file.write"]
//! ```
//!
//! The event's tool name, tool input and working directory are read at
//! `tool`, `input` and `cwd` (defaults `tool_name`, `tool_input`, `cwd`).
//! A tool no list names is of kind `tool`.
use super::event::{KINDS, Kind, SiteEvent};
use crate::defs::toml::Spanned;
use crate::defs::{Check, Field};
use crate::gate::def_check::schema;
use serde_json::{Map, Value};
use std::path::Path;
use writfold_core::diag::Report;

pub const SCHEMA: &str = "writfold-harness/1";
pub const DIR: &str = ".writfold/harness";
const KEYS: [&str; 14] = [
    "schema",
    "id",
    "session",
    "actor",
    "actor_type",
    "tool",
    "input",
    "cwd",
    "shell",
    "file_write",
    "file_read",
    "skill",
    "spawn",
    "reports",
];
const LISTS: [(&str, Kind); 5] = [
    ("shell", Kind::Shell),
    ("file_write", Kind::FileWrite),
    ("file_read", Kind::FileRead),
    ("skill", Kind::Skill),
    ("spawn", Kind::Spawn),
];

/// One harness's binding.
#[derive(Clone, Debug)]
pub struct Binding {
    pub id: String,
    session: String,
    actor: Option<String>,
    actor_type: Option<String>,
    tool: String,
    input: String,
    cwd: String,
    /// Each named tool, its kind and the input field its subject is in.
    tools: Vec<(String, Kind, String)>,
    /// The kinds this harness shows a hook.
    pub reports: Vec<Kind>,
}

fn named(list: &[(&str, Kind, &str)]) -> Vec<(String, Kind, String)> {
    list.iter()
        .map(|(t, k, f)| ((*t).to_owned(), *k, (*f).to_owned()))
        .collect()
}

impl Binding {
    /// Claude Code's own event.
    pub fn claude_code() -> Self {
        Binding {
            id: "claude-code".into(),
            session: "session_id".into(),
            actor: Some("agent_id".into()),
            actor_type: Some("agent_type".into()),
            tool: "tool_name".into(),
            input: "tool_input".into(),
            cwd: "cwd".into(),
            tools: named(&[
                ("Bash", Kind::Shell, "command"),
                ("Write", Kind::FileWrite, "file_path"),
                ("Edit", Kind::FileWrite, "file_path"),
                ("MultiEdit", Kind::FileWrite, "file_path"),
                ("NotebookEdit", Kind::FileWrite, "notebook_path"),
                ("Read", Kind::FileRead, "file_path"),
                ("Skill", Kind::Skill, "skill"),
                ("Task", Kind::Spawn, "subagent_type"),
                ("Agent", Kind::Spawn, "subagent_type"),
            ]),
            reports: vec![
                Kind::Shell,
                Kind::FileWrite,
                Kind::FileRead,
                Kind::Skill,
                Kind::Spawn,
                Kind::Tool,
            ],
        }
    }

    /// The binding named `id`: the built-in one, or the KB's file.
    pub fn named(kb: &Path, id: &str) -> Result<Self, String> {
        if id == "claude-code" {
            return Ok(Binding::claude_code());
        }
        if !crate::write::is_slug(id) {
            return Err(format!("{id:?} is not a harness id"));
        }
        let path = format!("{DIR}/{id}.toml");
        let text = std::fs::read_to_string(kb.join(&path))
            .map_err(|e| format!("cannot read the binding {path}: {e}"))?;
        check(&text, id, &path).map_err(|r| crate::defs::why(&r))
    }

    /// One event of this harness, as `via`.
    pub fn read(&self, event: &Map<String, Value>, via: &str) -> SiteEvent {
        let text = |key: &str| event.get(key).and_then(Value::as_str).map(str::to_owned);
        let tool = text(&self.tool).unwrap_or_default();
        let input = event
            .get(&self.input)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let found = self.tools.iter().find(|(t, _, _)| *t == tool);
        let kind = found.map(|(_, k, _)| *k).unwrap_or(Kind::Tool);
        let mut out = SiteEvent::new(kind, via);
        out.subject =
            found.and_then(|(_, _, f)| input.get(f).and_then(Value::as_str).map(str::to_owned));
        out.tool = tool;
        out.input = input;
        out.cwd = text(&self.cwd);
        out.session = text(&self.session);
        if let Some(actor) = self.actor.as_deref().and_then(text) {
            out.actor = actor;
        }
        out.actor_type = self.actor_type.as_deref().and_then(text);
        out
    }
}

fn pairs(c: &mut Check<'_>, key: &str, kind: Kind, out: &mut Vec<(String, Kind, String)>) -> bool {
    let empty = Spanned {
        value: Vec::new(),
        span: Default::default(),
    };
    let Some(items) = c.list(key).or(empty) else {
        return false;
    };
    let mut ok = true;
    for item in items.value {
        match item
            .str()
            .split_once(':')
            .map(|(t, f)| (t.trim(), f.trim()))
        {
            Some((tool, field)) if !tool.is_empty() && !field.is_empty() => {
                if out.iter().any(|(t, _, _)| t == tool) {
                    let why = format!("tool {tool:?} is bound twice");
                    c.error("value.twice", Some(item.span), why)
                        .label("bound again");
                    ok = false;
                } else {
                    out.push((tool.to_owned(), kind, field.to_owned()));
                }
            }
            _ => {
                let why = format!("{:?} is not `tool name: input field`", item.str());
                c.error("value.shape", Some(item.span), why)
                    .help(format!("write it as {key} = [\"bash: command\"]"));
                ok = false;
            }
        }
    }
    ok
}

fn reports(c: &mut Check<'_>) -> Option<Vec<Kind>> {
    let items = c.need_list("reports", "a harness binding", "reports = [\"shell\"]")?;
    let mut out = Vec::new();
    let mut ok = true;
    for item in items {
        match Kind::parse(item.str()) {
            Some(kind) if !out.contains(&kind) => out.push(kind),
            Some(_) => {
                let why = format!("`reports` names {} twice", item.str());
                c.error("value.twice", Some(item.span), why)
                    .label("named again");
                ok = false;
            }
            None => {
                let why = format!("unknown kind {:?}", item.str());
                c.unknown_value(item.span, why, "not a kind", item.str(), &KINDS, '"');
                ok = false;
            }
        }
    }
    ok.then_some(out)
}

/// An optional string key: `Some(None)` absent, `None` bad.
fn optional(c: &mut Check<'_>, key: &str) -> Option<Option<String>> {
    match c.text(key) {
        Field::Got(t) => Some(Some(t.value.text)),
        Field::Absent => Some(None),
        Field::Bad => None,
    }
}

fn key_or(c: &mut Check<'_>, key: &str, default: &str) -> Option<String> {
    optional(c, key).map(|v| v.unwrap_or_else(|| default.to_owned()))
}

/// Parse and check one binding file whose name is `<stem>.toml`.
pub fn check(text: &str, stem: &str, path: &str) -> Result<Binding, Report> {
    let mut c = Check::read(path, text, "a harness binding");
    c.unknown_keys(&KEYS, &KEYS, "not a harness binding key");
    schema(&mut c, SCHEMA);
    let id = c.need_text("id", &format!("id = \"{stem}\""));
    let id = id.filter(|id| c.named("id", id, stem));
    let session = c.need_text("session", "session = \"session_id\"");
    let actor = optional(&mut c, "actor");
    let actor_type = optional(&mut c, "actor_type");
    let tool = key_or(&mut c, "tool", "tool_name");
    let input = key_or(&mut c, "input", "tool_input");
    let cwd = key_or(&mut c, "cwd", "cwd");
    let mut tools = Vec::new();
    let mut ok = true;
    for (key, kind) in LISTS {
        ok &= pairs(&mut c, key, kind, &mut tools);
    }
    let reports = reports(&mut c);
    let binding = (|| {
        ok.then_some(())?;
        Some(Binding {
            id: id?.value.text,
            session: session?.value.text,
            actor: actor?,
            actor_type: actor_type?,
            tool: tool?,
            input: input?,
            cwd: cwd?,
            tools,
            reports: reports?,
        })
    })();
    c.finish(binding)
}
