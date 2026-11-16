//! Knowledge-base mode at the plain level, from the cached answers: the
//! record inspector, the status view, the lists' words and captions, and
//! titles for ids that a list or the projects already name.
use crate::app::App;
use crate::cells::Glyphs;
use crate::kb_entry::{Entry, marks};
use crate::kb_state::{KbView, Scope};
use crate::lines::Tone;
use crate::say::{Phrase, Piece, Told};
use crate::say_kb;
use crate::source::{Reply, field};
use serde_json::Value;

/// A record's title where one is on hand, else its id.
pub fn title_of(app: &App, id: &str) -> String {
    let projects = app.kb_projects();
    if let Some(row) = projects.iter().find(|r| field(r, "id") == id) {
        return field(row, "text").to_owned();
    }
    let entries = app.kb_entries();
    if let Some(entry) = entries.iter().find(|e| !e.header && e.id == id && e.titled) {
        return entry.label.clone();
    }
    let listed = app.kb.answers.values().flat_map(|r| &r.rows);
    for row in listed.filter(|r| field(r, "id") == id) {
        let title = Some(field(row, "title")).filter(|t| !t.is_empty());
        if let Some(title) = title {
            return title.to_owned();
        }
    }
    app.kb
        .records
        .get(id)
        .and_then(|r| r.status.rows.first())
        .filter(|r| field(r, "id") == id && !field(r, "text").is_empty())
        .map_or_else(|| id.to_owned(), |r| field(r, "text").to_owned())
}

/// The list's entries with plain words: titles for headers, a cited
/// source's place, the record's status (tracker), the relation from the
/// row's side (impact), the reason (stale).
pub fn entries(app: &App, entries: Vec<Entry>) -> Vec<Entry> {
    let view = app.kb.view;
    entries
        .into_iter()
        .map(|mut e| {
            if e.header {
                if matches!(view, KbView::Tracker | KbView::Impact) {
                    e.label = title_of(app, &e.label);
                }
                return e;
            }
            match view {
                KbView::Tracker => e.word = e.status.clone(),
                KbView::Impact => {
                    e.word = crate::say_words::relation(&e.word, e.from).text().into()
                }
                KbView::Stale => {
                    e.word = say_kb::reason(&e).text().into();
                    // A typed record id (`inv:…`) reads better as its
                    // title, where an answer on hand has one.
                    let title = title_of(app, &e.id);
                    if !e.titled && e.kind != "cite" && e.id.contains(':') && title != e.id {
                        e.label = title;
                        e.titled = true;
                    } else if !e.titled {
                        e.label = crate::say_shape::locator(&e.id).0.to_owned();
                    }
                }
                _ => {}
            }
            e.origin.clear();
            e
        })
        .collect()
}

/// The mark legend, for the marks the list carries.
pub fn legend(app: &App, entries: &[Entry]) -> String {
    let found: Vec<_> = entries
        .iter()
        .filter(|e| !e.header)
        .map(|e| marks(e, app.kb.stale.as_ref()))
        .collect();
    let mut parts = Vec::new();
    if found.iter().any(|m| m.moved) {
        parts.push("! cites a source that has moved");
    }
    if found.iter().any(|m| m.unknown) {
        parts.push("? has a source not checked");
    }
    parts.join("   ")
}

/// The words above and below a list: the impact direction, a caption,
/// the walk's trailer.
pub fn around(
    app: &App,
    reply: Option<&Reply>,
    legend: &str,
) -> (Vec<String>, Vec<(String, Tone)>) {
    let name = app.kb_scope().name().unwrap_or("the KB").to_owned();
    let subject = app
        .kb_subject()
        .map(|s| title_of(app, &s))
        .unwrap_or_default();
    let walk = reply.map(|r| {
        let (text, partial) = say_kb::trailer(r);
        (text, if partial { Tone::Warn } else { Tone::Dim })
    });
    let with = |text: String| {
        if legend.is_empty() {
            text
        } else {
            format!("{text}   {legend}")
        }
    };
    let walked = walk.map(|(text, tone)| (with(text), tone));
    let dim = |text: String| (text, Tone::Dim);
    let mut top = Vec::new();
    let mut bottom = Vec::new();
    match (app.kb.view, app.kb_scope()) {
        (KbView::Impact, _) => {
            top.push(match app.kb.direction.name() {
                "in" => format!("what points at {subject}"),
                "out" => format!("what {subject} points at"),
                _ => format!("everything linked to {subject}"),
            });
            top.push(String::new());
            bottom.extend(walked);
        }
        (KbView::Tracker, Scope::Project(_)) => {
            bottom.push(dim(
                "tickets, investigations and signatures within two links".into(),
            ));
            bottom.extend(walked);
        }
        (KbView::Tracker, Scope::All) => {
            bottom.push(dim(with(
                "every ticket, investigation and signature".into(),
            )));
        }
        (KbView::Why, Scope::Project(_)) => bottom.push(dim(with(format!("{name}'s decisions")))),
        (KbView::Why, Scope::All) => bottom.push(dim(with("every decision in the KB".into()))),
        (KbView::Stale, _) => {
            let count = |key| reply.and_then(|r| r.count(key)).unwrap_or(0);
            bottom.push(dim(format!(
                "{} need a look · {} checked · from the last recorded check; nothing was re-run",
                count("stale"),
                count("checked")
            )));
        }
        (KbView::Status, _) => {}
    }
    (top, bottom)
}

/// The status view: the project's own row and fields, then each row in
/// the generic form.
pub fn status(app: &App, reply: &Reply, glyphs: Glyphs) -> Told {
    let (head, rest): (Vec<&Value>, Vec<&Value>) = reply
        .rows
        .iter()
        .partition(|r| field(r, "section") == "header");
    let mut told = Told::default();
    for row in head {
        let pieces = vec![
            Piece::Quoted(field(row, "text").into()),
            Piece::Quoted(field(row, "status").into()),
        ];
        told.lines.push(Phrase::led(0, Tone::Head, pieces, 1));
        say_kb::fields(&mut told, row);
        if told.lines.get(1).is_some_and(|p| p.pieces.is_empty()) {
            told.lines.remove(1);
        }
        for line in told.lines.iter_mut().skip(1) {
            line.indent = 2;
        }
        crate::say::align(&mut told.lines[1..]);
        told.lines.push(Phrase::blank());
    }
    let others = Reply {
        rows: rest.into_iter().cloned().collect(),
        ..reply.clone()
    };
    let names = |row: &Value| -> Option<String> {
        let run = app
            .runs
            .rows
            .iter()
            .find(|r| field(r, "id") == field(row, "id"))?;
        (field(row, "kind") == "run").then(|| field(run, "text").to_owned())
    };
    let body = crate::say_row::generic(&others, glyphs, &names);
    told.lines.extend(body.lines);
    told.withheld += body.withheld;
    told.notes += body.notes;
    told
}
