//! Knowledge-base mode in plain words: the inspector's sections, a cited
//! source's place and reason, relation phrases by edge end and the walk's
//! trailer. Records' titles, `why` and `decided` lines and field values
//! are authored and quoted whole; ids and flags never are.
use crate::cells::Glyphs;
use crate::kb_entry::{Entry, flag_marks, glyph};
use crate::lines::{Tone, more};
use crate::say::{Phrase, Piece, Told, align, quote, said};
use crate::say_shape::{locator, pair};
use crate::say_words::{MOVED, find, relation};
use crate::source::{Reply, field, flags};
use serde_json::Value;

/// A cited source as a place: `src/batch.rs:2-4 in demo-service`.
pub fn place(id: &str) -> String {
    match locator(id) {
        (path, Some(repo)) => format!("{path} in {repo}"),
        (path, None) => path.to_owned(),
    }
}

/// `X` of a `moved→X` flag.
pub fn moved_to<'a>(flags: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    flags.into_iter().find_map(|f| f.strip_prefix("moved→"))
}

/// Why a stale row is listed, in one word or two: the first flag with a
/// phrase (`moved` for `moved→X`), else the row's status quoted.
pub fn reason(entry: &Entry) -> Piece {
    let flags = entry.flags.iter().map(String::as_str);
    if moved_to(flags.clone()).is_some() {
        return said("moved");
    }
    flags
        .filter_map(|f| find(MOVED, f))
        .map(|w| said(w.short))
        .next()
        .unwrap_or_else(|| Piece::Quoted(entry.status.clone()))
}

/// The long form of each flag the entry carries, as sentences.
pub fn reasons(entry: &Entry) -> Vec<Phrase> {
    let mut out = Vec::new();
    for flag in &entry.flags {
        let text = match flag.strip_prefix("moved→") {
            Some(to) => format!("The cited lines are now at {to}."),
            None => match find(MOVED, flag) {
                Some(word) if word.long.ends_with('.') => word.long.to_owned(),
                _ => continue,
            },
        };
        out.push(Phrase::new(0, Tone::Bad, vec![said(text)]));
    }
    out
}

/// Title (or place) and status.
pub fn header(entry: &Entry) -> Phrase {
    let label = if entry.kind == "cite" {
        place(&entry.id)
    } else {
        entry.label.clone()
    };
    let mut pieces = vec![Piece::Quoted(label)];
    let status = if entry.kind == "cite" {
        reason(entry)
    } else {
        Piece::Quoted(entry.status.clone())
    };
    if !status.text().is_empty() {
        pieces.push(status);
    }
    Phrase::led(0, Tone::Current, pieces, 1)
}

/// The second line: the kind, a readable id, who decided.
pub fn identity(entry: &Entry, decided: Option<&str>) -> Phrase {
    if entry.kind == "cite" {
        let what = match entry.id.split_once(':').map(|(scheme, _)| scheme) {
            Some("code") => "a citation of code",
            Some("web") => "a citation of a web page",
            Some("record") => "a citation of a record",
            _ => "a citation",
        };
        return Phrase::new(0, Tone::Dim, vec![said(what)]);
    }
    let kind = if entry.kind.is_empty() {
        "record"
    } else {
        &entry.kind
    };
    let mut first = kind.to_owned();
    if let Some(id) = quote(&entry.id).filter(|_| entry.titled && !entry.id.contains(':')) {
        first = format!("{first} {}", id.text());
    }
    let mut pieces = vec![said(first)];
    pieces.extend(decided.and_then(|d| quote(&format!("decided {d}"))));
    Phrase::new(0, Tone::Dim, pieces)
}

/// A record's fields as `key   value`; the status is the header's.
pub fn fields(told: &mut Told, row: &Value) {
    let mut rows = Vec::new();
    for value in more(row).filter(|m| field(m, "label") == "field") {
        match pair(field(value, "text")).filter(|(_, v)| !v.contains('=')) {
            Some(("status", _)) => {}
            Some((key, value)) => rows.push(Phrase::led(
                0,
                Tone::Plain,
                vec![
                    said(key.replace('_', " ")),
                    Piece::Quoted(crate::say_shape::respelt(value)),
                ],
                1,
            )),
            None => told.withheld += 1,
        }
    }
    if !rows.is_empty() {
        align(&mut rows);
        told.lines.push(Phrase::blank());
        told.lines.extend(rows);
    }
}

