//! The output contract shared by every verb: budgeted rows, notes, and a
//! trailer that is always the last line. Budget and cursor follow Tuft:
//! each row group costs `ceil((bytes + 1) / 4)` estimated tokens per line,
//! rows stop at a group boundary, and the resume cursor is Tuft's v2 cursor
//! bound to the query digest, the KB pin digest and the digest of all rows.
use crate::error::Result;
use crate::index::{IndexState, Pin, hex};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use tuft::cursor::{Binding, Cursor};

/// A continuation line (`why:`, `cite:`, `via:` …) of the row above it.
#[derive(Clone, Debug, Default)]
pub struct More {
    pub label: &'static str,
    pub text: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub flags: Vec<String>,
}

/// One fact: `<kind> <id> [<status>] <text> @<path>[:<line>] [!<flag>…]`.
#[derive(Clone, Debug, Default)]
pub struct Row {
    pub kind: &'static str,
    pub id: String,
    pub status: Option<String>,
    pub text: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub flags: Vec<String>,
    pub more: Vec<More>,
    /// The output section, for per-section omission counts.
    pub section: Option<&'static str>,
}

/// What a verb found, before paging.
#[derive(Clone, Debug, Default)]
pub struct Answer {
    pub rows: Vec<Row>,
    pub notes: Vec<String>,
    /// `answer=unknown reason=<…>`.
    pub unknown: Option<&'static str>,
    /// A refusal (exit 3), such as an ambiguous target.
    pub refused: bool,
    /// The examined scope was incomplete for a verb-specific reason.
    pub partial: bool,
    /// The verb found a fault it reports in its rows (exit 1), such as an
    /// index that differs from a rebuild.
    pub failed: bool,
    /// Extra trailer counts such as `stale=` and `unverifiable=`.
    pub counts: Vec<(&'static str, usize)>,
}

/// The invocation: verb and arguments (they make the query digest), budget,
/// cursor and rendering.
#[derive(Clone, Debug)]
pub struct Frame {
    pub verb: String,
    pub args: Vec<String>,
    pub budget: usize,
    pub cursor: Option<String>,
    pub json: bool,
}

/// The snapshot the answer was read at.
pub struct Scope<'a> {
    pub pin: &'a Pin,
    pub index: IndexState,
    pub examined: usize,
    pub total: usize,
    pub unreadable: usize,
}

/// Rendered output and its exit code (0 complete, 2 partial/unknown/stale,
/// 3 refused).
pub struct Output {
    pub text: String,
    pub exit: u8,
}

fn plain(text: &str, space_ok: bool) -> bool {
    !text.is_empty()
        && !text.starts_with('"')
        && !text
            .chars()
            .any(|c| c.is_control() || (!space_ok && c.is_whitespace()))
}

/// Raw unless a control character, a leading quote (or, for IDs and paths,
/// whitespace) forces a JSON string literal.
fn shown(text: &str, space_ok: bool) -> String {
    if plain(text, space_ok) {
        text.to_owned()
    } else {
        Value::String(text.to_owned()).to_string()
    }
}

fn position(out: &mut String, path: Option<&String>, line: Option<u32>, flags: &[String]) {
    if let Some(path) = path {
        out.push_str(" @");
        out.push_str(&shown(path, false));
        if let Some(line) = line {
            out.push_str(&format!(":{line}"));
        }
    }
    for flag in flags {
        out.push_str(" !");
        out.push_str(flag);
    }
}

/// The compact text of one row group (no final line break).
pub fn render_row(row: &Row) -> String {
    let mut out = format!("{} {}", row.kind, shown(&row.id, false));
    if let Some(status) = &row.status {
        out.push_str(&format!(" [{}]", shown(status, false)));
    }
    if !row.text.is_empty() {
        out.push(' ');
        out.push_str(&shown(&row.text, true));
    }
    position(&mut out, row.path.as_ref(), row.line, &row.flags);
    for more in &row.more {
        out.push_str(&format!("\n  {}: {}", more.label, shown(&more.text, true)));
        position(&mut out, more.path.as_ref(), more.line, &more.flags);
    }
    out
}

fn cost(text: &str) -> usize {
    text.split('\n')
        .map(|line| (line.len() + 1).div_ceil(4))
        .sum()
}

/// The first 12 hex digits identify the query in the trailer; the full
/// digest binds the cursor.
pub fn query_digest(frame: &Frame) -> String {
    let mut material = format!("writfold-kb-query/1\n{}\n", frame.verb);
    for arg in &frame.args {
        material.push_str(arg);
        material.push('\n');
    }
    material.push_str(&format!("budget={}", frame.budget));
    hex(&Sha256::digest(material.as_bytes()))
}

struct Page {
    from: usize,
    to: usize,
    needs: Option<usize>,
    cursor: Option<String>,
}

fn page(groups: &[String], frame: &Frame, query: &str, pin: &Pin) -> Result<Page> {
    let mut view = Sha256::new();
    for group in groups {
        view.update(group.as_bytes());
        view.update(b"\n");
    }
    let view = hex(&view.finalize());
    let binding = Binding {
        program: query,
        commit: &pin.digest,
        view: &view,
    };
    let from = match &frame.cursor {
        Some(text) => Cursor::parse(text)?.resume(&binding, groups.len())?,
        None => 0,
    };
    let (mut to, mut used) = (from, 0);
    while to < groups.len() && used + cost(&groups[to]) <= frame.budget {
        used += cost(&groups[to]);
        to += 1;
    }
    let needs = (to == from && to < groups.len()).then(|| cost(&groups[to]));
    // No cursor when no row fits: resuming would make no progress; `needs`
    // says what budget the next row takes.
    let cursor = (to < groups.len() && to > from).then(|| Cursor::issue(&binding, to).to_string());
    Ok(Page {
        from,
        to,
        needs,
        cursor,
    })
}

