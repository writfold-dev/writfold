//! The checks of the delivery files ([`super::config`]): a deny list and
//! the guard hook's configuration (a mode file is [`super::mode_check`]).
//! Every fault is reported at its position.
use super::config::{Deny, Guard, Rule, is_name};
use crate::defs::{Check, Spanned, Text};
use crate::gate::def_check::schema;
use writfold_core::diag::Report;

pub(super) fn list(c: &mut Check<'_>, key: &str) -> Option<Vec<Spanned<Text>>> {
    c.list(key)
        .or(Spanned {
            value: Vec::new(),
            span: Default::default(),
        })
        .map(|l| l.value)
}

/// `audience` or `mode` must equal what the file name says.
pub(super) fn named(c: &mut Check<'_>, key: &str, want: &str, file: &str) -> bool {
    let Some(v) = c.need_text(key, &format!("{key} = \"{want}\"")) else {
        return false;
    };
    if v.str() == want {
        return true;
    }
    let why = format!("`{key}` is {:?}, but the file is named {file}", v.str());
    let d = c.error("file.name", Some(v.span), why);
    d.label("does not match the file name");
    d.help(format!("write {key} = \"{want}\""));
    false
}

fn rule(c: &mut Check<'_>, entry: &Spanned<Text>) -> Option<Rule> {
    let Some((name, pattern)) = entry.str().split_once(": ") else {
        let why = format!("rule {:?} is not \"name: pattern\"", entry.str());
        c.error("value.form", Some(entry.span), why)
            .label("no `: ` after the rule's name");
        return None;
    };
    if !is_name(name) {
        let why = format!("rule name {name:?} must be [a-z0-9.-]");
        c.error("value.form", Some(entry.sub(0, name.len())), why)
            .label("not a name");
        return None;
    }
    let built = regex::bytes::RegexBuilder::new(pattern)
        .unicode(false)
        .case_insensitive(true)
        .size_limit(1 << 20)
        .build();
    match built {
        Ok(regex) => Some(Rule {
            name: name.to_owned(),
            pattern: pattern.to_owned(),
            regex,
        }),
        Err(e) => {
            let span = entry.sub(name.len() + 2, pattern.len());
            let why = format!("rule {name}: the pattern is not a regular expression");
            let text = e.to_string();
            let last = text
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("")
                .trim();
            let d = c.error("value.pattern", Some(span), why);
            d.label("does not compile");
            d.note(last.trim_start_matches("error: ").to_owned());
            None
        }
    }
}

/// `deny-<audience>.toml`.
pub fn deny(audience: &str, path: &str, text: &str) -> Result<Deny, Report> {
    let mut c = Check::read(path, text, "a deny list");
    let keys = ["schema", "audience", "rules"];
    c.unknown_keys(&keys, &keys, "not a deny list key");
    schema(&mut c, "writfold-deny/1");
    let ok = named(
        &mut c,
        "audience",
        audience,
        &format!("deny-{audience}.toml"),
    );
    let mut rules = Some(Vec::new());
    for entry in list(&mut c, "rules").unwrap_or_default() {
        match (rule(&mut c, &entry), rules.as_mut()) {
            (Some(r), Some(all)) => all.push(r),
            _ => rules = None,
        }
    }
    let deny = rules.filter(|_| ok).map(|rules| Deny {
        audience: audience.to_owned(),
        path: Some(path.to_owned()),
        rules,
    });
    c.finish(deny)
}

/// `guard.toml`; `root` (when known) is where route modes load from.
pub fn guard(path: &str, text: &str, root: Option<&std::path::Path>) -> Result<Guard, Report> {
    let mut c = Check::read(path, text, "the delivery guard");
    let keys = ["schema", "commands", "tools", "routes"];
    c.unknown_keys(&keys, &keys, "not a delivery guard key");
    schema(&mut c, "writfold-guard/1");
    let commands = list(&mut c, "commands");
    let mut ok = true;
    for command in commands
        .iter()
        .flatten()
        .filter(|c| c.str().split_whitespace().next().is_none())
    {
        let why = "a command must name at least one word".into();
        c.error("value.form", Some(command.span), why)
            .label("empty");
        ok = false;
    }
    let tools = list(&mut c, "tools");
    let taken: Vec<String> = (commands.iter().flatten())
        .chain(tools.iter().flatten())
        .map(|w| w.str().split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    let mut routes = Some(Vec::new());
    for entry in list(&mut c, "routes").unwrap_or_default() {
        match (
            super::route::read(&mut c, &entry, &taken, root),
            routes.as_mut(),
        ) {
            (Some(r), Some(all)) => all.push(r),
            _ => routes = None,
        }
    }
    let guard = (|| {
        let commands = commands?
            .iter()
            .map(|c| c.str().split_whitespace().map(str::to_owned).collect())
            .collect();
        let tools = tools?.into_iter().map(|t| t.value.text).collect();
        let routes = routes?;
        ok.then(|| Guard {
            path: path.to_owned(),
            commands,
            tools,
            routes,
        })
    })();
    c.finish(guard)
}
