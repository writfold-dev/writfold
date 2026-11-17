//! The keys of a delivery mode file read one by one ([`super::mode_check`]):
//! its built-in `checks`, its one-line `description`, `waivable`, and the
//! closed `rendering` and `audience`; each fault at its position.
use super::config::BUILTIN;
use crate::defs::{Check, Field, Spanned, Text};
use crate::gate::def;
use writfold_core::diag::suggest::{did_you_mean, suggest};

/// `checks`: `None` when absent (the mode's defaults); each name must be a
/// built-in check.
pub(super) fn checks(c: &mut Check<'_>) -> Option<Option<Vec<Spanned<Text>>>> {
    let checks = match c.list("checks") {
        Field::Absent => None,
        other => Some(other.got().map(|l| l.value)),
    };
    for check in checks
        .iter()
        .flatten()
        .flatten()
        .filter(|x| !BUILTIN.contains(&x.str()))
    {
        let why = format!(
            "{} is not a built-in check; list a gate under gates",
            check.str()
        );
        c.unknown_value(
            check.span,
            why,
            "not a built-in check",
            check.str(),
            &BUILTIN,
            '"',
        );
    }
    checks
}

/// `description`: one line; `Some(None)` when absent, `None` on a fault.
pub(super) fn description(c: &mut Check<'_>) -> Option<Option<String>> {
    match c.text("description") {
        Field::Got(d) if d.str().is_empty() || d.str().contains(['\n', '\r']) => {
            c.error(
                "value.form",
                Some(d.span),
                "description must be one line".into(),
            )
            .label("empty or several lines");
            None
        }
        other => other.got().map(|d| Some(d.value.text)).or(Some(None)),
    }
}

/// `waivable`: names of the mode's built-in checks and attest names; a mode
/// gate carries its own `accept`. `Some(None)` when absent, `None` on a
/// fault.
pub(super) fn waivable(
    c: &mut Check<'_>,
    declared: &[String],
    gates: &[&str],
    text: &str,
) -> Option<Option<(Vec<String>, u32)>> {
    let list = match c.list("waivable") {
        Field::Absent => return Some(None),
        Field::Bad => return None,
        Field::Got(l) => l.value,
    };
    let names: Vec<&str> = declared.iter().map(String::as_str).collect();
    let mut ok = true;
    for item in &list {
        if gates.contains(&item.str()) {
            let why = format!(
                "`{}` is a mode gate: set accept in {}/{}.toml",
                item.str(),
                def::DIR,
                item.str()
            );
            c.error("value.conflict", Some(item.span), why)
                .label("a mode gate");
            ok = false;
        } else if !names.contains(&item.str()) {
            let why = format!("`{}` is not an obligation of this mode", item.str());
            let d = c.error("ref.undefined", Some(item.span), why);
            d.label("not in checks or attest");
            d.help = did_you_mean(&suggest(item.str(), &names), '"');
            ok = false;
        }
    }
    let line = super::mode::key_line(text, "waivable");
    ok.then(|| Some((list.into_iter().map(|w| w.value.text).collect(), line)))
}

/// A text key from a closed set: `Some(None)` when absent, `None` on a
/// fault.
pub(super) fn closed(c: &mut Check<'_>, key: &str, set: &[&str]) -> Option<Option<String>> {
    match c.text(key) {
        Field::Absent => Some(None),
        Field::Bad => None,
        Field::Got(v) if set.contains(&v.str()) => Some(Some(v.value.text)),
        Field::Got(v) => {
            let why = format!("`{key}` is {:?}, not one of {}", v.str(), set.join(", "));
            c.unknown_value(v.span, why, "not known", v.str(), set, '"');
            None
        }
    }
}