/// Page, render and classify one answer.
pub fn emit(answer: &Answer, frame: &Frame, scope: &Scope<'_>) -> Result<Output> {
    let query = query_digest(frame);
    let groups: Vec<String> = answer.rows.iter().map(render_row).collect();
    let page = page(&groups, frame, &query, scope.pin)?;
    let mut omitted_by: Vec<(&str, usize)> = Vec::new();
    for row in &answer.rows[page.to..] {
        let name = row.section.unwrap_or("rows");
        match omitted_by.iter_mut().find(|(n, _)| *n == name) {
            Some((_, count)) => *count += 1,
            None => omitted_by.push((name, 1)),
        }
    }
    let partial = answer.partial
        || answer.unknown.is_some()
        || scope.unreadable > 0
        || matches!(scope.index, IndexState::Behind(_));
    let flagged = answer.counts.iter().any(|(name, n)| {
        *n > 0
            && matches!(
                *name,
                "stale"
                    | "unverifiable"
                    | "open"
                    | "failed"
                    | "unknown"
                    | "invalid"
                    | "disagree"
                    | "disorder"
            )
    });
    let exit = if answer.failed {
        1
    } else if answer.refused {
        3
    } else if partial || flagged {
        2
    } else {
        0
    };
    let trailer = Trailer {
        answer,
        scope,
        page: &page,
        omitted_by: &omitted_by,
        partial,
        query: &query,
        total: groups.len(),
    };
    let text = if frame.json {
        let rows: Vec<Value> = answer.rows[page.from..page.to]
            .iter()
            .map(row_json)
            .collect();
        let body = json!({"writfold_kb": 1, "verb": frame.verb, "rows": rows,
                          "notes": answer.notes, "trailer": trailer.json()});
        format!("{body}\n")
    } else {
        let mut text = String::new();
        for group in &groups[page.from..page.to] {
            text.push_str(group);
            text.push('\n');
        }
        for note in &answer.notes {
            text.push_str(&format!("# note: {note}\n"));
        }
        text.push_str(&trailer.line());
        text.push('\n');
        text
    };
    Ok(Output { text, exit })
}

struct Trailer<'a> {
    answer: &'a Answer,
    scope: &'a Scope<'a>,
    page: &'a Page,
    omitted_by: &'a [(&'a str, usize)],
    partial: bool,
    query: &'a str,
    total: usize,
}

impl Trailer<'_> {
    fn line(&self) -> String {
        let (s, p) = (self.scope, self.page);
        let mut out = format!(
            "# coverage={} records={}/{} shown={} omitted={}",
            if self.partial { "partial" } else { "complete" },
            s.examined,
            s.total,
            p.to - p.from,
            self.total - p.to
        );
        if self.omitted_by.len() > 1 || self.omitted_by.first().is_some_and(|(n, _)| *n != "rows") {
            let parts: Vec<String> = self
                .omitted_by
                .iter()
                .map(|(n, c)| format!("{n}:{c}"))
                .collect();
            out.push_str(&format!(" omitted_by={}", parts.join(",")));
        }
        if let Some(reason) = self.answer.unknown {
            out.push_str(&format!(" answer=unknown reason={reason}"));
        }
        for (name, count) in &self.answer.counts {
            out.push_str(&format!(" {name}={count}"));
        }
        if s.unreadable > 0 {
            out.push_str(&format!(" unreadable={}", s.unreadable));
        }
        if let Some(needs) = p.needs {
            out.push_str(&format!(" needs={needs}"));
        }
        out.push_str(&format!(
            " kb={} index={} cursor={} q={}",
            s.pin.label(),
            s.index.label(),
            p.cursor.as_deref().unwrap_or("none"),
            &self.query[..12]
        ));
        out
    }

    fn json(&self) -> Value {
        let (s, p) = (self.scope, self.page);
        let counts: Map<String, Value> = self
            .answer
            .counts
            .iter()
            .map(|(n, c)| ((*n).to_owned(), json!(c)))
            .collect();
        let omitted_by: Map<String, Value> = self
            .omitted_by
            .iter()
            .map(|(n, c)| ((*n).to_owned(), json!(c)))
            .collect();
        json!({"coverage": if self.partial { "partial" } else { "complete" },
               "records_examined": s.examined, "records_total": s.total,
               "shown": p.to - p.from, "omitted": self.total - p.to, "omitted_by": omitted_by,
               "answer": if self.answer.unknown.is_some() { "unknown" } else { "known" },
               "reason": self.answer.unknown, "counts": counts, "unreadable": s.unreadable,
               "needs": p.needs, "kb": s.pin.label(), "index": s.index.label(),
               "cursor": p.cursor, "q": &self.query[..12]})
    }
}

fn row_json(row: &Row) -> Value {
    let more: Vec<Value> = row
        .more
        .iter()
        .map(|m| json!({"label": m.label, "text": m.text, "path": m.path, "line": m.line, "flags": m.flags}))
        .collect();
    json!({"kind": row.kind, "id": row.id, "status": row.status, "text": row.text,
           "path": row.path, "line": row.line, "flags": row.flags, "more": more,
           "section": row.section})
}
