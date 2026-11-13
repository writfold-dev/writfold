//! Guard rule files: `.writfold/guards/<id>.toml`, schema
//! `writfold-guard-rule/1`, in the gate definitions' TOML subset
//! ([`crate::gate::toml`]) with unknown keys refused. A rule in the KB
//! root applies to every project; one in a repository applies only under
//! that repository (it is found by walking up from the working directory,
//! never by spawning Git). Finding rules only reads files.
use crate::gate::toml::{self, Value};
use crate::shell::{self, Word};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use writfold_core::guard::Action;

pub const SCHEMA: &str = "writfold-guard-rule/1";
/// Where rules live, under the KB root or a repository.
pub const DIR: &str = ".writfold/guards";
const KEYS: [&str; 9] = [
    "schema",
    "id",
    "event",
    "commands",
    "tools",
    "repos",
    "require",
    "action",
    "description",
];
/// The requirement names a rule may use; `gate:<id>` names one gate.
pub const REQUIREMENTS: [&str; 4] = ["run.current", "gates.met", "gate:<id>", "step.met"];

/// When a rule is evaluated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    PreToolUse,
    Stop,
}

/// One requirement from the fixed vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Require {
    /// The current-run pointer names an open run.
    RunCurrent,
    /// Every gate the run owes for this repository is met.
    GatesMet,
    /// This gate is met.
    Gate(String),
    /// The plan's current step owes nothing.
    StepMet,
}

impl Require {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "run.current" => Some(Require::RunCurrent),
            "gates.met" => Some(Require::GatesMet),
            "step.met" => Some(Require::StepMet),
            _ => text
                .strip_prefix("gate:")
                .filter(|id| crate::write::is_slug(id))
                .map(|id| Require::Gate(id.to_owned())),
        }
    }
}

/// One parsed rule.
#[derive(Clone, Debug)]
pub struct Rule {
    pub id: String,
    /// Shown path: `.writfold/guards/<id>.toml`, or `<dir>/.writfold/…`
    /// for a repository rule.
    pub path: String,
    pub event: Event,
    /// Commands as words (`git commit`), matched as a prefix of a simple
    /// command.
    pub commands: Vec<Vec<String>>,
    pub tools: Vec<String>,
    /// Repository-name patterns with `!` exclusions; empty: every one.
    pub repos: Vec<String>,
    pub require: Vec<Require>,
    pub action: Action,
    pub description: String,
}

impl Rule {
    /// Whether a Bash command line holds a simple command this rule names.
    pub fn matches_line(&self, line: &str) -> bool {
        shell::segments(line).iter().any(|segment| {
            let words: &[Word] = shell::command(segment);
            self.commands
                .iter()
                .any(|c| c.len() <= words.len() && c.iter().zip(words).all(|(a, w)| *a == w.text))
        })
    }
}

fn strings(map: &BTreeMap<String, Value>, key: &str) -> Result<Vec<String>, String> {
    match map.get(key) {
        None => Ok(Vec::new()),
        Some(Value::List(items)) => Ok(items.clone()),
        Some(_) => Err(format!("{key} must be an array of strings")),
    }
}

