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
pub mod lex;
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
