//! `use=` cycles among plan templates, found without splicing: a template
//! that includes itself through others, or nests past the kernel's
//! bound, is reported at the `use=` word that closes the cycle.
use super::Sink;
use super::lint::{File, Kind};
use crate::plan::refs::span_of;
use writfold_core::diag::Diagnostic;

/// Each template's name, its file and its includes `(step, name)`.
fn graph(all: &[File]) -> Vec<(&File, Vec<(String, String)>)> {
    all.iter()
        .filter(|f| f.kind == Kind::Template)
        .filter_map(|f| {
            let (_, _, steps) =
                crate::plan::template::check(&f.text, Some(&f.stem), &f.shown).ok()?;
            let uses = steps
                .into_iter()
                .filter_map(|s| s.include.map(|i| (s.id, i)))
                .collect();
            Some((f, uses))
        })
        .collect()
}

fn walk<'a>(
    graph: &'a [(&File, Vec<(String, String)>)],
    stack: &mut Vec<&'a str>,
    out: &mut Vec<Diagnostic>,
) {
    let Some(&top) = stack.last() else {
        return;
    };
    let Some((f, uses)) = graph.iter().find(|(f, _)| f.stem == top) else {
        return;
    };
    for (step, name) in uses {
        let path = format!("{} → {name}", stack.join(" → "));
        let why = match (
            stack.contains(&name.as_str()),
            stack.len() >= writfold_core::plan::MAX_DEPTH,
        ) {
            (true, _) => format!("{name} includes itself: {path}"),
            (_, true) => format!(
                "{} nests deeper than {} levels: {path}",
                stack[0],
                writfold_core::plan::MAX_DEPTH
            ),
            _ => {
                stack.push(name);
                walk(graph, stack, out);
                stack.pop();
                continue;
            }
        };
        let mut sink = Sink::new(&f.shown, &f.text);
        let span = span_of(&f.text, step, &format!("use={name}"));
        sink.error("ref.cycle", span, why).label("closes the cycle");
        out.extend(sink.found);
    }
}

/// Every cycle, once per `use=` word that closes it.
pub fn cycles(all: &[File]) -> Vec<Diagnostic> {
    let graph = graph(all);
    let mut out: Vec<Diagnostic> = Vec::new();
    for (f, _) in &graph {
        let mut stack = vec![f.stem.as_str()];
        walk(&graph, &mut stack, &mut out);
    }
    let mut kept: Vec<Diagnostic> = Vec::new();
    for d in out {
        if !kept.iter().any(|k| k.file == d.file && k.at == d.at) {
            kept.push(d);
        }
    }
    kept
}