fn text(map: &BTreeMap<String, Value>, key: &str) -> Result<Option<String>, String> {
    match map.get(key) {
        None => Ok(None),
        Some(Value::Str(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("{key} must be a string")),
    }
}

fn need(map: &BTreeMap<String, Value>, key: &str) -> Result<String, String> {
    text(map, key)?.ok_or_else(|| format!("{key} is required"))
}

/// Parse one rule file whose name is `<stem>.toml`.
pub fn parse(text_in: &str, stem: &str, path: &str) -> Result<Rule, String> {
    let map = toml::parse(text_in)?;
    if let Some(key) = map.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(format!("unknown key {key}"));
    }
    if need(&map, "schema")? != SCHEMA {
        return Err(format!("schema must be {SCHEMA}"));
    }
    let id = need(&map, "id")?;
    if !crate::write::is_slug(&id) || id != stem {
        return Err(format!(
            "id {id:?} must be a slug equal to the file name {stem}"
        ));
    }
    let event = match need(&map, "event")?.as_str() {
        "pre-tool-use" => Event::PreToolUse,
        "stop" => Event::Stop,
        _ => return Err("event must be pre-tool-use or stop".into()),
    };
    let commands: Vec<Vec<String>> = strings(&map, "commands")?
        .iter()
        .map(|c| c.split_whitespace().map(str::to_owned).collect())
        .collect();
    if commands.iter().any(Vec::is_empty) {
        return Err("a command must name at least one word".into());
    }
    let tools = strings(&map, "tools")?;
    let action = match (event, text(&map, "action")?) {
        (Event::PreToolUse, Some(a)) => Action::parse(&a).ok_or("action must be deny or ask")?,
        (Event::PreToolUse, None) => return Err("action is required (deny or ask)".into()),
        (Event::Stop, Some(_)) => {
            return Err("a stop rule takes no action: it adds context once a session".into());
        }
        (Event::Stop, None) => Action::Ask,
    };
    match event {
        Event::PreToolUse if commands.is_empty() && tools.is_empty() => {
            return Err("a pre-tool-use rule names commands or tools".into());
        }
        Event::Stop if !commands.is_empty() || !tools.is_empty() => {
            return Err("a stop rule names no commands or tools".into());
        }
        _ => {}
    }
    let repos = strings(&map, "repos")?;
    if let Some(bad) = repos.iter().find(|r| {
        let bare = r.strip_prefix('!').unwrap_or(r);
        bare.is_empty() || bare.contains(['/', '\\'])
    }) {
        return Err(format!("repos pattern {bad:?} is not a repository name"));
    }
    let names = strings(&map, "require")?;
    if names.is_empty() {
        return Err("require names at least one requirement".into());
    }
    let mut require = Vec::new();
    for (n, name) in names.iter().enumerate() {
        let parsed = Require::parse(name)
            .ok_or_else(|| format!("require {name:?} is not one of {}", REQUIREMENTS.join(", ")))?;
        if names[..n].contains(name) {
            return Err(format!("require names {name} twice"));
        }
        require.push(parsed);
    }
    Ok(Rule {
        id,
        path: path.to_owned(),
        event,
        commands,
        tools,
        repos,
        require,
        action,
        description: text(&map, "description")?.unwrap_or_default(),
    })
}

/// The rule directories that apply at `cwd`: the KB root's, then the
/// nearest `.writfold/guards/` at or above `cwd` when it is another one.
/// Only directories that exist are returned, so no rule means no work.
pub fn dirs(kb: &Path, cwd: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let root = kb.join(DIR);
    if root.is_dir() {
        out.push((root.clone(), DIR.to_owned()));
    }
    let near = cwd
        .ancestors()
        .map(|dir| (dir, dir.join(DIR)))
        .find(|(_, d)| d.is_dir());
    if let Some((top, dir)) = near {
        let same = match (std::fs::canonicalize(&root), std::fs::canonicalize(&dir)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        };
        if !same {
            out.push((dir, format!("{}/{DIR}", top.display())));
        }
    }
    out
}

/// Every rule of `dirs`, and every file that does not parse (path, why).
/// An ID defined twice keeps the first (the KB root's) and reports the
/// second.
pub fn read(dirs: &[(PathBuf, String)]) -> (Vec<Rule>, Vec<(String, String)>) {
    let mut found: Vec<Rule> = Vec::new();
    let mut problems = Vec::new();
    for (dir, shown) in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".toml") && !n.starts_with('.'))
            .collect();
        names.sort();
        for name in names {
            let stem = name.trim_end_matches(".toml");
            let path = format!("{shown}/{name}");
            let parsed = std::fs::read_to_string(dir.join(&name))
                .map_err(|e| e.to_string())
                .and_then(|text| parse(&text, stem, &path));
            match parsed {
                Ok(r) if found.iter().any(|f| f.id == r.id) => {
                    problems.push((path, format!("rule {} is defined twice", r.id)));
                }
                Ok(r) => found.push(r),
                Err(why) => problems.push((path, why)),
            }
        }
    }
    (found, problems)
}

/// Every rule that applies at `cwd`, and the files that do not parse.
pub fn all(kb: &Path, cwd: &Path) -> (Vec<Rule>, Vec<(String, String)>) {
    read(&dirs(kb, cwd))
}
