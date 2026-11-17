//! Diagnostics for the files a person or an agent writes: the fault's
//! file, line and column, the source line it is on, what is wrong and,
//! when Writfold can name it, the fix. Pure: no IO, no clock, no
//! environment; the caller decides colour and width ([`render::Style`]).
//!
//! Only [`blocks`] (whether a lint fails) is translated and proved; the
//! rest is rendering, not a decision kernel: iterators and derives are free.
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

pub mod lines;
pub mod render;
pub mod snippet;
pub mod suggest;

pub use lines::LineIndex;
pub use render::{Style, render, render_capped};
pub use suggest::{distance, suggest};

/// Every code a diagnostic may carry. Codes are never renamed or reused;
/// a test holds this list, the specification's table, the guide's table
/// and the codes the source emits to the same set.
pub const CODES: [&str; 28] = [
    "syntax.unclosed-array",
    "syntax.unclosed-string",
    "syntax.expected",
    "syntax.escape",
    "syntax.subset",
    "syntax.json",
    "key.unknown",
    "key.twice",
    "key.missing",
    "key.type",
    "key.range",
    "key.not-here",
    "value.unknown",
    "value.form",
    "value.twice",
    "value.pattern",
    "value.class",
    "value.unmeetable",
    "value.conflict",
    "ref.undefined",
    "ref.wrong-kind",
    "ref.cycle",
    "file.name",
    "file.twice",
    "file.unreadable",
    "section.missing",
    "section.form",
    "policy.waivable",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn name(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

/// A byte range of a file's text.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        let clamp = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        Span {
            start: clamp(start),
            end: clamp(end.max(start)),
        }
    }
}

/// Where a diagnostic points: 1-based line and column; `col` and `len`
/// count Unicode scalar values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct At {
    pub line: u32,
    pub col: u32,
    pub len: u32,
}

/// One fault in one file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    /// As rows show it: `.writfold/gates/unit.toml`, `<repo>:.writfold/…`.
    pub file: String,
    /// `None`: about the file as a whole (a missing key).
    pub at: Option<At>,
    /// One sentence: the fault.
    pub message: String,
    /// A few words printed beside the caret; may be empty.
    pub label: String,
    /// One concrete fix.
    pub help: Option<String>,
    /// Context: the closed set, where an array opened.
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn new(severity: Severity, code: &'static str, file: &str, message: String) -> Self {
        Diagnostic {
            severity,
            code,
            file: String::from(file),
            at: None,
            message,
            label: String::new(),
            help: None,
            notes: Vec::new(),
        }
    }

    pub fn label(&mut self, text: &str) -> &mut Self {
        self.label = String::from(text);
        self
    }

    pub fn help(&mut self, text: String) -> &mut Self {
        self.help = Some(text);
        self
    }

    pub fn note(&mut self, text: String) -> &mut Self {
        self.notes.push(text);
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// The text a report was made from: the renderer never re-reads a file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Source {
    pub file: String,
    pub text: String,
}

/// Diagnostics with the sources they point into.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Report {
    pub sources: Vec<Source>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_error()).count()
    }

    pub fn warnings(&self) -> usize {
        self.diagnostics.len() - self.errors()
    }

    /// The files with at least one diagnostic.
    pub fn files(&self) -> usize {
        let mut files: Vec<&str> = self.diagnostics.iter().map(|d| d.file.as_str()).collect();
        files.sort_unstable();
        files.dedup();
        files.len()
    }

    /// Add another report's sources and diagnostics.
    pub fn extend(&mut self, other: Report) {
        for source in other.sources {
            if !self.sources.iter().any(|s| s.file == source.file) {
                self.sources.push(source);
            }
        }
        self.diagnostics.extend(other.diagnostics);
    }

    /// The text of `file`, if it is one of the sources.
    pub fn text(&self, file: &str) -> Option<&str> {
        let found = self.sources.iter().find(|s| s.file == file);
        found.map(|s| s.text.as_str())
    }

    /// Order: files as the sources list them (KB root, then repository,
    /// by path), then by line and column, file-level diagnostics last,
    /// errors before warnings at one position; one diagnostic per code
    /// and position.
    pub fn sort(&mut self) {
        let rank = |file: &str| self.sources.iter().position(|s| s.file == file);
        let mut keyed: Vec<_> = self
            .diagnostics
            .drain(..)
            .map(|d| {
                let place = d.at.map_or((1, 0, 0), |a| (0, a.line, a.col));
                let file = rank(&d.file).unwrap_or(usize::MAX);
                ((file, d.file.clone(), place, d.severity), d)
            })
            .collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        let mut out: Vec<Diagnostic> = Vec::new();
        for (_, d) in keyed {
            let twice = out.iter().any(|o| {
                o.file == d.file
                    && o.code == d.code
                    && o.at == d.at
                    && (o.at.is_some() || o.message == d.message)
            });
            if !twice {
                out.push(d);
            }
        }
        self.diagnostics = out;
    }
}

fn plural(n: usize, one: &str) -> String {
    match n {
        1 => format!("1 {one}"),
        _ => format!("{n} {one}s"),
    }
}

/// `2 errors in 1 file`, `1 error, 3 warnings in 2 files`.
pub fn tally(errors: usize, warnings: usize, files: usize) -> String {
    let mut out = plural(errors, "error");
    if warnings > 0 {
        out.push_str(&format!(", {}", plural(warnings, "warning")));
    }
    format!("{out} in {}", plural(files, "file"))
}

/// `1 error`, `2 errors, 1 warning`: the count of one file.
/// Whether a lint blocks (exit 2): any error, or any warning under
/// `--deny-warnings`. Translated and proved (`proofs/Writfold/Walk.lean`).
pub fn blocks(errors: usize, warnings: usize, deny: bool) -> bool {
    errors > 0 || (deny && warnings > 0)
}

pub fn count(errors: usize, warnings: usize) -> String {
    let mut out = plural(errors, "error");
    if warnings > 0 {
        out.push_str(&format!(", {}", plural(warnings, "warning")));
    }
    out
}

/// The compact form, where a snippet does not fit (rows, hooks):
/// `file:line:col: message; help`. Control characters are escaped.
pub fn line(d: &Diagnostic) -> String {
    let mut out = lines::clean(&d.file);
    if let Some(at) = d.at {
        out.push_str(&format!(":{}:{}", at.line, at.col));
    }
    out.push_str(": ");
    out.push_str(&lines::clean(&d.message));
    if let Some(help) = &d.help {
        out.push_str("; ");
        out.push_str(&lines::clean(help));
    }
    out
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_more;
