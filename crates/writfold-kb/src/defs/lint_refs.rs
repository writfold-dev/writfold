//! The references `kb lint` checks between files that parsed: a plan
//! step's gates, modes and included templates, `required.toml`'s
//! templates, a guard rule's `gate:<id>`, the same name in both roots and
//! `use=` cycles. A reference to a file that exists but has errors is not
//! reported again. A KB-root file naming a gate the KB root lacks is a
//! warning when no repository is in view (the gate may live in the
//! repository a run works in), an error when one is.
use super::lint::{File, Kind};
use super::{Sink, Slot, Value, toml};
use crate::gate::def::Origin;
use crate::plan::refs::span_of;
use writfold_core::diag::suggest::{did_you_mean, suggest};
use writfold_core::diag::{Report, Severity, Span};

/// Where item `value` of array `key` is written.
fn item_span(text: &str, key: &str, value: &str) -> Option<Span> {
    let doc = toml::read(text, &mut Sink::new("", text));
    let Some(Slot::Value(v)) = doc.get(key).map(|e| &e.value) else {
        return None;
    };
    let Value::List(items) = &v.value else {
        return None;
    };
    items.iter().find(|i| i.str() == value).map(|i| i.span)
}

struct Names {
    gates: Vec<(String, bool)>,
    broken: Vec<String>,
    templates: Vec<String>,
    repo: bool,
}

fn clean(f: &File) -> bool {
    f.report.diagnostics.iter().all(|d| !d.is_error())
}

fn names(all: &[File], repo: bool) -> Names {
    let mut names = Names {
        gates: Vec::new(),
        broken: Vec::new(),
        templates: Vec::new(),
        repo,
    };
    for f in all {
        match (f.kind, clean(f)) {
            (Kind::Gate, true) if names.gates.iter().any(|(g, _)| *g == f.stem) => {}
            (Kind::Gate, true) => {
                let parsed =
                    crate::gate::def_check::check(&f.text, &f.stem, f.origin, &f.dir, &f.shown);
                let candidate = parsed.is_ok_and(|d| d.on_candidate());
                names.gates.push((f.stem.clone(), candidate));
            }
            (Kind::Gate, false) => names.broken.push(f.stem.clone()),
            (Kind::Template, _) if !names.templates.contains(&f.stem) => {
                names.templates.push(f.stem.clone())
            }
            _ => {}
        }
    }
    names
}

fn push(
    sink: &mut Sink<'_>,
    severity: Severity,
    span: Option<Span>,
    (code, why, label): (&'static str, String, String),
    help: Option<String>,
) {
    let d = sink.push(severity, code, span, why);
    d.label = label;
    d.help = help;
}

/// A gate word: defined and judging the repository, or reported.
fn gate_ref(
    sink: &mut Sink<'_>,
    f: &File,
    n: &Names,
    (owner, gate): (&str, &str),
    span: Option<Span>,
) {
    if n.broken.iter().any(|b| b == gate) {
        return;
    }
    match n.gates.iter().find(|(id, _)| id == gate) {
        Some((_, false)) => {}
        Some((_, true)) => {
            let why = format!("gate `{gate}` judges delivery candidates; {owner} cannot owe it");
            let label = "a candidate gate".to_owned();
            push(
                sink,
                Severity::Error,
                span,
                ("ref.wrong-kind", why, label),
                None,
            );
        }
        None => {
            let ids: Vec<&str> = n
                .gates
                .iter()
                .filter(|(_, c)| !c)
                .map(|(id, _)| id.as_str())
                .collect();
            let severity = match f.origin == Origin::Kb && !n.repo {
                true => Severity::Warning,
                false => Severity::Error,
            };
            let why = format!("{owner} owes gate `{gate}`, which is not defined");
            let label = format!(
                "no {}/{gate}.toml in the KB root or the repository",
                crate::gate::def::DIR
            );
            push(
                sink,
                severity,
                span,
                ("ref.undefined", why, label),
                did_you_mean(&suggest(gate, &ids), '`'),
            );
        }
    }
}

fn template_ref(sink: &mut Sink<'_>, n: &Names, owner: &str, name: &str, span: Option<Span>) {
    if n.templates.iter().any(|t| t == name) {
        return;
    }
    let known: Vec<&str> = n.templates.iter().map(String::as_str).collect();
    let why = format!("{owner} names template `{name}`, which is not defined");
    let label = format!(
        "no {}/{name}.toml in the KB root or the repository",
        crate::plan::template::DIR
    );
    push(
        sink,
        Severity::Error,
        span,
        ("ref.undefined", why, label),
        did_you_mean(&suggest(name, &known), '`'),
    );
}

fn mode_ref(sink: &mut Sink<'_>, f: &File, owner: &str, mode: &str, span: Option<Span>) {
    let file = format!("{}/{mode}.toml", crate::deliver::config::DIR);
    let kb = match f.origin {
        Origin::Kb => f
            .dir
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent()),
        Origin::Repo => None,
    };
    let exists =
        crate::deliver::mode::MODES.contains(&mode) || kb.is_some_and(|k| k.join(&file).is_file());
    if !exists && f.origin == Origin::Kb {
        let why = format!("{owner} owes deliver:{mode}, which is not a delivery mode");
        let label = format!("no {file} in the KB root, and not a built-in mode");
        let modes: Vec<&str> = crate::deliver::mode::MODES.to_vec();
        push(
            sink,
            Severity::Error,
            span,
            ("ref.undefined", why, label),
            did_you_mean(&suggest(mode, &modes), '`'),
        );
    }
}

