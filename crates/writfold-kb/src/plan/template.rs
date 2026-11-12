//! Plan templates: `.writfold/plans/<name>.toml`, schema `writfold-plan/1`,
//! in the strict TOML subset gate definitions use ([`crate::gate::toml`]).
//! A step is one string, `id: gate gate …`: a slug, a colon, and the IDs
//! of the gates the step owes (none is allowed). Unknown keys are refused,
//! as for gates. The digest is the SHA-256 of the template's exact bytes:
//! a run keeps a snapshot under that digest, so a later edit to the
//! template cannot rewrite the run.
//!
//! Not built yet (refused, with the reason): `use=<template>` includes,
//! which splice another template's steps (the next slice).
use crate::error::{KbError, Result};
use crate::gate::toml::{self, Value};
use crate::write::is_slug;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "writfold-plan/1";
/// Where templates live, under the KB root or the subject repository.
pub const DIR: &str = ".writfold/plans";
const KEYS: [&str; 4] = ["schema", "name", "description", "steps"];
/// At most this many steps in one template.
pub const MAX_STEPS: usize = 64;

/// One step as a template writes it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Step {
    pub id: String,
    pub gates: Vec<String>,
}

/// One parsed template.
#[derive(Clone, Debug)]
pub struct Template {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<Step>,
    /// `sha256:<hex>` of the exact bytes.
    pub digest: String,
    pub bytes: Vec<u8>,
}

fn bad(why: String) -> KbError {
    KbError::Refused(format!("plan template: {why}"))
}

/// `sha256:<hex>` of a template's bytes.
pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", crate::index::hex(&Sha256::digest(bytes)))
}

/// One `id: gate gate …` string.
pub fn step(text: &str) -> std::result::Result<Step, String> {
    let Some((id, rest)) = text.split_once(':') else {
        return Err(format!("step {text:?} must be written \"id: gate gate …\""));
    };
    let id = id.trim();
    if !is_slug(id) {
        return Err(format!("step ID {id:?} is not a slug"));
    }
    let mut gates: Vec<String> = Vec::new();
    for word in rest.split_whitespace() {
        if word.starts_with("use=") {
            return Err(format!(
                "step {id}: {word} includes another template; splicing is not built yet"
            ));
        }
        if !is_slug(word) {
            return Err(format!("step {id}: gate {word:?} is not a gate ID"));
        }
        if gates.iter().any(|g| g == word) {
            return Err(format!("step {id} names gate {word} twice"));
        }
        gates.push(word.to_owned());
    }
    Ok(Step {
        id: id.to_owned(),
        gates,
    })
}

/// Parse a template's bytes; `stem`, when given, is the file name the
/// `name` must equal.
pub fn parse(bytes: &[u8], stem: Option<&str>) -> Result<Template> {
    let text = std::str::from_utf8(bytes).map_err(|_| bad("not UTF-8".into()))?;
    let map = toml::parse(text).map_err(bad)?;
    if let Some(key) = map.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(bad(format!("unknown key {key}")));
    }
    let string = |key: &str| match map.get(key) {
        None => Ok(None),
        Some(Value::Str(s)) => Ok(Some(s.clone())),
        Some(_) => Err(bad(format!("{key} must be a string"))),
    };
    if string("schema")?.as_deref() != Some(SCHEMA) {
        return Err(bad(format!("schema must be {SCHEMA}")));
    }
    let name = string("name")?.ok_or_else(|| bad("name is required".into()))?;
    if !is_slug(&name) || stem.is_some_and(|s| s != name) {
        return Err(bad(format!(
            "name {name:?} must be a slug equal to the file name"
        )));
    }
    let description = string("description")?;
    if description
        .as_deref()
        .is_some_and(|d| d.contains(['\n', '\r']))
    {
        return Err(bad("description must be one line".into()));
    }
    let Some(Value::List(items)) = map.get("steps") else {
        return Err(bad("steps must be an array of strings".into()));
    };
    if items.is_empty() || items.len() > MAX_STEPS {
        return Err(bad(format!("steps must hold 1 to {MAX_STEPS} steps")));
    }
    let mut steps: Vec<Step> = Vec::new();
    for item in items {
        let parsed = step(item).map_err(bad)?;
        if steps.iter().any(|s| s.id == parsed.id) {
            return Err(bad(format!("step {} is listed twice", parsed.id)));
        }
        steps.push(parsed);
    }
    Ok(Template {
        name,
        description,
        steps,
        digest: digest(bytes),
        bytes: bytes.to_vec(),
    })
}

/// The template `name` from the KB root's `.writfold/plans/`, else the
/// subject repository's (`repo`: its top and its name); the same name in
/// both is refused. Returns the template and where it was read from.
pub fn find(kb: &Path, repo: Option<(&Path, &str)>, name: &str) -> Result<(Template, String)> {
    if !is_slug(name) {
        return Err(bad(format!("{name:?} is not a template name")));
    }
    let file = format!("{DIR}/{name}.toml");
    let mut roots: Vec<(PathBuf, String)> = vec![(kb.join(&file), file.clone())];
    if let Some((top, repo)) = repo {
        let path = top.join(&file);
        let same = matches!((std::fs::canonicalize(&roots[0].0), std::fs::canonicalize(&path)), (Ok(a), Ok(b)) if a == b);
        if !same {
            roots.push((path, format!("{repo}:{file}")));
        }
    }
    let found: Vec<&(PathBuf, String)> = roots.iter().filter(|(p, _)| p.is_file()).collect();
    match found.as_slice() {
        [] => Err(KbError::Refused(format!(
            "no plan template {name}: add {file} to the KB root or the repository"
        ))),
        [(path, source)] => Ok((parse(&std::fs::read(path)?, Some(name))?, source.clone())),
        _ => Err(bad(format!(
            "{name} is defined in both the KB and the repository"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse, step};

    const GOOD: &str = "schema = \"writfold-plan/1\"\nname = \"change\"\nsteps = [\n  \"research: backfilled\",\n  \"implement: quality unit\",\n  \"note:\",\n]\n";

    #[test]
    fn steps_parse_and_refuse() -> Result<(), String> {
        let t = parse(GOOD.as_bytes(), Some("change")).map_err(|e| e.to_string())?;
        assert_eq!(t.steps.len(), 3);
        assert_eq!(t.steps[1].gates, ["quality", "unit"]);
        assert!(t.steps[2].gates.is_empty());
        assert!(t.digest.starts_with("sha256:"));
        for bad in [
            "no colon",
            ": unit",
            "Bad: unit",
            "review: use=review",
            "x: unit unit",
            "x: Unit",
        ] {
            assert!(step(bad).is_err(), "{bad}");
        }
        for bad in [
            GOOD.replace("change\"", "other\""),
            GOOD.replace("plan/1", "plan/2"),
            format!("{GOOD}colour = \"blue\"\n"),
            GOOD.replace("\"note:\",", "\"note:\", \"note: unit\","),
            "schema = \"writfold-plan/1\"\nname = \"change\"\nsteps = []\n".into(),
        ] {
            assert!(parse(bad.as_bytes(), Some("change")).is_err(), "{bad}");
        }
        // Comments and layout are part of the bytes: the digest is of the file.
        let commented = format!("# a comment\n{GOOD}");
        let other = parse(commented.as_bytes(), None).map_err(|e| e.to_string())?;
        assert_ne!(other.digest, t.digest);
        Ok(())
    }
}
