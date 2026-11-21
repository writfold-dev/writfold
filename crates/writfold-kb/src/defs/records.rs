//! `kb lint --records`: the records a person writes, read the way the
//! index reads them, with each fault at its position: a JSON record that
//! is not JSON (the parser's own line and column), a front matter fence
//! that is missing or never closed, a key given twice, a required key
//! absent, a citation token that does not read, and a token missing a
//! field its kind declares (with the written field it may mean).
use super::Sink;
use super::lint::{File, Kind as FileKind};
use crate::gate::def::Origin;
use crate::layout::{Kind, classify};
use crate::stale::kinds::Kinds;
use std::path::Path;
use writfold_core::diag::suggest::{did_you_mean, suggest};
use writfold_core::diag::{LineIndex, Report, Span};

/// The key a record kind cannot be read without, when it has one.
fn required(kind: Kind) -> Option<(&'static str, &'static str)> {
    match kind {
        Kind::Proposal => Some(("id", "a decision record")),
        Kind::Ticket => Some(("ticket", "a ticket record")),
        _ => None,
    }
}

/// Byte offset of 1-based `line`, `col` (columns as bytes, as serde counts).
fn offset(text: &str, line: usize, col: usize) -> usize {
    let start: usize = text
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum();
    (start + col.saturating_sub(1)).min(text.len())
}

fn json(sink: &mut Sink<'_>, text: &str) -> bool {
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(_) => true,
        Err(e) => {
            let at = offset(text, e.line(), e.column());
            let why = e.to_string();
            let why = why.split(" at line ").next().unwrap_or(&why).to_owned();
            let next = text[at..].chars().next().map_or(0, char::len_utf8);
            sink.error(
                "syntax.json",
                Some(Span::new(at, at + next)),
                format!("this record is not JSON: {why}"),
            )
            .label("here");
            false
        }
    }
}

/// The front matter: its fence, its keys once each, its required key.
fn front(sink: &mut Sink<'_>, kind: Kind, text: &str) -> bool {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let fence = |l: &str| l.trim_end_matches(['\r', '\n']).trim() == "---";
    if !lines.first().is_some_and(|l| fence(l)) {
        if let Some((key, what)) = required(kind) {
            let d = sink.error(
                "syntax.expected",
                Some(Span::new(0, 0)),
                format!("{what} starts with a --- front matter fence"),
            );
            d.label("no front matter");
            d.help(format!(
                "add ---, a line {key}: <value>, and --- at the top"
            ));
            return false;
        }
        return true;
    }
    let Some(close) = lines.iter().skip(1).position(|l| fence(l)).map(|p| p + 1) else {
        let d = sink.error(
            "syntax.expected",
            Some(Span::new(0, 3)),
            "the front matter opened here is never closed".into(),
        );
        d.label("opens here");
        d.help("add a line --- after the fields".into());
        return false;
    };
    let mut seen: Vec<(String, u32)> = Vec::new();
    let mut at = lines[0].len();
    for (n, line) in lines[1..close].iter().enumerate() {
        let start = at;
        at += line.len();
        let Some((key, _)) = line
            .split_once(':')
            .filter(|(k, _)| !k.starts_with([' ', '\t', '-', '#']) && !k.trim().is_empty())
        else {
            continue;
        };
        let span = Span::new(start, start + key.len());
        match seen.iter().find(|(k, _)| k == key.trim()) {
            Some((_, first)) => {
                let d = sink.error(
                    "key.twice",
                    Some(span),
                    format!("`{}` is given twice", key.trim()),
                );
                d.label("given again");
                d.note(format!("first given on line {first}"));
            }
            None => seen.push((key.trim().to_owned(), u32::try_from(n + 2).unwrap_or(0))),
        }
    }
    if let Some((key, what)) = required(kind).filter(|(k, _)| !seen.iter().any(|(s, _)| s == k)) {
        let d = sink.error("key.missing", None, format!("{what} needs `{key}`"));
        d.help(format!("add a line to the front matter: {key}: <value>"));
        return false;
    }
    sink.found.is_empty()
}

/// Citation tokens that do not read, and a declared field a token lacks.
fn tokens(sink: &mut Sink<'_>, text: &str, kinds: &Kinds) {
    let index = LineIndex::new(text);
    for c in crate::parse::cite::tokens(text)
        .into_iter()
        .filter(|c| !c.illustrative)
    {
        let line = index.line(text, c.line);
        let line_start = offset(text, usize::try_from(c.line).unwrap_or(1), 1);
        let col = line.find(&c.raw).unwrap_or(0);
        let span = Span::new(
            line_start + col,
            line_start + col + c.raw.len().min(line.len() - col),
        );
        if let Some(why) = &c.malformed {
            sink.error("value.form", Some(span), format!("citation token: {why}"))
                .label("does not read");
            continue;
        }
        let fields: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&c.fields).unwrap_or_default();
        let Some(kind) = c.kind.as_deref().and_then(|k| kinds.get(k)) else {
            continue;
        };
        for need in kind.required.iter().filter(|f| !fields.contains_key(*f)) {
            let written: Vec<&str> = fields.keys().map(String::as_str).collect();
            let close: Vec<&str> = written
                .iter()
                .copied()
                .filter(|w| !suggest(w, &[need.as_str()]).is_empty())
                .collect();
            let why = format!("a citation of kind `{}` needs `{need}`", kind.name);
            let d = sink.error("key.missing", Some(span), why);
            d.label("in this token");
            d.help = match close.first() {
                Some(w) => {
                    did_you_mean(&[need.as_str()], '`').map(|h| format!("{h} (written `{w}`)"))
                }
                None => Some(format!("add {need}=<value> to the token")),
            };
        }
    }
}

/// One record file's faults.
fn record(rel: &str, kind: Kind, text: &str, kinds: &Kinds) -> Report {
    let mut sink = Sink::new(rel, text);
    let readable = match kind {
        _ if rel.ends_with(".json") => json(&mut sink, text),
        _ if kind.needs_identity() => front(&mut sink, kind, text),
        _ => true,
    };
    if readable && let Err(why) = crate::parse::parse(rel, kind, text.as_bytes()) {
        let code = if why.contains("has no") {
            "key.missing"
        } else {
            "value.form"
        };
        sink.error(code, None, format!("this record cannot be read: {why}"));
    }
    if rel.ends_with(".md") {
        tokens(&mut sink, text, kinds);
    }
    let mut report = sink.report();
    if report.diagnostics.is_empty() {
        report.sources.clear();
    }
    report
}

/// Every record of the KB root, read and checked.
pub fn files(kb: &Path) -> Vec<File> {
    let kinds = crate::stale::kinds::read(kb).unwrap_or_default();
    let mut paths = Vec::new();
    walk(kb, kb, &mut paths);
    paths.sort();
    let mut out = Vec::new();
    for rel in paths {
        let Some(kind) = classify(&rel) else { continue };
        let (text, report) = match super::read_text(&kb.join(&rel), &rel) {
            Ok(text) => {
                let report = record(&rel, kind, &text, &kinds);
                (text, report)
            }
            Err(problem) => (String::new(), problem.report),
        };
        let stem = rel.rsplit('/').next().unwrap_or(&rel).to_owned();
        out.push(File {
            kind: FileKind::Record,
            origin: Origin::Kb,
            dir: kb.to_path_buf(),
            stem,
            shown: rel,
            text,
            report,
        });
    }
    super::lint_aliases::check(&mut out);
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}
