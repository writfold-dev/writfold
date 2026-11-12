//! Delivery configuration under the KB root's `.writfold/deliveries/`:
//! a mode file per mode (optional; built-in defaults otherwise), a deny
//! list per audience, and the guard hook's list of publishing commands.
//! Every file is the gate definitions' TOML subset ([`crate::gate::toml`]);
//! unknown keys are refused, so a misspelt field never silently changes a
//! check.
use crate::error::{KbError, Result};
use crate::gate::toml::{self, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// Where delivery configuration lives, under the KB root.
pub const DIR: &str = ".writfold/deliveries";
pub const MODES: [&str; 2] = ["pr-comment", "slack-summary"];
pub const AUDIENCES: [&str; 2] = ["internal", "external"];
pub const PARAGRAPHS: &str = "paragraphs.bounded";
pub const REFERENCES: &str = "references.denied";
pub const CLAIMS: &str = "claims.cited-fresh";
/// The built-in checks, in the order they run and are shown.
pub const BUILTIN: [&str; 3] = [PARAGRAPHS, REFERENCES, CLAIMS];

/// One mode: which checks apply, the paragraph bound, and the attested
/// obligations (statements a person or agent must make).
#[derive(Clone, Debug)]
pub struct Mode {
    pub name: String,
    pub checks: Vec<String>,
    pub paragraph_words: usize,
    pub attest: Vec<String>,
    /// `built-in`, or the mode file's KB path.
    pub source: String,
}

/// The built-in default of a mode: every check; 150 words per paragraph
/// for a PR comment, 60 for a Slack summary.
fn default(name: &str) -> Mode {
    Mode {
        name: name.to_owned(),
        checks: BUILTIN.iter().map(|c| (*c).to_owned()).collect(),
        paragraph_words: if name == "slack-summary" { 60 } else { 150 },
        attest: Vec::new(),
        source: "built-in".into(),
    }
}

fn bad(path: &str, why: String) -> KbError {
    KbError::Refused(format!("{path}: {why}"))
}

/// Parse a configuration file: its keys must be among `keys`, and its
/// `schema` must be `schema`. `None` when the file does not exist.
fn read(
    root: &Path,
    name: &str,
    schema: &str,
    keys: &[&str],
) -> Result<Option<(String, BTreeMap<String, Value>)>> {
    let path = format!("{DIR}/{name}");
    let text = match std::fs::read_to_string(root.join(&path)) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let map = toml::parse(&text).map_err(|why| bad(&path, why))?;
    if let Some(key) = map.keys().find(|k| !keys.contains(&k.as_str())) {
        return Err(bad(&path, format!("unknown key {key}")));
    }
    if map.get("schema") != Some(&Value::Str(schema.into())) {
        return Err(bad(&path, format!("schema must be {schema}")));
    }
    Ok(Some((path, map)))
}

fn strings(map: &BTreeMap<String, Value>, key: &str, path: &str) -> Result<Vec<String>> {
    match map.get(key) {
        None => Ok(Vec::new()),
        Some(Value::List(items)) => Ok(items.clone()),
        Some(_) => Err(bad(path, format!("{key} must be an array of strings"))),
    }
}

/// An obligation name: lower-case letters, digits, `.` and `-`.
pub fn is_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        && !name.starts_with(['.', '-'])
}

/// The mode `name`: `.writfold/deliveries/<name>.toml` when present, else
/// the built-in default.
pub fn mode(root: &Path, name: &str) -> Result<Mode> {
    if !MODES.contains(&name) {
        return Err(KbError::Usage(format!(
            "--mode must be one of {}",
            MODES.join(", ")
        )));
    }
    let keys = ["schema", "mode", "checks", "paragraph_words", "attest"];
    let Some((path, map)) = read(
        root,
        &format!("{name}.toml"),
        "writfold-delivery-mode/1",
        &keys,
    )?
    else {
        return Ok(default(name));
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
        return Err(bad(&path, format!("{check} is not a built-in check")));
    }
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
    Ok(mode)
}

/// One deny rule: a name and an ASCII-mode, case-insensitive pattern.
#[derive(Clone, Debug)]
pub struct Rule {
    pub name: String,
    pub pattern: String,
    pub regex: regex::bytes::Regex,
}

/// The deny list of one audience; `path` is `None` when it has none.
#[derive(Clone, Debug)]
pub struct Deny {
    pub audience: String,
    pub path: Option<String>,
    pub rules: Vec<Rule>,
}

