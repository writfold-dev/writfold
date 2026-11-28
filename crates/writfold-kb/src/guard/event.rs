//! `writfold-event/1`, the one form every guard site reads (#183, RFC
//! 2026-11-27-guard-check-and-posture).
//!
//! The site's judges read Claude Code's PreToolUse shape (`tool_name`,
//! `tool_input`, `session_id`), so that shape is the canonical map: an
//! event from `guard check` or another harness's binding becomes one, with
//! two keys Claude Code never sends. [`KIND`] says which kind the call is
//! (so a `commands` rule applies to every `shell` call, whatever the tool
//! is called), and [`VIA`] who reported it. A map without [`KIND`] is
//! Claude Code's own: its shell tool is `Bash`, as before, and the hook's
//! output is unchanged.
use serde_json::{Map, Value, json};

pub const SCHEMA: &str = "writfold-event/1";
/// The canonical map's key for the event's kind.
pub const KIND: &str = "writfold_kind";
/// The canonical map's key for who reported the event.
pub const VIA: &str = "writfold_via";

/// What kind of call an event is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Shell,
    FileWrite,
    FileRead,
    Skill,
    Spawn,
    Tool,
}

pub const KINDS: [&str; 6] = ["shell", "file.write", "file.read", "skill", "spawn", "tool"];

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Shell => "shell",
            Kind::FileWrite => "file.write",
            Kind::FileRead => "file.read",
            Kind::Skill => "skill",
            Kind::Spawn => "spawn",
            Kind::Tool => "tool",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "shell" => Some(Kind::Shell),
            "file.write" => Some(Kind::FileWrite),
            "file.read" => Some(Kind::FileRead),
            "skill" => Some(Kind::Skill),
            "spawn" => Some(Kind::Spawn),
            "tool" => Some(Kind::Tool),
            _ => None,
        }
    }

    /// The canonical `tool_input` field the kind's subject goes in.
    fn field(self) -> Option<&'static str> {
        match self {
            Kind::Shell => Some("command"),
            Kind::FileWrite | Kind::FileRead => Some("file_path"),
            Kind::Skill => Some("skill"),
            Kind::Spawn => Some("subagent_type"),
            Kind::Tool => None,
        }
    }
}

/// One event as a site is handed it.
#[derive(Clone, Debug)]
pub struct SiteEvent {
    /// `pre-tool` or `stop`.
    pub event: &'static str,
    pub kind: Kind,
    pub tool: String,
    /// The command line (`shell`), the path (`file.*`) or the name
    /// (`skill`, `spawn`).
    pub subject: Option<String>,
    pub input: Map<String, Value>,
    pub cwd: Option<String>,
    pub session: Option<String>,
    pub actor: String,
    pub actor_type: Option<String>,
    /// `check`, `binding:<id>`.
    pub via: String,
}

impl SiteEvent {
    pub fn new(kind: Kind, via: &str) -> Self {
        SiteEvent {
            event: "pre-tool",
            kind,
            tool: String::new(),
            subject: None,
            input: Map::new(),
            cwd: None,
            session: None,
            actor: "main".to_owned(),
            actor_type: None,
            via: via.to_owned(),
        }
    }

    /// The canonical map the site's judges read.
    pub fn canonical(&self) -> Map<String, Value> {
        let mut input = self.input.clone();
        if let (Some(field), Some(subject)) = (self.kind.field(), &self.subject) {
            input.insert(field.to_owned(), Value::from(subject.as_str()));
        }
        let mut out = Map::new();
        out.insert("hook_event_name".into(), "PreToolUse".into());
        out.insert("tool_name".into(), Value::from(self.tool.as_str()));
        out.insert("tool_input".into(), Value::Object(input));
        if let Some(session) = &self.session {
            out.insert("session_id".into(), Value::from(session.as_str()));
        }
        if let Some(cwd) = &self.cwd {
            out.insert("cwd".into(), Value::from(cwd.as_str()));
        }
        out.insert(KIND.into(), self.kind.name().into());
        out.insert(VIA.into(), Value::from(self.via.as_str()));
        out
    }

    /// The event as read, for `guard check --json`.
    pub fn json(&self) -> Value {
        let field = |kind: Kind| {
            (self.kind == kind)
                .then_some(self.subject.as_deref())
                .flatten()
        };
        let path = field(Kind::FileWrite).or(field(Kind::FileRead));
        let name = field(Kind::Skill).or(field(Kind::Spawn));
        json!({"schema": SCHEMA, "event": self.event, "kind": self.kind.name(),
            "tool": self.tool, "command": field(Kind::Shell), "path": path, "name": name,
            "input": self.input, "cwd": self.cwd, "session": self.session,
            "actor": self.actor, "actor_type": self.actor_type, "via": self.via})
    }
}

/// Whether a canonical map is a shell call: its [`KIND`], or Claude Code's
/// `Bash` when it has none.
pub fn is_shell(input: &Map<String, Value>) -> bool {
    match input.get(KIND).and_then(Value::as_str) {
        Some(kind) => kind == "shell",
        None => input.get("tool_name").and_then(Value::as_str) == Some("Bash"),
    }
}

/// The guard log's `via` for a canonical map: `check` for `guard check`
/// and a binding, `hook` for Claude Code's own event.
pub fn via(input: &Map<String, Value>) -> &'static str {
    match input.get(VIA) {
        Some(_) => "check",
        None => "hook",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_round_trip() {
        for name in KINDS {
            assert_eq!(Kind::parse(name).map(Kind::name), Some(name));
        }
        assert_eq!(Kind::parse("bash"), None);
    }

    #[test]
    fn a_shell_call_under_any_name_is_shell() {
        let mut e = SiteEvent::new(Kind::Shell, "check");
        e.tool = "run_terminal".into();
        e.subject = Some("git commit -m x".into());
        let map = e.canonical();
        assert!(is_shell(&map));
        assert_eq!(via(&map), "check");
        assert_eq!(map["tool_input"]["command"], "git commit -m x");
        let claude: Map<String, Value> =
            serde_json::from_str(r#"{"tool_name":"Bash","tool_input":{"command":"ls"}}"#)
                .unwrap_or_default();
        assert!(is_shell(&claude));
        assert_eq!(via(&claude), "hook");
    }
}
