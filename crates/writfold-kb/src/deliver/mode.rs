//! Delivery modes. Any `.writfold/deliveries/<name>.toml` in the KB root
//! with `schema = "writfold-delivery-mode/1"` defines the mode `<name>`;
//! `pr-comment` and `slack-summary` are built in and used when no file
//! names them. A mode lists its obligations and nothing more:
//!
//! - `checks`: built-in checks (all three by default for a built-in mode,
//!   none for a user mode);
//! - `gates`: candidate gates from the KB root's `.writfold/gates/`
//!   (`subject = "candidate"`): scripts run against the candidate through
//!   `{candidate}`, and attested gates whose instructions are shown;
//! - `attest`: plain attested names, as before.
//!
//! Everything is checked when the mode is loaded: unknown keys, unknown or
//! malformed gates, a repository gate, a mode named where a gate belongs
//! and a name listed twice are refused with the file and the reason. A
//! mode cannot name another mode and a gate cannot name a gate, so no
//! cycle can form. Loading a mode never runs anything.
use super::config::{BUILTIN, DIR, bad, is_name, read, strings};
use crate::error::{KbError, Result};
use crate::gate::def::{self, Definition};
use crate::gate::toml::Value;
use std::path::Path;

/// The modes that exist without a file.
pub const MODES: [&str; 2] = ["pr-comment", "slack-summary"];
const KEYS: [&str; 7] = [
    "schema",
    "mode",
    "description",
    "checks",
    "paragraph_words",
    "gates",
    "attest",
];

/// One mode: which checks apply, the paragraph bound, its gates and its
/// attested obligations (statements a person or agent must make).
#[derive(Clone, Debug)]
pub struct Mode {
    pub name: String,
    pub checks: Vec<String>,
    pub paragraph_words: usize,
    /// Candidate gates, in the order listed.
    pub gates: Vec<Definition>,
    pub attest: Vec<String>,
    /// One line the mode file says about itself (shown with its
    /// candidates), such as what its checks cannot see.
    pub description: Option<String>,
    /// `built-in`, or the mode file's KB path.
    pub source: String,
}

/// The default of a mode: for a built-in mode every check, 150 words per
/// paragraph for a PR comment and 60 for a Slack summary; for a user mode
/// no check and 150 words.
fn default(name: &str) -> Mode {
    let builtin = MODES.contains(&name);
    Mode {
        name: name.to_owned(),
        checks: match builtin {
            true => BUILTIN.iter().map(|c| (*c).to_owned()).collect(),
            false => Vec::new(),
        },
        paragraph_words: if name == "slack-summary" { 60 } else { 150 },
        gates: Vec::new(),
        attest: Vec::new(),
        description: None,
        source: "built-in".into(),
    }
}

/// The mode `name`: `.writfold/deliveries/<name>.toml` when present, else
/// the built-in default; any other name is refused, naming the file to add.
pub fn mode(root: &Path, name: &str) -> Result<Mode> {
    if !is_name(name) || name == "guard" || name.starts_with("deny-") {
        return Err(KbError::Usage(format!(
            "--mode {name:?} is not a mode name ([a-z0-9.-], not guard or deny-*)"
        )));
    }
    let file = format!("{name}.toml");
    let schema = "writfold-delivery-mode/1";
    let Some((path, map)) = read(root, &file, schema, &KEYS)? else {
        return match MODES.contains(&name) {
            true => Ok(default(name)),
            false => Err(KbError::Refused(format!(
                "no delivery mode {name}: add {DIR}/{file} (schema = \"{schema}\", mode = \"{name}\"), or use a built-in mode: {}",
                MODES.join(", ")
            ))),
        };
    };
    if map.get("mode") != Some(&Value::Str(name.into())) {
        return Err(bad(&path, format!("mode must be {name}")));
    }
    let mut mode = default(name);
    mode.source = path.clone();
    if map.contains_key("checks") {
        mode.checks = strings(&map, "checks", &path)?;
    }
    if let Some(check) = mode.checks.iter().find(|c| !BUILTIN.contains(&c.as_str())) {
        return Err(bad(
            &path,
            format!(
                "{check} is not a built-in check ({}); list a gate under gates",
                BUILTIN.join(", ")
            ),
        ));
    }
    mode.description = match map.get("description") {
        None => None,
        Some(Value::Str(d)) if !d.is_empty() && !d.contains(['\n', '\r']) => Some(d.clone()),
        Some(_) => return Err(bad(&path, "description must be one line".into())),
    };
    mode.gates = gates(root, &strings(&map, "gates", &path)?, &path)?;
    mode.attest = strings(&map, "attest", &path)?;
    if let Some(name) = mode
        .attest
        .iter()
        .find(|n| !is_name(n) || BUILTIN.contains(&n.as_str()))
    {
        return Err(bad(
            &path,
            format!("attest name {name:?} must be a new name of [a-z0-9.-]"),
        ));
    }
    match map.get("paragraph_words") {
        None => {}
        Some(Value::Int(n)) if (1..=10_000).contains(n) => {
            mode.paragraph_words = usize::try_from(*n).unwrap_or(1);
        }
        Some(_) => return Err(bad(&path, "paragraph_words must be 1 to 10000".into())),
    }
    unique_names(&mode, &path)?;
    Ok(mode)
}

/// The candidate gates a mode names, from the KB root only.
fn gates(root: &Path, ids: &[String], path: &str) -> Result<Vec<Definition>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let (found, problems) = def::all(root, None);
    let mut out = Vec::new();
    for id in ids {
        let file = format!("{}/{id}.toml", def::DIR);
        if let Some((_, _, why)) = problems.iter().find(|(_, p, _)| *p == file) {
            return Err(bad(path, format!("gate {id}: {file}: {why}")));
        }
        let Some(d) = found.iter().find(|d| d.id == *id) else {
            let hint = match MODES.contains(&id.as_str())
                || root.join(DIR).join(format!("{id}.toml")).is_file()
            {
                true => format!("; {id} is a delivery mode, and a mode cannot name a mode"),
                false => String::new(),
            };
            return Err(bad(
                path,
                format!("unknown gate {id}: define {file} in the KB root{hint}"),
            ));
        };
        if !d.on_candidate() {
            return Err(bad(
                path,
                format!(
                    "gate {id} judges a repository; a mode's gates need subject = \"candidate\" (and {{candidate}} in a script's argv)"
                ),
            ));
        }
        out.push(d.clone());
    }
    Ok(out)
}

/// Every obligation of a mode has its own name.
fn unique_names(mode: &Mode, path: &str) -> Result<()> {
    let mut seen: Vec<&str> = Vec::new();
    let names = mode
        .checks
        .iter()
        .map(String::as_str)
        .chain(mode.gates.iter().map(|g| g.id.as_str()))
        .chain(mode.attest.iter().map(String::as_str));
    for name in names {
        if seen.contains(&name) {
            return Err(bad(path, format!("{name} is listed twice")));
        }
        seen.push(name);
    }
    Ok(())
}
