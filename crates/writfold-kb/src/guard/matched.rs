//! Which guard rules a PreToolUse call matches, and where each is judged.
//!
//! A Bash command line is read into invocations ([`crate::shell`]); a
//! `commands` rule matches an invocation it names and is judged at that
//! invocation's directory. The KB root's rules apply to every invocation;
//! a repository's own rules (the nearest `.writfold/guards/` at or above a
//! directory) to the invocations at or below it, and only the KB root's to
//! one whose directory cannot be told. A `tools` rule is judged at the
//! event's working directory. Finding rules only reads files: one upward
//! walk by `stat` per distinct directory.
use super::paths;
use super::require::Place;
use super::rules::{self, Event, Rule};
use crate::shell::{self, Dir, Invocation, Why};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

/// At most this many distinct directories are judged per call; a matched
/// invocation beyond them cannot be told ([`Why::TooMany`]).
pub const MOST: usize = 4;

/// The rule files that apply somewhere in one call.
#[derive(Default)]
pub struct Loaded {
    root: Vec<Rule>,
    /// Each repository rule directory found, with its rules.
    repos: Vec<(PathBuf, Vec<Rule>)>,
    /// For each directory looked at, the index of its repository rules.
    near: Vec<(PathBuf, Option<usize>)>,
    pub problems: Vec<(String, String)>,
}

impl Loaded {
    /// Read the KB root's rules and those near each of `places`. `None`
    /// when no rule directory applies anywhere (nothing was read).
    pub fn find(kb: &Path, places: &[PathBuf]) -> Option<Self> {
        let root = rules::root_dir(kb);
        let mut loaded = Loaded::default();
        let mut dirs: Vec<(PathBuf, String)> = Vec::new();
        for place in places {
            let index = rules::near_dir(kb, place).map(|near| {
                let at = dirs.iter().position(|(d, _)| *d == near.0);
                at.unwrap_or_else(|| {
                    dirs.push(near);
                    dirs.len() - 1
                })
            });
            loaded.near.push((place.clone(), index));
        }
        if root.is_none() && dirs.is_empty() {
            return None;
        }
        let (found, problems) = rules::read(root.as_slice());
        loaded.root = found;
        loaded.problems = problems;
        for dir in dirs {
            let (found, problems) = rules::read(std::slice::from_ref(&dir));
            loaded.problems.extend(problems);
            let mut kept = Vec::new();
            for rule in found {
                match loaded.root.iter().any(|r| r.id == rule.id) {
                    true => {
                        let why = format!("rule {} is defined twice", rule.id);
                        loaded.problems.push((rule.path.clone(), why));
                    }
                    false => kept.push(rule),
                }
            }
            loaded.repos.push((dir.0, kept));
        }
        Some(loaded)
    }

    /// The pre-tool-use rules that apply at `dir` (`None`: the KB root's
    /// only).
    fn at(&self, dir: Option<&Path>) -> impl Iterator<Item = &Rule> {
        let index = dir.and_then(|d| self.near.iter().find(|(p, _)| p == d).and_then(|(_, i)| *i));
        let repo = index.and_then(|i| self.repos.get(i)).map(|(_, r)| r);
        self.root
            .iter()
            .chain(repo.into_iter().flatten())
            .filter(|r| r.event == Event::PreToolUse)
    }

    fn any(&self, f: impl Fn(&Rule) -> bool) -> bool {
        self.root
            .iter()
            .chain(self.repos.iter().flat_map(|(_, r)| r))
            .any(f)
    }
}

/// One call, as read from the event.
pub struct Call<'a> {
    pub input: &'a Map<String, Value>,
    pub tool: &'a str,
    pub cwd: &'a Path,
    pub invocations: Vec<Invocation>,
}

