//! `items_from = "<gate>/<file>"` on a generated checklist (#165): the
//! scope gate must be defined, be a mechanical gate and `retain` that file,
//! or no universe can ever be present. A scope gate whose `scope` is
//! narrower than `**` is the author's claim that no related record lives
//! outside it: a record added there leaves the scope current. That is
//! advice, never an error. Tested, not proved.
use super::lint::{File, Kind};
use super::{Sink, Slot, toml};
use crate::gate::def::{Definition, Origin};
use writfold_core::diag::{Report, Severity, Span};

fn parsed(f: &File) -> Option<Definition> {
    crate::gate::def_check::check(&f.text, &f.stem, f.origin, &f.dir, &f.shown).ok()
}

/// Where `items_from` is written.
fn span(text: &str) -> Option<Span> {
    let doc = toml::read(text, &mut Sink::new("", text));
    match doc.get("items_from").map(|e| &e.value) {
        Some(Slot::Value(v)) => Some(v.span),
        _ => None,
    }
}

/// What is wrong with the scope gate `gate` for file `file`, if anything:
/// the code, why, and whether it is only advice.
fn fault(
    all: &[File],
    gate: &str,
    file: &str,
    repo: bool,
    origin: Origin,
) -> Option<(&'static str, String, Severity)> {
    let Some(g) = (all.iter()).find(|g| g.kind == Kind::Gate && g.stem == gate) else {
        let severity = match origin == Origin::Kb && !repo {
            true => Severity::Warning,
            false => Severity::Error,
        };
        let why = format!("items_from names gate `{gate}`, which is not defined");
        return Some(("ref.undefined", why, severity));
    };
    let d = parsed(g)?;
    if d.class != "mechanical" {
        let why = format!(
            "items_from names gate `{gate}`, of class {}; a scope is kept only by a mechanical gate's receipt",
            d.class
        );
        return Some(("ref.wrong-kind", why, Severity::Error));
    }
    if !d.retain.iter().any(|r| r == file) {
        let why =
            format!("items_from names {gate}/{file}, but gate `{gate}` does not retain {file}");
        return Some(("ref.wrong-kind", why, Severity::Error));
    }
    if !d.scope.iter().any(|p| p == "**") {
        let why = format!(
            "the scope gate `{gate}` notices changes only in {}: a related record added elsewhere leaves the scope current",
            d.scope.join(", ")
        );
        return Some(("policy.narrow-scope", why, Severity::Warning));
    }
    None
}

/// Every `items_from` fault among `all`.
pub fn check(all: &[File], repo: bool) -> Report {
    let mut report = Report::default();
    for f in all.iter().filter(|f| f.kind == Kind::Gate) {
        let Some(from) = parsed(f).and_then(|d| d.checklist.and_then(|s| s.generated)) else {
            continue;
        };
        let Some((code, why, severity)) = fault(all, &from.gate, &from.file, repo, f.origin) else {
            continue;
        };
        let mut sink = Sink::new(&f.shown, &f.text);
        let d = sink.push(severity, code, span(&f.text), why);
        d.label = "the scope gate".to_owned();
        report.extend(sink.report());
    }
    report
}
