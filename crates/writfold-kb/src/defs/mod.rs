//! The files a person or an agent writes by hand (gate definitions, guard
//! rules, plan templates …): one reader with spans ([`toml`]), typed
//! access that reports every fault ([`fields`]), and the sink the faults
//! go to. Every fault becomes a [`Diagnostic`] with the file, line and
//! column, ready for [`writfold_core::diag::render`].
use writfold_core::diag::{Diagnostic, LineIndex, Report, Severity, Source, Span, line};

pub mod array;
pub mod closed;
pub mod doc;
pub mod fields;
pub mod jsonl;
pub mod lex;
pub mod lint;
pub mod lint_aliases;
pub mod lint_cycles;
pub mod lint_items_from;
pub mod lint_looks;
pub mod lint_out;
pub mod lint_policy;
pub mod lint_refs;
pub mod records;
pub mod toml;

pub use fields::{Check, Field};
pub use toml::{Doc, Entry, Slot, Spanned, Text, Value};

/// Where one file's diagnostics are collected.
pub struct Sink<'t> {
    pub file: String,
    pub text: &'t str,
    index: LineIndex,
    pub found: Vec<Diagnostic>,
    /// The reader recovered from a syntax fault: a skipped line may hold
    /// a key, so no key is reported missing.
    pub recovered: bool,
    /// Keys an unknown key was said to mean: not also reported missing.
    pub suggested: Vec<String>,
}

impl<'t> Sink<'t> {
    pub fn new(file: &str, text: &'t str) -> Self {
        Sink {
            file: file.to_owned(),
            text,
            index: LineIndex::new(text),
            found: Vec::new(),
            recovered: false,
            suggested: Vec::new(),
        }
    }

    pub fn at(&self, span: Span) -> writfold_core::diag::At {
        self.index.at(self.text, span)
    }

    pub fn push(
        &mut self,
        severity: Severity,
        code: &'static str,
        span: Option<Span>,
        message: String,
    ) -> &mut Diagnostic {
        let mut d = Diagnostic::new(severity, code, &self.file, message);
        d.at = span.map(|s| self.at(s));
        self.found.push(d);
        let n = self.found.len() - 1;
        &mut self.found[n]
    }

    pub fn error(
        &mut self,
        code: &'static str,
        span: Option<Span>,
        message: String,
    ) -> &mut Diagnostic {
        self.push(Severity::Error, code, span, message)
    }

    /// A reader fault: reported, and the file marked as recovered.
    pub fn fault(&mut self, fault: lex::Fault) {
        self.recovered = true;
        let d = self.error(fault.code, Some(fault.span), fault.message);
        d.label(fault.label);
        d.help = fault.help;
        d.notes = fault.notes;
    }

    pub fn errors(&self) -> usize {
        self.found.iter().filter(|d| d.is_error()).count()
    }

    /// The report: the file's text and its diagnostics, in order.
    pub fn report(self) -> Report {
        let mut report = Report {
            sources: vec![Source {
                file: self.file,
                text: self.text.to_owned(),
            }],
            diagnostics: self.found,
        };
        report.sort();
        report
    }
}

/// A file that could not be used: where it is and why.
#[derive(Clone, Debug)]
pub struct Problem {
    /// As rows show it, relative to its root.
    pub path: String,
    pub report: Report,
}

impl Problem {
    /// A problem about the file as a whole (unreadable, defined twice).
    pub fn whole(path: &str, shown: &str, code: &'static str, why: String) -> Self {
        let d = Diagnostic::new(Severity::Error, code, shown, why);
        Problem {
            path: path.to_owned(),
            report: Report {
                sources: Vec::new(),
                diagnostics: vec![d],
            },
        }
    }

    /// The compact line of the first diagnostic, `(+N more)` after it.
    pub fn why(&self) -> String {
        why(&self.report)
    }
}

/// The compact line of a report's first diagnostic, `(+N more)` after it.
pub fn why(report: &Report) -> String {
    let Some(first) = report.diagnostics.first() else {
        return String::new();
    };
    let more = report.diagnostics.len() - 1;
    match more {
        0 => line(first),
        n => format!("{} (+{n} more)", line(first)),
    }
}

/// Read a file's text, reporting what makes it unusable.
pub fn read_text(path: &std::path::Path, shown: &str) -> Result<String, Problem> {
    let bytes = std::fs::read(path).map_err(|e| {
        Problem::whole(
            shown,
            shown,
            "file.unreadable",
            format!("cannot be read: {e}"),
        )
    })?;
    String::from_utf8(bytes)
        .map_err(|_| Problem::whole(shown, shown, "file.unreadable", "is not UTF-8 text".into()))
}

/// The `diagnostics` array of a JSON reply: one object per diagnostic;
/// `line`, `col` and `len` are null together for a file-level one.
pub fn json(report: &Report) -> serde_json::Value {
    let all = report.diagnostics.iter().map(|d| {
        serde_json::json!({"severity": d.severity.name(), "code": d.code, "file": d.file,
            "line": d.at.map(|a| a.line), "col": d.at.map(|a| a.col), "len": d.at.map(|a| a.len),
            "message": d.message, "label": d.label, "help": d.help, "notes": d.notes})
    });
    serde_json::Value::Array(all.collect())
}

/// The row of a definition file that does not parse: its text is the
/// first fault, its source that fault's line; `help:` and `next:
/// writfold kb lint <path>` follow.
pub fn invalid_row(
    kind: &'static str,
    id: &str,
    problem: &Problem,
    section: Option<&'static str>,
) -> crate::emit::Row {
    let first = problem.report.diagnostics.first();
    let shown = first.map_or_else(|| problem.path.clone(), |d| d.file.clone());
    let mut more = Vec::new();
    if let Some(help) = first.and_then(|d| d.help.clone()) {
        more.push(crate::emit::More {
            label: "help",
            text: help,
            ..Default::default()
        });
    }
    more.push(crate::emit::More {
        label: "next",
        text: format!("writfold kb lint {shown}"),
        ..Default::default()
    });
    crate::emit::Row {
        kind,
        id: id.to_owned(),
        status: Some("invalid".into()),
        text: first.map_or_else(String::new, |d| d.message.clone()),
        path: Some(shown),
        line: Some(first.and_then(|d| d.at).map_or(1, |a| a.line)),
        flags: vec!["invalid".into()],
        more,
        section,
        ..Default::default()
    }
}

/// The stem of a definition's file name: what a row names it by.
pub fn stem(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .unwrap_or("")
        .trim_end_matches(".toml")
}