impl<'a> Call<'a> {
    pub fn read(input: &'a Map<String, Value>, cwd: &'a Path, home: Option<&Path>) -> Self {
        let tool = input.get("tool_name").and_then(Value::as_str).unwrap_or("");
        let line = input
            .get("tool_input")
            .and_then(|t| t.get("command"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let invocations = match tool == "Bash" {
            true => {
                let home = home.map(|h| h.to_string_lossy().into_owned());
                shell::invocations(line, &cwd.to_string_lossy(), home.as_deref())
            }
            false => Vec::new(),
        };
        Self {
            input,
            tool,
            cwd,
            invocations,
        }
    }

    /// The directories rules are looked for at: the working directory,
    /// then each distinct resolved invocation directory.
    pub fn places(&self) -> Vec<PathBuf> {
        let mut out = vec![self.cwd.to_path_buf()];
        for inv in &self.invocations {
            if let Dir::At(d) = &inv.dir {
                let d = PathBuf::from(d);
                if !out.contains(&d) {
                    out.push(d);
                }
            }
        }
        out
    }
}

/// Why a matched invocation's directory cannot be told, with the way out.
pub fn unresolved(inv: &Invocation, why: Why) -> String {
    format!(
        "cannot tell which repository this command acts on ({}: {}); name the directory literally",
        inv.words.join(" "),
        why.text()
    )
}

/// The rules matched at one place, each with the command it matched (its
/// words, empty for a `tools` rule).
pub struct Group<'r> {
    pub place: Place,
    pub rules: Vec<&'r Rule>,
    pub commands: Vec<String>,
}

/// The matched rules grouped by where each is judged, in order of first
/// match; a rule is judged once per place.
pub fn groups<'r>(loaded: &'r Loaded, call: &Call<'_>) -> Vec<Group<'r>> {
    let mut hits: Vec<(Dir, &Rule, Option<&Invocation>)> = Vec::new();
    let file = loaded
        .any(|r| !r.paths.is_empty())
        .then(|| paths::target(call.input, call.cwd))
        .flatten();
    let cwd: PathBuf = call.cwd.components().collect();
    let here = Dir::At(cwd.to_string_lossy().into_owned());
    for rule in loaded.at(Some(call.cwd)) {
        if rule.tools.iter().any(|t| t == call.tool) && paths::admits(&rule.paths, file.as_deref())
        {
            hits.push((here.clone(), rule, None));
        }
    }
    for inv in &call.invocations {
        let dir = match &inv.dir {
            Dir::At(d) => Some(Path::new(d)),
            Dir::Unresolved(_) => None,
        };
        for rule in loaded.at(dir).filter(|r| r.names(&inv.words)) {
            hits.push((inv.dir.clone(), rule, Some(inv)));
        }
    }
    let mut out: Vec<(Dir, Group<'r>)> = Vec::new();
    let mut judged = 0;
    for (dir, rule, inv) in hits {
        let known = out.iter().any(|(d, _)| *d == dir);
        let dir = match (&dir, known) {
            (Dir::At(_), false) if judged == MOST => Dir::Unresolved(Why::TooMany),
            _ => dir,
        };
        let slot = match out.iter().position(|(d, _)| *d == dir) {
            Some(i) => i,
            None => {
                let place = match (&dir, inv) {
                    (Dir::At(d), _) => {
                        judged += 1;
                        Place::At(PathBuf::from(d))
                    }
                    (Dir::Unresolved(why), Some(inv)) => Place::Unknown(unresolved(inv, *why)),
                    (Dir::Unresolved(why), None) => Place::Unknown(why.text().to_owned()),
                };
                let group = Group {
                    place,
                    rules: Vec::new(),
                    commands: Vec::new(),
                };
                out.push((dir, group));
                out.len() - 1
            }
        };
        let group = &mut out[slot].1;
        if !group.rules.iter().any(|r| r.id == rule.id) {
            group.rules.push(rule);
            group
                .commands
                .push(inv.map(|i| i.words.join(" ")).unwrap_or_default());
        }
    }
    out.into_iter().map(|(_, group)| group).collect()
}