pub fn heading(told: &mut Told, text: &str) {
    if told.lines.last().is_some_and(|p| !p.pieces.is_empty()) {
        told.lines.push(Phrase::blank());
    }
    told.lines
        .push(Phrase::new(0, Tone::Head, vec![said(text)]));
}

/// `Why`, each line whole; then the sources, with only the moved ones
/// named.
pub fn decision(told: &mut Told, row: &Value) {
    let whys: Vec<Piece> = more(row)
        .filter(|m| field(m, "label") == "why")
        .map(|m| Piece::Quoted(field(m, "text").into()))
        .collect();
    if !whys.is_empty() {
        heading(told, "Why");
        told.lines.extend(
            whys.into_iter()
                .map(|w| Phrase::new(2, Tone::Plain, vec![w])),
        );
    }
    let cites: Vec<&Value> = more(row).filter(|m| field(m, "label") == "cite").collect();
    if cites.is_empty() {
        return;
    }
    let flagged: Vec<&&Value> = cites
        .iter()
        .filter(|c| {
            let marks = flag_marks(flags(c));
            marks.moved || marks.unknown
        })
        .collect();
    let moved = flagged
        .iter()
        .filter(|c| flag_marks(flags(c)).moved)
        .count();
    let (n, of) = (moved, cites.len());
    let verb = if n == 1 { "has" } else { "have" };
    heading(told, &format!("Sources · {n} of {of} {verb} moved"));
    for cite in flagged {
        let id = field(cite, "text").split(' ').next().unwrap_or_default();
        let mark = if flag_marks(flags(cite)).moved {
            "!"
        } else {
            "?"
        };
        let lead = vec![said(mark), Piece::Quoted(place(id))];
        told.lines.push(Phrase::led(2, Tone::Bad, lead, 2));
        let detail = match moved_to(flags(cite)) {
            Some(to) => Some(said(format!("moved to {to}"))),
            None => flags(cite)
                .into_iter()
                .find_map(|f| find(MOVED, f))
                .map(|w| said(w.short)),
        };
        told.lines
            .extend(detail.map(|d| Phrase::new(4, Tone::Dim, vec![d])));
    }
}

/// One section of rows: glyph, title (or id), and the relation from the
/// row's side (`replaced by it`) when the row came by a link.
pub fn section(told: &mut Told, reply: &Reply, name: &str, heading_text: &str, glyphs: Glyphs) {
    let rows: Vec<&Value> = reply
        .rows
        .iter()
        .filter(|r| r.get("section").and_then(Value::as_str) == Some(name))
        .collect();
    if rows.is_empty() {
        return;
    }
    heading(told, heading_text);
    for row in rows {
        let label = Some(field(row, "text"))
            .filter(|t| !t.is_empty())
            .map_or_else(
                || Piece::Quoted(field(row, "id").into()),
                |t| Piece::Quoted(t.into()),
            );
        let status = field(row, "status");
        let mut pieces = vec![Piece::Mark(glyph(status, glyphs)), label];
        let via = more(row).find(|m| field(m, "label") == "via");
        match via.map(|v| field(v, "text").split(' ').next().unwrap_or_default()) {
            Some(rel) if name != "cited-by" => pieces.push(relation(rel, true)),
            Some(_) => {}
            None if !status.is_empty() => pieces.push(Piece::Quoted(status.into())),
            None => {}
        }
        told.lines.push(Phrase::led(2, Tone::Plain, pieces, 2));
    }
}

/// The walk's trailer in words, and whether coverage is partial.
pub fn trailer(reply: &Reply) -> (String, bool) {
    let shown = reply
        .trailer
        .get("shown")
        .and_then(Value::as_u64)
        .unwrap_or(reply.rows.len() as u64);
    let s = if shown == 1 { "" } else { "s" };
    let mut text = format!("{shown} record{s} reached");
    let skipped: Vec<String> = ["retired", "disputed", "inferred", "other"]
        .into_iter()
        .filter_map(|reason| {
            let n = reply
                .count(&format!("skipped_{reason}"))
                .filter(|n| *n > 0)?;
            let s = if n == 1 { "" } else { "s" };
            Some(format!("{n} {reason} link{s}"))
        })
        .collect();
    if !skipped.is_empty() {
        text.push_str(&format!(" · not followed: {}", skipped.join(", ")));
    }
    if let Some(frontier) = reply.count("frontier").filter(|n| *n > 0) {
        text.push_str(&format!(" · {frontier} more beyond two links"));
    }
    let partial = reply.trailer_text("coverage").as_deref() == Some("partial");
    (text, partial)
}
