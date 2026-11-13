//! The frame as lines with a tone, shared by the terminal view and
//! `--once`. A row is printed as the CLI prints it (`kind id [status] text
//! @path:line !flag`, continuation lines indented), so every line on screen
//! carries its source position. Tones only colour what the verb already
//! flagged.
use crate::app::{App, View};
use crate::source::{Reply, field, flags};
use serde_json::Value;

/// How a line is drawn: failed, stale and invalid in red, open and unknown
/// in yellow, satisfied in green.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Dim,
    Head,
    Good,
    Warn,
    Bad,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub tone: Tone,
}

fn line(text: impl Into<String>, tone: Tone) -> Line {
    Line {
        text: text.into(),
        tone,
    }
}

const BAD: &[&str] = &[
    "failed",
    "stale",
    "invalid",
    "changed",
    "gone",
    "expired",
    "moved",
    "regression",
    "disagree",
];
const WARN: &[&str] = &["open", "unknown", "unverifiable", "draft", "past-ttl"];

/// The tone of a row from its status and flags.
pub fn tone(row: &Value) -> Tone {
    let flagged = flags(row);
    let has = |words: &[&str]| {
        flagged
            .iter()
            .any(|f| words.iter().any(|w| f.starts_with(w)))
    };
    if has(BAD) {
        Tone::Bad
    } else if has(WARN) {
        Tone::Warn
    } else if field(row, "status") == "satisfied" {
        Tone::Good
    } else {
        Tone::Plain
    }
}

fn position(row: &Value) -> String {
    let mut out = String::new();
    let path = field(row, "path");
    if !path.is_empty() {
        out.push_str(&format!(" @{path}"));
        if let Some(n) = row.get("line").and_then(Value::as_u64) {
            out.push_str(&format!(":{n}"));
        }
    }
    for flag in flags(row) {
        out.push_str(&format!(" !{flag}"));
    }
    out
}

/// One row group: its head line and its continuation lines.
pub fn row(row: &Value) -> Vec<Line> {
    let mut head = format!("{} {}", field(row, "kind"), field(row, "id"));
    let status = field(row, "status");
    if !status.is_empty() {
        head.push_str(&format!(" [{status}]"));
    }
    let text = field(row, "text");
    if !text.is_empty() {
        head.push_str(&format!(" {text}"));
    }
    head.push_str(&position(row));
    let tone = tone(row);
    let mut out = vec![line(head, tone)];
    for more in row
        .get("more")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let text = format!(
            "  {}: {}{}",
            field(more, "label"),
            field(more, "text"),
            position(more)
        );
        let more_tone = match crate::lines::tone(more) {
            Tone::Plain if tone == Tone::Bad => Tone::Bad,
            Tone::Plain => Tone::Dim,
            other => other,
        };
        out.push(line(text, more_tone));
    }
    out
}

/// Every row, then the notes, or the verb's refusal.
pub fn reply(reply: &Reply) -> Vec<Line> {
    if let Some(error) = &reply.error {
        return vec![line(
            format!("refused (exit {}): {error}", reply.exit),
            Tone::Bad,
        )];
    }
    let mut out: Vec<Line> = reply.rows.iter().flat_map(row).collect();
    out.extend(
        reply
            .notes
            .iter()
            .map(|n| line(format!("# note: {n}"), Tone::Dim)),
    );
    if out.is_empty() {
        out.push(line("(no rows)", Tone::Dim));
    }
    out
}

/// One `kb runs` row as a list entry: state and ID, task, place, and the
/// last checkpoint's time and first line.
pub fn run_item(row: &Value) -> Vec<Line> {
    let state = field(row, "status");
    let tone = if state == "closed" {
        Tone::Dim
    } else {
        Tone::Plain
    };
    let mut head = format!("[{state}] {}", field(row, "id"));
    for flag in flags(row) {
        head.push_str(&format!(" !{flag}"));
    }
    let mut out = vec![
        line(head, tone),
        line(format!("  {}", field(row, "text")), tone),
    ];
    for more in row
        .get("more")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let text = field(more, "text");
        match field(more, "label") {
            "field" if !text.is_empty() => out.push(line(format!("  {text}"), Tone::Dim)),
            "checkpoint" => out.push(line(format!("  last: {text}"), Tone::Dim)),
            _ => {}
        }
    }
    out
}

