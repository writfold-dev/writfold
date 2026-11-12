//! Required steps: `.writfold/plans/required.toml`, schema
//! `writfold-plan-required/1`, in the same TOML subset as templates. It
//! names templates every run's plan includes:
//!
//! ```toml
//! schema = "writfold-plan-required/1"
//! start = ["intake"]     # spliced at the root before the plan's own steps
//! end = ["reflect"]      # spliced at the root after them
//! ```
//!
//! Both lists are optional. The templates are spliced, like any other, when
//! a plan is first set (`plan set`, or `plan use` on a run without a plan),
//! as root splices of their own: their steps are ordinary steps with
//! ordinary gates. A plan whose run lacks a required end template (the file
//! changed after the plan was set) cannot complete: `plan advance` refuses
//! to close its last open step until `plan use <template>` adds it.
use super::template::{self, DIR, RESERVED};
use crate::error::{KbError, Result};
use crate::gate::toml::{self, Value};
use crate::write::is_slug;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "writfold-plan-required/1";
const KEYS: [&str; 3] = ["schema", "start", "end"];

/// The required templates, by name, and where the file was read from.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Required {
    pub start: Vec<String>,
    pub end: Vec<String>,
    pub source: Option<String>,
}

fn bad(why: String) -> KbError {
    KbError::Refused(format!("{DIR}/{RESERVED}.toml: {why}"))
}

/// Parse the file's bytes.
pub fn parse(bytes: &[u8]) -> Result<Required> {
    let text = std::str::from_utf8(bytes).map_err(|_| bad("not UTF-8".into()))?;
    let map = toml::parse(text).map_err(bad)?;
    if let Some(key) = map.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(bad(format!("unknown key {key}")));
    }
    if map.get("schema") != Some(&Value::Str(SCHEMA.into())) {
        return Err(bad(format!("schema must be {SCHEMA}")));
    }
    let list = |key: &str| -> Result<Vec<String>> {
        let items = match map.get(key) {
            None => return Ok(Vec::new()),
            Some(Value::List(items)) => items.clone(),
            Some(_) => return Err(bad(format!("{key} must be an array of template names"))),
        };
        for (i, name) in items.iter().enumerate() {
            if !is_slug(name) || name == RESERVED {
                return Err(bad(format!("{key}: {name:?} is not a template name")));
            }
            if items[..i].contains(name) {
                return Err(bad(format!("{key} names {name} twice")));
            }
        }
        Ok(items)
    };
    let (start, end) = (list("start")?, list("end")?);
    if let Some(both) = start.iter().find(|n| end.contains(n)) {
        return Err(bad(format!("{both} is both a start and an end template")));
    }
    Ok(Required {
        start,
        end,
        source: None,
    })
}

/// The required steps from the KB root's `.writfold/plans/required.toml`,
/// else the subject repository's (the same file in both is refused, as
/// for templates); none when neither exists.
pub fn find(kb: &Path, repo: Option<(&Path, &str)>) -> Result<Required> {
    let file = format!("{DIR}/{RESERVED}.toml");
    let mut roots: Vec<(PathBuf, String)> = vec![(kb.join(&file), file.clone())];
    if let Some((top, name)) = repo {
        let path = top.join(&file);
        let same = matches!((std::fs::canonicalize(&roots[0].0), std::fs::canonicalize(&path)), (Ok(a), Ok(b)) if a == b);
        if !same {
            roots.push((path, format!("{name}:{file}")));
        }
    }
    let found: Vec<&(PathBuf, String)> = roots.iter().filter(|(p, _)| p.is_file()).collect();
    match found.as_slice() {
        [] => Ok(Required::default()),
        [(path, source)] => Ok(Required {
            source: Some(source.clone()),
            ..parse(&std::fs::read(path)?)?
        }),
        _ => Err(bad("defined in both the KB and the repository".into())),
    }
}

/// The templates a new plan splices at the root, in order: the start
/// templates, `main`, then the end templates (`main` once, wherever the
/// file names it).
pub fn order(required: &Required, main: &str) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for name in &required.start {
        if name != main {
            out.push((name.clone(), true));
        }
    }
    out.push((main.to_owned(), false));
    for name in &required.end {
        if name != main {
            out.push((name.clone(), true));
        }
    }
    out
}

/// The required end templates no root splice of `graph` holds.
pub fn missing_end(required: &Required, graph: &super::fold::Graph) -> Vec<String> {
    required
        .end
        .iter()
        .filter(|name| {
            !graph
                .uses
                .iter()
                .any(|u| u.parent.is_empty() && &u.template == *name)
        })
        .cloned()
        .collect()
}

/// Every template a required list names must exist now.
pub fn check(kb: &Path, repo: Option<(&Path, &str)>, required: &Required) -> Result<()> {
    for name in required.start.iter().chain(&required.end) {
        template::find(kb, repo, name)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{order, parse};

    #[test]
    fn the_file_parses_and_refuses() -> Result<(), String> {
        let good =
            "schema = \"writfold-plan-required/1\"\nstart = [\"intake\"]\nend = [\"reflect\"]\n";
        let r = parse(good.as_bytes()).map_err(|e| e.to_string())?;
        assert_eq!(
            (r.start.as_slice(), r.end.as_slice()),
            (&["intake".to_owned()][..], &["reflect".to_owned()][..])
        );
        let names: Vec<String> = order(&r, "change").into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, ["intake", "change", "reflect"]);
        let names: Vec<String> = order(&r, "reflect").into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, ["intake", "reflect"]);
        for bad in [
            good.replace("required/1", "required/2"),
            format!("{good}at = \"start\"\n"),
            good.replace("intake", "Intake"),
            good.replace("intake", "reflect"),
            good.replace("\"intake\"", "\"intake\", \"intake\""),
            good.replace("[\"intake\"]", "\"intake\""),
            good.replace("intake", "required"),
        ] {
            assert!(parse(bad.as_bytes()).is_err(), "{bad}");
        }
        Ok(())
    }
}
