//! Plan templates: `.writfold/plans/<name>.toml`, schema `writfold-plan/1`,
//! in the strict TOML subset gate definitions use ([`crate::gate::toml`]).
//! A step is one string, `id: gate gate …`: a slug, a colon, and the IDs
//! of the gates the step owes (none is allowed). Unknown keys are refused,
//! as for gates. The digest is the SHA-256 of the template's exact bytes:
//! a run keeps a snapshot under that digest, so a later edit to the
//! template cannot rewrite the run.
//!
//! A step may also include another template, `use=<name>`: when the step
//! is spliced, the named template's steps are spliced under it as its
//! children. Includes are static: they are resolved when the template is
//! spliced ([`closure`]), every included template is snapshotted, and a
//! cycle or a nesting deeper than [`writfold_core::plan::MAX_DEPTH`] is
//! refused.
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
/// The name of the required-steps file in [`DIR`], never a template.
pub const RESERVED: &str = "required";

/// One step as a template writes it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Step {
    pub id: String,
    pub gates: Vec<String>,
    /// The template a `use=` names, spliced under this step.
    pub include: Option<String>,
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
    let mut include: Option<String> = None;
    for word in rest.split_whitespace() {
        if let Some(name) = word.strip_prefix("use=") {
            if !is_slug(name) || name == RESERVED {
                return Err(format!("step {id}: {word} does not name a template"));
            }
            if include.replace(name.to_owned()).is_some() {
                return Err(format!("step {id} includes more than one template"));
            }
            continue;
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
        include,
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
    if !is_slug(&name) || name == RESERVED || stem.is_some_and(|s| s != name) {
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
        if parsed.include.as_deref() == Some(name.as_str()) {
            return Err(bad(format!("step {} includes {name} itself", parsed.id)));
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
    if !is_slug(name) || name == RESERVED {
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

/// A template with every template it includes, directly or not, each
/// with where it was read from.
pub struct Closure {
    pub top: Template,
    pub source: String,
    /// The included templates, each once, in first-included order.
    pub includes: Vec<(Template, String)>,
    /// Levels of steps the splice adds: 1 without includes.
    pub height: usize,
}

impl Closure {
    /// The template `name` among the closure.
    pub fn get(&self, name: &str) -> Option<&Template> {
        std::iter::once(&self.top)
            .chain(self.includes.iter().map(|(t, _)| t))
            .find(|t| t.name == name)
    }

    /// Every template of the closure, the top first.
    pub fn all(&self) -> impl Iterator<Item = &Template> {
        std::iter::once(&self.top).chain(self.includes.iter().map(|(t, _)| t))
    }
}

type Repo<'a> = Option<(&'a Path, &'a str)>;

/// Resolve every `use=` of `top`, refusing a cycle and a nesting deeper
/// than the kernel's bound.
pub fn closure(kb: &Path, repo: Repo<'_>, top: Template, source: String) -> Result<Closure> {
    let mut includes: Vec<(Template, String)> = Vec::new();
    let mut stack = vec![top.name.clone()];
    let height = visit(kb, repo, &top, &mut stack, &mut includes)?;
    Ok(Closure {
        top,
        source,
        includes,
        height,
    })
}

fn visit(
    kb: &Path,
    repo: Repo<'_>,
    t: &Template,
    stack: &mut Vec<String>,
    found: &mut Vec<(Template, String)>,
) -> Result<usize> {
    let mut height = 1;
    for name in t.steps.iter().filter_map(|s| s.include.as_deref()) {
        if stack.iter().any(|s| s == name) {
            return Err(bad(format!(
                "{} includes itself: {} → {name}",
                stack[0],
                stack.join(" → ")
            )));
        }
        if stack.len() >= writfold_core::plan::MAX_DEPTH {
            return Err(bad(format!(
                "{} nests deeper than {} levels: {} → {name}",
                stack[0],
                writfold_core::plan::MAX_DEPTH,
                stack.join(" → ")
            )));
        }
        let included = match found.iter().find(|(f, _)| f.name == name) {
            Some((f, _)) => f.clone(),
            None => {
                let (f, source) = find(kb, repo, name)?;
                found.push((f.clone(), source));
                f
            }
        };
        stack.push(name.to_owned());
        let below = visit(kb, repo, &included, stack, found)?;
        stack.pop();
        height = height.max(below + 1);
    }
    Ok(height)
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
        let with = step("review: lint use=review").map_err(|e| e.to_string())?;
        assert_eq!(with.include.as_deref(), Some("review"));
        assert_eq!(with.gates, ["lint"]);
        assert!(t.steps[2].gates.is_empty());
        assert!(t.digest.starts_with("sha256:"));
        for bad in [
            "no colon",
            ": unit",
            "Bad: unit",
            "review: use=Review",
            "review: use=a use=b",
            "review: use=required",
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
            GOOD.replace("\"note:\",", "\"note: use=change\","),
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