fn template(sink: &mut Sink<'_>, f: &File, n: &Names) {
    let Ok((_, _, steps)) = crate::plan::template::check(&f.text, Some(&f.stem), &f.shown) else {
        return;
    };
    for step in &steps {
        let owner = format!("step `{}`", step.id);
        for word in &step.gates {
            let span = span_of(&f.text, &step.id, word);
            match crate::deliver::step::mode_of(word) {
                Some(mode) => mode_ref(sink, f, &owner, mode, span),
                None => gate_ref(sink, f, n, (&owner, word), span),
            }
        }
        if let Some(name) = &step.include {
            let span = span_of(&f.text, &step.id, &format!("use={name}"));
            template_ref(sink, n, &owner, name, span);
        }
    }
}

fn file(f: &File, n: &Names) -> Report {
    let mut sink = Sink::new(&f.shown, &f.text);
    match f.kind {
        Kind::Template if clean(f) => template(&mut sink, f, n),
        Kind::Required if clean(f) => {
            if let Ok((_, items)) = crate::plan::required::parse_shown(f.text.as_bytes(), &f.shown)
            {
                for item in items {
                    template_ref(
                        &mut sink,
                        n,
                        "the required steps",
                        item.str(),
                        Some(item.span),
                    );
                }
            }
        }
        Kind::Guard if clean(f) => {
            if let Ok(rule) = crate::guard::rules::parse(&f.text, &f.stem, &f.shown) {
                for gate in rule.require.iter().filter_map(|r| match r {
                    crate::guard::rules::Require::Gate(g) => Some(g.as_str()),
                    _ => None,
                }) {
                    let span = item_span(&f.text, "require", &format!("gate:{gate}"));
                    let at = |n: u32| usize::try_from(n).unwrap_or(0);
                    let span =
                        span.map(|s| Span::new(at(s.start) + 6, at(s.end).saturating_sub(1)));
                    gate_ref(&mut sink, f, n, (&format!("rule `{}`", f.stem), gate), span);
                }
            }
        }
        _ => {}
    }
    sink.report()
}

/// Every reference fault between `all`, and every name in both roots.
pub fn check(all: &[File], repo: bool) -> Report {
    let n = names(all, repo);
    let mut report = Report::default();
    for f in all {
        let found = file(f, &n);
        if !found.diagnostics.is_empty() {
            report.extend(found);
        }
        let twice = all.iter().any(|o| {
            o.origin == Origin::Kb
                && f.origin == Origin::Repo
                && o.kind == f.kind
                && o.stem == f.stem
        });
        if twice {
            let mut sink = Sink::new(&f.shown, &f.text);
            let why = format!(
                "`{}` is defined in both the KB root and the repository",
                f.stem
            );
            push(
                &mut sink,
                Severity::Error,
                None,
                ("file.twice", why, String::new()),
                Some("keep one of the two files".into()),
            );
            report.extend(sink.report());
        }
    }
    report.diagnostics.extend(super::lint_cycles::cycles(all));
    report
}
