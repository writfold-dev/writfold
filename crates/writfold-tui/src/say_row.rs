//! The run text views in plain words: Timeline, Inbox and Guards, and the
//! generic row form (glyph, title, `kind · state`, readable continuations)
//! for Stale, Brief, Resume and the KB status view. Notes are replaced by
//! a fixed sentence only when the verb's own wording is recognised.
use crate::cells::Glyphs;
use crate::lines::{self, Tone, more};
use crate::say::{Phrase, Piece, Told, align, quote, said};
use crate::say_shape::{event, pair, when};
use crate::say_words::{MOVED, find};
use crate::source::{Reply, field, flags};
use serde_json::Value;

/// A row's name as another view shows it (a step on the map, a run in
/// the runs list); `None` when the lookup fails.
pub type Names<'a> = &'a dyn Fn(&Value) -> Option<String>;

/// The verb's notes: a recognised note becomes its sentence, a readable
/// one is quoted whole, any other is counted.
pub fn notes(told: &mut Told, answer: &Reply, known: &[(&str, &str)]) {
    let mut out = Vec::new();
    for note in &answer.notes {
        let piece = known
            .iter()
            .find(|(start, _)| note.starts_with(start))
            .map(|(_, sentence)| said(*sentence))
            .or_else(|| quote(note));
        match piece {
            Some(piece) => out.push(Phrase::new(0, Tone::Dim, vec![piece])),
            None => told.notes += 1,
        }
    }
    if !out.is_empty() {
        told.lines.push(Phrase::blank());
        told.lines.extend(out);
    }
}

fn column(at: &str) -> Piece {
    said(format!("{at:<18}"))
}

/// One Timeline row: when, what, and its text or name.
fn timeline_row(row: &Value, names: Names<'_>, told: &mut Told) -> Option<Phrase> {
    let kind = field(row, "kind");
    let (at, what, rest): (String, String, Option<Piece>) = match kind {
        "plan" => {
            let (at, rest) = event(row)?;
            let what = match field(row, "status") {
                "spliced" => "plan added".to_owned(),
                status => format!("step {status}"),
            };
            let name = names(row).unwrap_or_else(|| field(row, "id").to_owned());
            let mut text = name;
            if let Some(more) = quote(rest) {
                text = format!("{text} · {}", more.text());
            }
            (at, what, Some(Piece::Quoted(text)))
        }
        "checkpoint" => {
            let (at, _) = when(field(row, "id"))?;
            (
                at,
                kind.into(),
                Some(Piece::Quoted(field(row, "text").into())),
            )
        }
        _ => {
            let (at, rest) = when(field(row, "text"))?;
            // `created TITLE`: a verb word, then the record's own title.
            let verb = rest.split(' ').next().unwrap_or_default();
            let authored = !verb.is_empty() && verb.bytes().all(|b| b.is_ascii_lowercase());
            let text = if authored {
                Some(Piece::Quoted(rest.to_owned()))
            } else {
                quote(rest)
            };
            if text.is_none() {
                told.withheld += 1;
            }
            (at, kind.into(), text)
        }
    };
    let mut pieces = vec![column(&at), said(what)];
    pieces.extend(rest);
    Some(Phrase::led(0, lines::tone(row), pieces, 2))
}

pub fn timeline(answer: &Reply, names: Names<'_>) -> Told {
    let mut told = Told::default();
    let mut rows = Vec::new();
    for row in &answer.rows {
        match timeline_row(row, names, &mut told) {
            Some(phrase) => rows.push(phrase),
            None => told.withheld += 1,
        }
    }
    align(&mut rows);
    told.lines = rows;
    notes(&mut told, answer, &[]);
    told.lines.push(Phrase::blank());
    told.lines.push(Phrase::new(
        0,
        Tone::Dim,
        vec![said("Times are UTC, as recorded.")],
    ));
    told
}

/// The generic row form: glyph, title (or name), `kind · state`, then
/// the continuations that read as words.
pub fn generic(answer: &Reply, glyphs: Glyphs, names: Names<'_>) -> Told {
    let mut told = Told::default();
    for row in &answer.rows {
        let status = field(row, "status");
        let state = flags(row)
            .into_iter()
            .find_map(|f| find(MOVED, f))
            .map_or_else(|| Piece::Quoted(status.to_owned()), |w| said(w.short));
        let id = field(row, "id");
        // A gate's, step's, guard's or citation's text is the verb's
        // account, not a title: those rows are named by their id.
        let titled = !matches!(
            field(row, "kind"),
            "gate" | "plan" | "step" | "guard" | "cite"
        );
        let title = names(row)
            .map(Piece::Quoted)
            .or_else(|| quote(field(row, "text")).filter(|_| titled))
            .or_else(|| {
                (field(row, "kind") == "cite").then(|| Piece::Quoted(crate::say_kb::place(id)))
            })
            .or_else(|| quote(id))
            .unwrap_or_else(|| {
                told.withheld += 1;
                said("untitled")
            });
        let mark = Piece::Mark(crate::kb_entry::glyph(status, glyphs));
        let mut pieces = vec![mark, title, said(field(row, "kind"))];
        if !status.is_empty() {
            pieces.push(state);
        }
        told.lines.push(Phrase::led(0, lines::tone(row), pieces, 2));
        continuations(&mut told, row);
    }
    notes(&mut told, answer, &[]);
    told
}

fn continuations(told: &mut Told, row: &Value) {
    let mut fields = Vec::new();
    for value in more(row) {
        let text = field(value, "text");
        match field(value, "label") {
            "next" | "read" | "confirm" | "accept" => {}
            "field" => match pair(text).filter(|(_, v)| !v.contains('=')) {
                Some((key, value)) => fields.push(Phrase::led(
                    4,
                    Tone::Dim,
                    vec![
                        said(key.replace('_', " ")),
                        Piece::Quoted(crate::say_shape::respelt(value)),
                    ],
                    1,
                )),
                None => told.withheld += 1,
            },
            _ => match quote(text) {
                Some(piece) => told.lines.push(Phrase::new(4, Tone::Dim, vec![piece])),
                None => told.withheld += 1,
            },
        }
    }
    align(&mut fields);
    told.lines.extend(fields);
}
