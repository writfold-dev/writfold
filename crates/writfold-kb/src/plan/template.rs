//! Plan templates: `.writfold/plans/<name>.toml`, schema `writfold-plan/1`,
//! in the strict TOML subset gate definitions use ([`crate::defs::toml`]).
//! A step is one string, `id: gate gate …`: a slug, a colon, and the IDs
//! of the gates the step owes (none is allowed); `deliver:<mode>` owes a delivery. Unknown keys are refused,
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
use crate::write::is_slug;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "writfold-plan/1";
/// Where templates live, under the KB root or the subject repository.
pub const DIR: &str = ".writfold/plans";
pub const KEYS: [&str; 4] = ["schema", "name", "description", "steps"];
/// At most this many steps in one template.
pub const MAX_STEPS: usize = 64;
/// The name of the required-steps file in [`DIR`], never a template.
pub const RESERVED: &str = "required";

/// One step as a template writes it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Step {
    pub id: String,
    pub gates: Vec<String>,
    /// The template a `use=` names, spliced under this step; for a loop,
    /// its body (`loop=`).
    pub include: Option<String>,
    /// A loop's bounds (`loop=` with `max=` and `min=`, #76).
    pub repeat: Option<Repeat>,
}

/// A loop step's bounds: at least `min` rounds of its body, at most `max`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Repeat {
    pub min: usize,
    pub max: usize,
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

/// One `id: gate gate …` string ([`super::step_word`]).
pub fn step(text: &str) -> std::result::Result<Step, String> {
    super::step_word::read(text).map_err(|f| f.why)
}

/// Parse a template's bytes; `stem`, when given, is the file name the
/// `name` must equal.
pub fn parse(bytes: &[u8], stem: Option<&str>) -> Result<Template> {
    let shown = format!("{DIR}/{}.toml", stem.unwrap_or("template"));
    parse_shown(bytes, stem, &shown)
}

/// [`parse`], its diagnostics shown as `shown`.
pub fn parse_shown(bytes: &[u8], stem: Option<&str>, shown: &str) -> Result<Template> {
    let text = std::str::from_utf8(bytes).map_err(|_| unreadable(shown))?;
    let (name, description, steps) =
        check(text, stem, shown).map_err(|r| KbError::Definition(Box::new(r)))?;
    Ok(Template {
        name,
        description,
        steps,
        digest: digest(bytes),
        bytes: bytes.to_vec(),
    })
}

fn unreadable(shown: &str) -> KbError {
    let problem =
        crate::defs::Problem::whole(shown, shown, "file.unreadable", "is not UTF-8 text".into());
    KbError::Definition(Box::new(problem.report))
}

pub use super::template_check::check;

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
        [(path, source)] => Ok((
            parse_shown(&std::fs::read(path)?, Some(name), source)?,
            source.clone(),
        )),
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

    /// Every template with where it was read from, the top first.
    pub fn sourced(&self) -> impl Iterator<Item = (&Template, &str)> {
        std::iter::once((&self.top, self.source.as_str()))
            .chain(self.includes.iter().map(|(t, s)| (t, s.as_str())))
    }
}

type Repo<'a> = Option<(&'a Path, &'a str)>;

/// Resolve every `use=` of `top`, refusing a cycle and a nesting deeper
/// than the kernel's bound, each at the `use=` word that makes it.
pub fn closure(kb: &Path, repo: Repo<'_>, top: Template, source: String) -> Result<Closure> {
    let mut includes: Vec<(Template, String)> = Vec::new();
    let mut stack = vec![top.name.clone()];
    let height = visit(kb, repo, (&top, &source), &mut stack, &mut includes)?;
    Ok(Closure {
        top,
        source,
        includes,
        height,
    })
}

fn cycle(t: (&Template, &str), step: &str, name: &str, why: String) -> KbError {
    let report = super::refs::at_word(t.0, t.1, (step, &format!("use={name}")), "ref.cycle", why);
    KbError::Definition(Box::new(report))
}

fn visit(
    kb: &Path,
    repo: Repo<'_>,
    t: (&Template, &str),
    stack: &mut Vec<String>,
    found: &mut Vec<(Template, String)>,
) -> Result<usize> {
    let mut height = 1;
    for (step, name) in
        t.0.steps
            .iter()
            .filter_map(|s| s.include.as_deref().map(|n| (s.id.as_str(), n)))
    {
        let path = format!("{} → {name}", stack.join(" → "));
        if stack.iter().any(|s| s == name) {
            return Err(cycle(
                t,
                step,
                name,
                format!("{} includes itself: {path}", stack[0]),
            ));
        }
        if stack.len() >= writfold_core::plan::MAX_DEPTH {
            let why = format!(
                "{} nests deeper than {} levels: {path}",
                stack[0],
                writfold_core::plan::MAX_DEPTH
            );
            return Err(cycle(t, step, name, why));
        }
        let (included, source) = match found.iter().find(|(f, _)| f.name == name) {
            Some((f, s)) => (f.clone(), s.clone()),
            None => {
                let (f, source) = find(kb, repo, name).map_err(|e| match e {
                    KbError::Refused(why) if why.starts_with("no plan template") => {
                        super::refs::no_template(kb, repo.map(|r| r.0), t, step, name)
                    }
                    other => other,
                })?;
                found.push((f.clone(), source.clone()));
                (f, source)
            }
        };
        stack.push(name.to_owned());
        let below = visit(kb, repo, (&included, &source), stack, found)?;
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
