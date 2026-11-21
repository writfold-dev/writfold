//! Guard rule files: `.writfold/guards/<id>.toml`, schema
//! `writfold-guard-rule/1`, in the gate definitions' TOML subset
//! ([`crate::defs::toml`]) with unknown keys refused. A rule in the KB
//! root applies to every project; one in a repository applies only under
//! that repository (it is found by walking up from the working directory,
//! never by spawning Git). Finding rules only reads files.
use crate::defs::{Problem, read_text};
use crate::shell;
use std::path::{Path, PathBuf};
use writfold_core::diag::Report;
use writfold_core::guard::{Action, Mode, Repeat};

pub const SCHEMA: &str = "writfold-guard-rule/1";
/// Where rules live, under the KB root or a repository.
pub const DIR: &str = ".writfold/guards";
/// The requirement names a rule may use; `gate:<id>` names one gate.
pub const REQUIREMENTS: [&str; 5] = [
    "run.current",
    "gates.met",
    "gate:<id>",
    "step.met",
    "step.settled",
];

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
    /// `step.met`, or the current step is a spent loop (#79).
    StepSettled,
}

impl Require {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "run.current" => Some(Require::RunCurrent),
            "gates.met" => Some(Require::GatesMet),
            "step.met" => Some(Require::StepMet),
            "step.settled" => Some(Require::StepSettled),
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
    /// File-path globs with `!` exclusions ([`writfold_core::scope`]),
    /// relative to the file's repository; empty: any call of `tools`.
    pub paths: Vec<String>,
    /// Repository-name patterns with `!` exclusions; empty: every one.
    pub repos: Vec<String>,
    pub require: Vec<Require>,
    pub action: Action,
    /// How often a stop rule nudges; `Session` for a pre-tool-use rule.
    pub repeat: Repeat,
    /// `audit`: judged and logged, never decides the call.
    pub mode: Mode,
    pub description: String,
}

impl Rule {
    /// Whether one invocation's words are a command this rule names
    /// ([`shell::matches`]: a whole-word prefix, not before a lone
    /// `--help`).
    pub fn names(&self, words: &[String]) -> bool {
        self.commands.iter().any(|c| shell::matches(c, words))
    }
}

/// Parse and check one rule file whose name is `<stem>.toml`; every
/// fault is in the report ([`super::rule_check`]).
pub fn parse(text_in: &str, stem: &str, path: &str) -> Result<Rule, Report> {
    super::rule_check::check(text_in, stem, path)
}

/// The KB root's rule directory, when it exists.
pub fn root_dir(kb: &Path) -> Option<(PathBuf, String)> {
    let root = kb.join(DIR);
    root.is_dir().then(|| (root, DIR.to_owned()))
}

/// The nearest `.writfold/guards/` at or above `dir` when it is not the
/// KB root's (found by `stat`, never by Git), with its shown path.
pub fn near_dir(kb: &Path, dir: &Path) -> Option<(PathBuf, String)> {
    let (top, found) = dir
        .ancestors()
        .map(|d| (d, d.join(DIR)))
        .find(|(_, d)| d.is_dir())?;
    let same = match (
        std::fs::canonicalize(kb.join(DIR)),
        std::fs::canonicalize(&found),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    (!same).then(|| (found, format!("{}/{DIR}", top.display())))
}

/// The rule directories that apply at `cwd`: the KB root's, then the
/// nearest `.writfold/guards/` at or above `cwd` when it is another one.
/// Only directories that exist are returned, so no rule means no work.
pub fn dirs(kb: &Path, cwd: &Path) -> Vec<(PathBuf, String)> {
    root_dir(kb).into_iter().chain(near_dir(kb, cwd)).collect()
}

/// Every rule of `dirs`, and every file that does not parse, with all its
/// faults. An ID defined twice keeps the first (the KB root's) and
/// reports the second.
pub fn read(dirs: &[(PathBuf, String)]) -> (Vec<Rule>, Vec<Problem>) {
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
            let parsed = read_text(&dir.join(&name), &path).and_then(|text| {
                parse(&text, stem, &path).map_err(|report| Problem {
                    path: path.clone(),
                    report,
                })
            });
            match parsed {
                Ok(r) if found.iter().any(|f| f.id == r.id) => {
                    problems.push(twice(&r));
                }
                Ok(r) => found.push(r),
                Err(problem) => problems.push(problem),
            }
        }
    }
    (found, problems)
}

/// A rule whose ID another rule file already defines.
pub fn twice(rule: &Rule) -> Problem {
    let why = format!("rule {} is defined twice", rule.id);
    Problem::whole(&rule.path, &rule.path, "file.twice", why)
}

/// Every rule that applies at `cwd`, and the files that do not parse.
pub fn all(kb: &Path, cwd: &Path) -> (Vec<Rule>, Vec<Problem>) {
    read(&dirs(kb, cwd))
}