/// The gate counts of `kb gate status`, in a fixed order; red when any
/// gate is failed, stale or backed by an invalid receipt.
pub fn gate_summary(status: &Reply) -> Line {
    if status.trailer_text("reason").as_deref() == Some("no-gates") {
        return line("gates: none defined", Tone::Dim);
    }
    if let Some(error) = &status.error {
        return line(format!("gates: {error}"), Tone::Dim);
    }
    let counts = status.counts();
    let get = |name: &str| counts.iter().find(|(n, _)| n == name).map(|(_, c)| *c);
    let order = [
        "gates",
        "satisfied",
        "open",
        "stale",
        "failed",
        "unknown",
        "attested",
        "waived",
        "invalid",
    ];
    let parts: Vec<String> = order
        .iter()
        .filter_map(|name| get(name).map(|n| format!("{name}={n}")))
        .collect();
    let bad = ["stale", "failed", "invalid"]
        .iter()
        .any(|n| get(n).unwrap_or(0) > 0);
    let warn = ["open", "unknown"].iter().any(|n| get(n).unwrap_or(0) > 0);
    let tone = match (bad, warn) {
        (true, _) => Tone::Bad,
        (false, true) => Tone::Warn,
        _ => Tone::Good,
    };
    line(format!("gates: {}", parts.join(" ")), tone)
}

/// The detail pane's title and lines for the selected run.
pub fn detail(app: &App) -> (String, Vec<Line>) {
    let run = app.run().unwrap_or_default();
    let project = app.project();
    let scope = project.as_deref().unwrap_or("(inferred)");
    let title = match app.view {
        View::Resume => format!("resume {run}"),
        View::Stale => format!("stale {scope}"),
        View::Brief => format!("brief {scope}"),
    };
    let mut out = Vec::new();
    if app.view == View::Resume
        && let Some(status) = app.gate_status()
    {
        out.push(gate_summary(status));
    }
    match app.detail() {
        Some(answer) => out.extend(reply(answer)),
        None => out.push(line("(no run selected)", Tone::Dim)),
    }
    (title, out)
}

/// The bottom line: the KB pin and the index state the runs were read at.
pub fn footer(app: &App) -> String {
    let at = |key: &str| {
        app.runs
            .trailer_text(key)
            .unwrap_or_else(|| "unknown".into())
    };
    format!(
        "kb={} index={} {}  read-only; no daemon, nothing here claims a run is live",
        at("kb"),
        at("index"),
        crate::poll::subject_text(&app.stamps.subject)
    )
}

pub const HELP: &[&str] = &[
    "writfold watch: a read-only view of the KB verbs",
    "",
    "  j k ↑ ↓   select a run (or scroll the right pane)",
    "  Enter     move between the runs and the right pane",
    "  Tab       right pane: resume → stale → brief",
    "  g s b     resume / stale / brief",
    "  r         ask the verbs again now",
    "  ?         this help      q  quit",
    "",
    "KB and local state are polled every 2 s; the subject every 10 s.",
    "--repo selects the gate subject. Nothing here writes to the KB.",
];

/// The whole frame as plain text (`--once`).
pub fn once(app: &App) -> String {
    let mut out = String::new();
    let open = app
        .runs
        .rows
        .iter()
        .filter(|r| field(r, "status") != "closed")
        .count();
    out.push_str(&format!(
        "== runs ({} listed, {open} not closed) ==\n",
        app.runs.rows.len()
    ));
    if let Some(error) = &app.runs.error {
        out.push_str(&format!("refused (exit {}): {error}\n", app.runs.exit));
    }
    for (n, i) in app.order.iter().enumerate() {
        let mark = if n == app.selected { "> " } else { "  " };
        for (k, l) in run_item(&app.runs.rows[*i]).into_iter().enumerate() {
            out.push_str(if k == 0 { mark } else { "  " });
            out.push_str(&l.text);
            out.push('\n');
        }
    }
    let (title, lines) = detail(app);
    out.push_str(&format!("== {title} ==\n"));
    for l in lines {
        out.push_str(&l.text);
        out.push('\n');
    }
    out.push_str(&format!("== {} ==\n", footer(app)));
    out
}
