//! Tools, skills and agent profiles a person writes under `.writfold/`
//! (#144, RFC 2026-11-25-agent-vocabulary):
//!
//! - `.writfold/tools/<id>.toml` (`writfold-tool/1`): `id`, `description`.
//! - `.writfold/skills/<id>.toml` (`writfold-skill/1`): `tools`, at most
//!   one `plan`, `needs`, and exactly one of `instructions` and
//!   `instructions_file`.
//! - `.writfold/agents/<id>.toml` (`writfold-agent-profile/1`): ordered
//!   `skills`, `tools`, `packet`, the hint `model`, `needs`, and exactly one of
//!   `instructions` and `instructions_file`.
//!
//! Reading is tested ([`manifest`]); whether a profile's closure holds
//! together is decided by [`writfold_core::compose`] over the facts
//! [`refs`] gathers. `agent inspect` records nothing; `agent prepare`
//! ([`prepare`]) records an invocation ([`invocation`]) and writes a
//! package ([`package`]). Nothing here starts a model.
pub mod inspect;
pub mod invocation;
pub mod manifest;
pub mod package;
pub mod prepare;
pub mod refs;
pub mod shown;

use std::path::PathBuf;

pub const TOOLS: &str = ".writfold/tools";
pub const SKILLS: &str = ".writfold/skills";
pub const AGENTS: &str = ".writfold/agents";

/// `agent inspect`'s request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Inspect {
    pub id: String,
    pub repo: Option<PathBuf>,
}

/// One `agent` verb.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentVerb {
    Inspect(Inspect),
    Prepare(prepare::Prepare),
}

/// Answer one `agent` verb.
pub fn execute(
    ws: &crate::workspace::Workspace,
    request: &crate::Request,
    verb: &AgentVerb,
    now: i64,
) -> crate::error::Result<crate::emit::Output> {
    match verb {
        AgentVerb::Inspect(i) => inspect::run(&ws.root, request, &i.id, i.repo.as_deref()),
        AgentVerb::Prepare(p) => prepare::run(ws, request, p, now),
    }
}

/// Which of the three files a definition is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefKind {
    Tool,
    Skill,
    Agent,
}

impl DefKind {
    pub fn schema(self) -> &'static str {
        match self {
            DefKind::Tool => "writfold-tool/1",
            DefKind::Skill => "writfold-skill/1",
            DefKind::Agent => "writfold-agent-profile/1",
        }
    }

    /// `tool`, `skill`, `agent`.
    pub fn word(self) -> &'static str {
        match self {
            DefKind::Tool => "tool",
            DefKind::Skill => "skill",
            DefKind::Agent => "agent",
        }
    }

    pub fn what(self) -> &'static str {
        match self {
            DefKind::Tool => "a tool definition",
            DefKind::Skill => "a skill definition",
            DefKind::Agent => "an agent profile",
        }
    }

    pub fn not_a_key(self) -> &'static str {
        match self {
            DefKind::Tool => "not a tool key",
            DefKind::Skill => "not a skill key",
            DefKind::Agent => "not an agent profile key",
        }
    }

    pub fn dir(self) -> &'static str {
        match self {
            DefKind::Tool => TOOLS,
            DefKind::Skill => SKILLS,
            DefKind::Agent => AGENTS,
        }
    }
}

/// Where a skill's or profile's instructions are.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instructions {
    /// A tool has none.
    None,
    Inline(String),
    /// Relative to the root holding the definition.
    File(String),
}

/// One definition that parsed.
#[derive(Clone, Debug)]
pub struct Def {
    pub kind: DefKind,
    pub id: String,
    pub description: Option<String>,
    pub tools: Vec<String>,
    /// A profile's skills, in their listed order.
    pub skills: Vec<String>,
    /// A skill's plan template.
    pub plan: Option<String>,
    /// A profile requires a context packet.
    pub packet: bool,
    /// A hint: recorded and exported, not judged.
    pub model: Option<String>,
    /// Gates that must be met when a worker is set up (#154).
    pub needs: Vec<String>,
    pub instructions: Instructions,
    /// As rows show it.
    pub shown: String,
    /// The KB root or repository holding it.
    pub top: PathBuf,
}
