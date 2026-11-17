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
//! Both lists are optional. `unskippable` (optional) names start or end
//! templates whose own steps `plan skip` refuses; the lock is taken into
//! the run's pin when the plan is set ([`super::pin`]), so a later edit of
//! this file does not change a run already set. The templates are spliced, like any other, when
//! a plan is first set (`plan set`, or `plan use` on a run without a plan),
//! as root splices of their own: their steps are ordinary steps with
//! ordinary gates. A plan whose run lacks a required end template (the file
//! changed after the plan was set) cannot complete: `plan advance` refuses
//! to close its last open step until `plan use <template>` adds it.
use super::template::{self, DIR, RESERVED};
use crate::defs::{Check, Spanned, Text};
use crate::error::{KbError, Result};
use crate::write::is_slug;
use sha2::Digest;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "writfold-plan-required/1";
const KEYS: [&str; 4] = ["schema", "start", "end", "unskippable"];

/// The required templates, by name, and where the file was read from.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Required {
    pub start: Vec<String>,
    pub end: Vec<String>,
    /// Required templates whose steps `plan skip` refuses.
    pub unskippable: Vec<String>,
    pub source: Option<String>,
    /// `sha256:<hex>` of the file's bytes, when there is a file.
    pub digest: Option<String>,
}

fn bad(why: String) -> KbError {
    KbError::Refused(format!("{DIR}/{RESERVED}.toml: {why}"))
}

/// One list of template names, each a slug once.
fn names(c: &mut Check<'_>, key: &str) -> Option<Vec<Spanned<Text>>> {
    let items = c
        .list(key)
        .or(Spanned {
            value: Vec::new(),
            span: Default::default(),
        })?
        .value;
    let mut ok = true;
    for (i, item) in items.iter().enumerate() {
        if !is_slug(item.str()) || item.str() == RESERVED {
            let why = format!("`{key}`: {:?} is not a template name", item.str());
            c.error("value.form", Some(item.span), why)
                .label("not a template name");
            ok = false;
        } else if items[..i].iter().any(|x| x.str() == item.str()) {
            let why = format!("`{key}` names {} twice", item.str());
            c.error("value.twice", Some(item.span), why)
                .label("named again");
            ok = false;
        }
    }
    ok.then_some(items)
}

/// `unskippable`: templates also named in `start` or `end`.
fn unskippable(
    c: &mut Check<'_>,
    start: Option<&[Spanned<Text>]>,
    end: Option<&[Spanned<Text>]>,
) -> Option<Vec<String>> {
    let items = names(c, "unskippable")?;
    let known: Vec<&str> = (start.into_iter().flatten())
        .chain(end.into_iter().flatten())
        .map(|i| i.str())
        .collect();
    let mut ok = true;
    for item in &items {
        if !known.contains(&item.str()) {
            let why = format!(
                "`unskippable` names {}, which is not a start or end template",
                item.str()
            );
            let label = "not in start or end";
            c.unknown_value(item.span, why, label, item.str(), &known, '"');
            c.sink
                .found
                .last_mut()
                .into_iter()
                .for_each(|d| d.code = "ref.undefined");
            ok = false;
        }
    }
    ok.then(|| items.iter().map(|i| i.value.text.clone()).collect())
}

/// Parse the file's bytes, shown as `shown`; every fault is reported.
pub fn parse_shown(bytes: &[u8], shown: &str) -> Result<(Required, Vec<Spanned<Text>>)> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        let problem = crate::defs::Problem::whole(
            shown,
            shown,
            "file.unreadable",
            "is not UTF-8 text".into(),
        );
        return Err(KbError::Definition(Box::new(problem.report)));
    };
    let mut c = Check::read(shown, text, "the required steps");
    c.unknown_keys(&KEYS, &KEYS, "not a required-steps key");
    crate::gate::def_check::schema(&mut c, SCHEMA);
    let (start, end) = (names(&mut c, "start"), names(&mut c, "end"));
    if let (Some(start), Some(end)) = (&start, &end) {
        for both in end
            .iter()
            .filter(|e| start.iter().any(|s| s.str() == e.str()))
        {
            let why = format!("{} is both a start and an end template", both.str());
            c.error("value.twice", Some(both.span), why)
                .label("also a start template");
        }
    }
    let locked = unskippable(&mut c, start.as_deref(), end.as_deref());
    let parts = (|| Some((start?, end?, locked?)))();
    let (start, end, locked) = c
        .finish(parts)
        .map_err(|r| KbError::Definition(Box::new(r)))?;
    let plain = |items: &[Spanned<Text>]| items.iter().map(|i| i.value.text.clone()).collect();
    let required = Required {
        start: plain(&start),
        end: plain(&end),
        unskippable: locked,
        source: None,
        digest: None,
    };
    Ok((required, start.into_iter().chain(end).collect()))
}

/// Parse the file's bytes.
pub fn parse(bytes: &[u8]) -> Result<Required> {
    parse_shown(bytes, &format!("{DIR}/{RESERVED}.toml")).map(|(r, _)| r)
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
        [(path, source)] => {
            let bytes = std::fs::read(path)?;
            let hex = crate::index::hex(&sha2::Sha256::digest(&bytes));
            Ok(Required {
                source: Some(source.clone()),
                digest: Some(format!("sha256:{hex}")),
                ..parse_shown(&bytes, source)?.0
            })
        }
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

/// The required templates (start or end) no root splice of `graph` holds:
/// the file gained them after the plan was set.
pub fn missing_end(required: &Required, graph: &super::fold::Graph) -> Vec<String> {
    (required.start.iter())
        .chain(&required.end)
        .filter(|name| {
            !graph
                .uses
                .iter()
                .any(|u| u.parent.is_empty() && &u.template == *name)
        })
        .cloned()
        .collect()
}

/// Every template a required list names must exist now; one that does
/// not is reported at its name in the file.
pub fn check(kb: &Path, repo: Option<(&Path, &str)>, required: &Required) -> Result<()> {
    for name in required.start.iter().chain(&required.end) {
        let Err(e) = template::find(kb, repo, name) else {
            continue;
        };
        let missing = matches!(&e, KbError::Refused(why) if why.starts_with("no plan template"));
        let source = required.source.clone().unwrap_or_default();
        let file = match source.split_once(':') {
            Some((_, f)) if repo.is_some() => repo.map(|(top, _)| top.join(f)),
            _ => Some(kb.join(&source)),
        };
        let bytes = file.and_then(|f| std::fs::read(f).ok());
        let Some((_, items)) = bytes
            .as_deref()
            .and_then(|b| parse_shown(b, &source).ok())
            .filter(|_| missing)
        else {
            return Err(e);
        };
        let text = String::from_utf8_lossy(bytes.as_deref().unwrap_or_default()).into_owned();
        let mut sink = crate::defs::Sink::new(&source, &text);
        let known = super::refs::names(kb, repo.map(|r| r.0));
        let known: Vec<&str> = known.iter().map(String::as_str).collect();
        let span = items.iter().find(|i| i.str() == name).map(|i| i.span);
        let d = sink.error(
            "ref.undefined",
            span,
            format!("template `{name}` is not defined"),
        );
        d.label(&format!(
            "no {DIR}/{name}.toml in the KB root or the repository"
        ));
        d.help = writfold_core::diag::suggest::did_you_mean(
            &writfold_core::diag::suggest(name, &known),
            '`',
        );
        return Err(KbError::Definition(Box::new(sink.report())));
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