/// `.writfold/deliveries/deny-<audience>.toml`: `rules = ["name: pattern",
/// …]`. A pattern is a regular expression in ASCII mode (`\b`, `\w` and
/// case folding are ASCII), matched case-insensitively, line by line.
pub fn deny(root: &Path, audience: &str) -> Result<Deny> {
    let file = format!("deny-{audience}.toml");
    let found = read(
        root,
        &file,
        "writfold-deny/1",
        &["schema", "audience", "rules"],
    )?;
    let Some((path, map)) = found else {
        return Ok(Deny {
            audience: audience.to_owned(),
            path: None,
            rules: Vec::new(),
        });
    };
    if map.get("audience") != Some(&Value::Str(audience.into())) {
        return Err(bad(&path, format!("audience must be {audience}")));
    }
    let mut rules = Vec::new();
    for entry in strings(&map, "rules", &path)? {
        let Some((name, pattern)) = entry.split_once(": ") else {
            return Err(bad(
                &path,
                format!("rule {entry:?} is not \"name: pattern\""),
            ));
        };
        if !is_name(name) {
            return Err(bad(&path, format!("rule name {name:?} must be [a-z0-9.-]")));
        }
        let regex = regex::bytes::RegexBuilder::new(pattern)
            .unicode(false)
            .case_insensitive(true)
            .size_limit(1 << 20)
            .build()
            .map_err(|e| bad(&path, format!("rule {name}: {e}")))?;
        rules.push(Rule {
            name: name.to_owned(),
            pattern: pattern.to_owned(),
            regex,
        });
    }
    Ok(Deny {
        audience: audience.to_owned(),
        path: Some(path),
        rules,
    })
}

/// One obligation of a candidate, from its mode.
#[derive(Clone, Debug)]
pub struct Obligation {
    pub name: String,
    /// `mechanical`, `observed` (depends on the cited repositories) or
    /// `attested`.
    pub class: &'static str,
    /// The digest of what the obligation means; a receipt or statement
    /// about another digest is stale.
    pub digest: String,
}

fn sha(text: &str) -> String {
    format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(text.as_bytes()))
    )
}

/// The obligations of a mode for an audience, built-in checks first.
pub fn obligations(mode: &Mode, deny: &Deny) -> Vec<Obligation> {
    let base = "writfold-delivery-check/1\n";
    let mut out = Vec::new();
    for check in &mode.checks {
        let (class, material) = match check.as_str() {
            PARAGRAPHS => ("mechanical", format!("words={}", mode.paragraph_words)),
            REFERENCES => {
                let rules: Vec<String> = deny
                    .rules
                    .iter()
                    .map(|r| format!("{}\t{}", r.name, r.pattern))
                    .collect();
                let list = match deny.path {
                    Some(_) => rules.join("\n"),
                    None => "(no deny list)".into(),
                };
                ("mechanical", format!("audience={}\n{list}", deny.audience))
            }
            _ => ("observed", String::new()),
        };
        out.push(Obligation {
            name: check.clone(),
            class,
            digest: sha(&format!("{base}{check}\n{material}")),
        });
    }
    for name in &mode.attest {
        out.push(Obligation {
            name: name.clone(),
            class: "attested",
            digest: sha(&format!("{base}attested\n{name}")),
        });
    }
    out
}

/// The guard hook's configuration: `.writfold/deliveries/guard.toml`.
#[derive(Clone, Debug)]
pub struct Guard {
    pub path: String,
    /// Publishing commands, as words (`gh pr comment`).
    pub commands: Vec<Vec<String>>,
    /// Tool names the harness calls directly (an MCP tool that posts).
    pub tools: Vec<String>,
}

/// The guard configuration, or `None` when there is none (the hook then
/// allows everything).
pub fn guard(root: &Path) -> Result<Option<Guard>> {
    let keys = ["schema", "commands", "tools"];
    let Some((path, map)) = read(root, "guard.toml", "writfold-guard/1", &keys)? else {
        return Ok(None);
    };
    let commands: Vec<Vec<String>> = strings(&map, "commands", &path)?
        .iter()
        .map(|c| c.split_whitespace().map(str::to_owned).collect::<Vec<_>>())
        .collect();
    if commands.iter().any(Vec::is_empty) {
        return Err(bad(&path, "a command must name at least one word".into()));
    }
    Ok(Some(Guard {
        tools: strings(&map, "tools", &path)?,
        commands,
        path,
    }))
}
