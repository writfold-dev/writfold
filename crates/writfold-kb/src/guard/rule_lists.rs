//! The array keys of a guard rule ([`super::rule_check`]): `require`,
//! `commands`, `repos`, and arrays that may be absent.
use super::rules::Require;
use crate::defs::{Check, Spanned, Text};
use writfold_core::diag::suggest::{did_you_mean, suggest};

/// The requirement names without `gate:<id>`, for suggestions.
const NAMED: [&str; 3] = ["run.current", "gates.met", "step.met"];

pub fn strings(c: &mut Check<'_>, key: &str) -> Option<Vec<Spanned<Text>>> {
    c.list(key)
        .or(Spanned {
            value: Vec::new(),
            span: Default::default(),
        })
        .map(|l| l.value)
}

/// The `require` key: one or more requirements, each once.
pub fn requirements(c: &mut Check<'_>) -> Option<Vec<Require>> {
    let names = c.need_list("require", "a guard rule", "require = [\"gates.met\"]")?;
    let mut out = Vec::new();
    let mut ok = true;
    for (n, name) in names.iter().enumerate() {
        if names[..n].iter().any(|x| x.str() == name.str()) {
            let why = format!("`require` names {} twice", name.str());
            c.error("value.twice", Some(name.span), why)
                .label("named again");
            ok = false;
            continue;
        }
        match Require::parse(name.str()) {
            Some(r) => out.push(r),
            None => {
                let found = suggest(name.str(), &NAMED);
                let why = format!("unknown requirement {:?}", name.str());
                let d = c.error("value.unknown", Some(name.span), why);
                d.label("not a requirement");
                if let Some(help) = did_you_mean(&found, '"') {
                    d.help(help);
                }
                d.note("a rule may require run.current, gates.met, step.met or gate:<id>".into());
                ok = false;
            }
        }
    }
    ok.then_some(out)
}

pub fn commands(c: &mut Check<'_>) -> Option<Vec<Vec<String>>> {
    let commands = strings(c, "commands")?;
    let mut ok = true;
    for command in &commands {
        if command.str().split_whitespace().next().is_none() {
            let why = "a command must name at least one word".into();
            c.error("value.form", Some(command.span), why)
                .label("empty");
            ok = false;
        }
    }
    ok.then(|| {
        commands
            .iter()
            .map(|c| c.str().split_whitespace().map(str::to_owned).collect())
            .collect()
    })
}

pub fn repos(c: &mut Check<'_>) -> Option<Vec<String>> {
    let repos = strings(c, "repos")?;
    let mut ok = true;
    for r in &repos {
        let bare = r.str().strip_prefix('!').unwrap_or(r.str());
        if bare.is_empty() || bare.contains(['/', '\\']) {
            let why = format!("repos pattern {:?} is not a repository name", r.str());
            c.error("value.form", Some(r.span), why)
                .label("not a repository name");
            ok = false;
        }
    }
    ok.then(|| repos.into_iter().map(|r| r.value.text).collect())
}
