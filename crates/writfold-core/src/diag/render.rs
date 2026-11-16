//! The full rendering of a report:
//!
//! ```text
//! error[key.unknown]: unknown key `timout_seconds` in a gate definition
//!  --> .writfold/gates/unit.toml:6:1
//!   |
//! 6 | timout_seconds = 600
//!   | ^^^^^^^^^^^^^^ not a gate key
//!   |
//!   = help: did you mean `timeout_seconds`?
//! ```
//!
//! Plain output is the coloured output with the escape sequences removed.
use super::lines::{LineIndex, clean, wrap};
use super::snippet::draw;
use super::{Diagnostic, Report, Severity, count, tally};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// How the caller's stream shows text.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub colour: bool,
    /// The terminal's width when known, else 100; never under 40.
    pub width: usize,
    /// Each character's display width.
    pub cell: fn(char) -> usize,
}

fn one(_: char) -> usize {
    1
}

impl Style {
    /// No colour, 100 columns, one cell per character.
    pub fn plain() -> Self {
        Style {
            colour: false,
            width: 100,
            cell: one,
        }
    }

    fn room(&self) -> usize {
        self.width.max(40)
    }
}

const RESET: &str = "\x1b[0m";
const BLUE: &str = "\x1b[1;34m";
const GREEN: &str = "\x1b[1;32m";
const BOLD: &str = "\x1b[1m";

fn tone(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "\x1b[1;31m",
        Severity::Warning => "\x1b[1;33m",
    }
}

fn paint(style: &Style, colour: &str, text: &str) -> String {
    match style.colour && !text.is_empty() {
        true => format!("{colour}{text}{RESET}"),
        false => String::from(text),
    }
}

/// Lines of `text` after a prefix of `lead` cells, wrapped at `room`.
fn wrapped(text: &str, lead: usize, room: usize, style: &Style) -> Vec<String> {
    let space = room.saturating_sub(lead).max(20);
    wrap(&clean(text), space, space, style.cell)
}

fn push_block(out: &mut String, head: &str, lead: usize, body: &[String]) {
    out.push_str(head);
    for (n, line) in body.iter().enumerate() {
        if n > 0 {
            out.push('\n');
            out.push_str(&" ".repeat(lead));
        }
        out.push_str(line);
    }
    out.push('\n');
}

/// One diagnostic, without a trailing blank line.
fn one_diagnostic(out: &mut String, d: &Diagnostic, text: Option<&str>, style: &Style) {
    let room = style.room();
    let head = format!("{}[{}]", d.severity.name(), d.code);
    let lead = head.chars().count() + 2;
    let body = wrapped(&d.message, lead, room, style);
    let first = format!("{}: ", paint(style, tone(d.severity), &head));
    let painted: Vec<String> = body.iter().map(|l| paint(style, BOLD, l)).collect();
    push_block(out, &first, lead, &painted);
    let line = d.at.map_or(0, |a| a.line);
    let gutter = format!("{line}").len().max(1);
    let pad = " ".repeat(gutter);
    let bar = paint(style, BLUE, "|");
    let file = clean(&d.file);
    let place = match d.at {
        Some(at) => format!("{file}:{}:{}", at.line, at.col),
        None => file,
    };
    out.push_str(&format!("{pad}{} {place}\n", paint(style, BLUE, "-->")));
    let shown = d.at.zip(text);
    if let Some((at, text)) = shown {
        let source = LineIndex::new(text).line(text, at.line);
        let room_line = room.saturating_sub(gutter + 3);
        let drawn = draw(source, at.col, at.len, room_line, style.cell);
        let number = paint(style, BLUE, &format!("{}", at.line));
        out.push_str(&format!("{pad} {bar}\n{number} {bar} {}\n", drawn.text));
        let mut mark = "^".repeat(drawn.carets);
        let label = clean(&d.label);
        if !label.is_empty() {
            mark = format!("{mark} {label}");
        }
        let space = " ".repeat(drawn.caret_at);
        let painted = paint(style, tone(d.severity), &mark);
        out.push_str(&format!("{pad} {bar} {space}{painted}\n"));
    }
    let extra: Vec<(&str, &str)> = d
        .help
        .iter()
        .map(|h| ("help", h.as_str()))
        .chain(d.notes.iter().map(|n| ("note", n.as_str())))
        .collect();
    if extra.is_empty() {
        return;
    }
    out.push_str(&format!("{pad} {bar}\n"));
    for (kind, text) in extra {
        let lead_text = format!("{pad} = {kind}: ");
        let lead = lead_text.chars().count();
        let body = wrapped(text, lead, room, style);
        let word = match kind {
            "help" => paint(style, GREEN, kind),
            _ => paint(style, BOLD, kind),
        };
        push_block(out, &format!("{pad} = {word}: "), lead, &body);
    }
}

/// The full rendering, capped at 20 diagnostics per file and 100 in all.
pub fn render(report: &Report, style: &Style) -> String {
    render_capped(report, style, 20, 100)
}

/// The rendering with explicit caps; whatever is cut is counted. After
/// the last diagnostic, when there is more than one or some were cut:
/// `N errors, M warnings in K files`.
pub fn render_capped(report: &Report, style: &Style, per_file: usize, total: usize) -> String {
    let mut out = String::new();
    let mut shown = 0;
    let mut n = 0;
    let all = &report.diagnostics;
    while n < all.len() {
        let file = &all[n].file;
        let end = all[n..]
            .iter()
            .position(|d| d.file != *file)
            .map_or(all.len(), |p| n + p);
        let text = report.text(file);
        let mut here = 0;
        for d in &all[n..end] {
            if here == per_file || shown == total {
                break;
            }
            if !out.is_empty() {
                out.push('\n');
            }
            one_diagnostic(&mut out, d, text, style);
            here += 1;
            shown += 1;
        }
        let cut = end - n - here;
        if cut > 0 && shown < total {
            out.push_str(&format!("\n… and {cut} more in this file\n"));
        }
        n = end;
    }
    if shown < all.len() && shown == total {
        out.push_str(&format!("\n… and {} more\n", all.len() - shown));
    }
    if all.len() > 1 || shown < all.len() {
        let line = tally(report.errors(), report.warnings(), report.files());
        out.push_str(&format!("\n{line}\n"));
    }
    out
}

/// `1 error`, for the refusal line that follows a rendering.
pub fn counted(report: &Report) -> String {
    count(report.errors(), report.warnings())
}
