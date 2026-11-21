//! References from plan templates to other files: the gates a step owes
//! and the templates `use=` and `required.toml` include. A name that is
//! not defined is reported at the word that names it, with the closest
//! defined name; a gate file that exists but does not parse is reported
//! with its own faults, not a second error.
use super::template::{Closure, DIR, RESERVED, Template};
use crate::defs::{Sink, Slot, Value, toml};
use crate::error::{KbError, Result};
use crate::gate::def;
use std::path::Path;
use writfold_core::diag::suggest::{did_you_mean, suggest};
use writfold_core::diag::{Report, Span};

/// Where `word` is written in step `step` of a template's text.
pub fn span_of(text: &str, step: &str, word: &str) -> Option<Span> {
    let doc = toml::read(text, &mut Sink::new("", text));
    let Some(Slot::Value(v)) = doc.get("steps").map(|e| &e.value) else {
        return None;
    };
    let Value::List(items) = &v.value else {
        return None;
    };
    let item = items.iter().find(|i| {
        i.str()
            .split_once(':')
            .is_some_and(|(id, _)| id.trim() == step)
    })?;
    let (at, len) = super::step_word::locate(item.str(), word)?;
    Some(item.sub(at, len))
}

/// The template names of the KB root and the repository.
pub fn names(kb: &Path, repo: Option<&Path>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for root in std::iter::once(kb).chain(repo) {
        // A plans directory that cannot be listed is reported by lint and
        // `kb policy` (#63); the names that were read are still offered.
        let (_, names, _) =
            crate::gate::discover::classify(&root.join(DIR), crate::gate::discover::list);
        for name in names {
            let stem = name.trim_end_matches(".toml");
            if name.ends_with(".toml") && stem != RESERVED && !out.iter().any(|o| o == stem) {
                out.push(stem.to_owned());
            }
        }
    }
    out.sort();
    out
}

/// A `ref.*` diagnostic at `word` of step `step` in `t` (shown as
/// `source`), or about the file when the word cannot be found.
pub fn at_word(
    t: &Template,
    source: &str,
    (step, word): (&str, &str),
    code: &'static str,
    why: String,
) -> Report {
    let text = String::from_utf8_lossy(&t.bytes).into_owned();
    let mut sink = Sink::new(source, &text);
    let span = span_of(&text, step, word);
    sink.error(code, span, why);
    sink.report()
}

fn with(mut report: Report, label: &str, help: Option<String>) -> Report {
    if let Some(d) = report.diagnostics.first_mut() {
        d.label = label.to_owned();
        d.help = help;
    }
    report
}

/// A `use=<name>` that names no template.
pub fn no_template(
    kb: &Path,
    repo: Option<&Path>,
    (t, source): (&Template, &str),
    step: &str,
    name: &str,
) -> KbError {
    let known = names(kb, repo);
    let known: Vec<&str> = known.iter().map(String::as_str).collect();
    let why = format!("step `{step}` includes template `{name}`, which is not defined");
    let report = at_word(
        t,
        source,
        (step, &format!("use={name}")),
        "ref.undefined",
        why,
    );
    let label = format!("no {DIR}/{name}.toml in the KB root or the repository");
    let help = did_you_mean(&suggest(name, &known), '`');
    KbError::Definition(Box::new(with(report, &label, help)))
}

/// Every gate a step of the closure owes must be defined and judge the
/// repository; every delivery must name a usable mode.
pub fn gates(kb: &Path, top: Option<&Path>, closure: &Closure) -> Result<()> {
    let (found, problems) = def::all(kb, top);
    let ids: Vec<&str> = found
        .iter()
        .filter(|d| !d.on_candidate())
        .map(|d| d.id.as_str())
        .collect();
    let mut report = Report::default();
    for (t, source) in closure.sourced() {
        for step in &t.steps {
            for gate in &step.gates {
                if let Some(mode) = crate::deliver::step::mode_of(gate) {
                    crate::deliver::word::spliced(kb, &t.name, mode)?;
                    continue;
                }
                if crate::deliver::step::posted_of(gate).is_some() {
                    continue;
                }
                let file = format!("{}/{gate}.toml", def::DIR);
                if let Some((_, problem)) = problems.iter().find(|(_, p)| p.path == file) {
                    report.extend(problem.report.clone());
                    continue;
                }
                let place = (step.id.as_str(), gate.as_str());
                let (why, label, help) = match found.iter().find(|d| &d.id == gate) {
                    Some(d) if d.on_candidate() => (
                        format!(
                            "gate `{gate}` judges delivery candidates; a plan step owes a delivery by its mode"
                        ),
                        "a candidate gate".to_owned(),
                        Some("name the delivery mode instead: deliver:<mode>".to_owned()),
                    ),
                    Some(_) => continue,
                    None => (
                        format!(
                            "step `{}` owes gate `{gate}`, which is not defined",
                            step.id
                        ),
                        format!("no {file} in the KB root or the repository"),
                        did_you_mean(&suggest(gate, &ids), '`'),
                    ),
                };
                let code = match why.contains("candidates") {
                    true => "ref.wrong-kind",
                    false => "ref.undefined",
                };
                report.extend(with(at_word(t, source, place, code, why), &label, help));
            }
        }
    }
    match report.diagnostics.is_empty() {
        true => Ok(()),
        false => {
            report.sort();
            Err(KbError::Definition(Box::new(report)))
        }
    }
}
