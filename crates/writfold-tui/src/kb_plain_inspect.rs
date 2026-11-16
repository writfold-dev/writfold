//! The record inspector at the plain level: header and identity, the
//! entry's reasons, the decision or the record's fields, then the
//! sections (see also, linked from, cited by, runs that name it) and the
//! verbs' notes, quoted whole or counted.
use crate::app::App;
use crate::cells::Glyphs;
use crate::kb_entry::Entry;
use crate::kb_state::Inspected;
use crate::lines::Tone;
use crate::say::{Phrase, Piece, Told, said};
use crate::say_kb::{self, heading};
use crate::source::field;
use serde_json::Value;

/// The record inspector's phrases.
pub fn inspector(app: &App, entry: &Entry, record: &Inspected, glyphs: Glyphs) -> Told {
    let mut told = Told::default();
    let status = &record.status;
    let main = record.why.as_ref().and_then(|why| {
        why.rows
            .iter()
            .find(|r| r.get("section").is_none_or(Value::is_null))
    });
    let decided = main
        .filter(|r| field(r, "id") == entry.id)
        .and_then(|r| crate::lines::more(r).find(|m| field(m, "label") == "decided"))
        .map(|m| field(m, "text"));
    told.lines.push(say_kb::header(entry));
    told.lines.push(say_kb::identity(entry, decided));
    for reply in [Some(status), record.why.as_ref()].into_iter().flatten() {
        if let Some(error) = &reply.error {
            let text = format!("refused (exit {}): {error}", reply.exit);
            told.lines
                .push(Phrase::new(0, Tone::Bad, vec![Piece::Quoted(text)]));
        }
    }
    let reasons = say_kb::reasons(entry);
    if !reasons.is_empty() {
        told.lines.push(Phrase::blank());
        told.lines.extend(reasons);
    }
    match main {
        Some(row) if field(row, "id") != entry.id => {
            heading(&mut told, "why answers with its successor");
            told.lines.push(Phrase::new(
                2,
                Tone::Plain,
                vec![Piece::Quoted(field(row, "text").into())],
            ));
            say_kb::decision(&mut told, row);
        }
        Some(row) => say_kb::decision(&mut told, row),
        None => {
            if let Some(row) = status.rows.first().filter(|r| field(r, "id") == entry.id) {
                say_kb::fields(&mut told, row);
            }
        }
    }
    sections(app, &mut told, entry, record, glyphs);
    told
}

fn sections(app: &App, told: &mut Told, entry: &Entry, record: &Inspected, glyphs: Glyphs) {
    if let Some(why) = &record.why {
        say_kb::section(told, why, "secondary", "See also", glyphs);
    }
    say_kb::section(told, &record.status, "tracked-by", "Linked from", glyphs);
    if record.timeline.is_none() {
        let mut uses = record.status.clone();
        for row in &mut uses.rows {
            row["section"] = Value::from("cited-by");
        }
        say_kb::section(told, &uses, "cited-by", "Cited by", glyphs);
    }
    if let Some(timeline) = record.timeline.as_ref().filter(|_| entry.kind != "run") {
        let mut seen: Vec<&str> = Vec::new();
        for row in timeline
            .rows
            .iter()
            .filter(|r| field(r, "kind") == "checkpoint")
        {
            let id = field(row, "id");
            if seen.contains(&id) {
                continue;
            }
            if seen.is_empty() {
                heading(told, "Runs that name it");
            }
            seen.push(id);
            let title = app.runs.rows.iter().find(|r| field(r, "id") == id);
            let title = title.map_or(id, |r| field(r, "text"));
            told.lines.push(Phrase::new(
                2,
                Tone::Plain,
                vec![Piece::Quoted(title.into())],
            ));
        }
    }
    for reply in [Some(&record.status), record.why.as_ref()]
        .into_iter()
        .flatten()
    {
        if let Some(reason) = reply.trailer_text("reason") {
            told.lines.push(Phrase::blank());
            let pieces = vec![said("answer unknown"), Piece::Quoted(reason)];
            told.lines.push(Phrase::new(0, Tone::Warn, pieces));
        }
        let mut notes = Told::default();
        crate::say_row::notes(&mut notes, reply, &[]);
        told.lines.extend(notes.lines);
        told.notes += notes.notes;
    }
}
