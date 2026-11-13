//! The rows `gate status`, `gate cover` and `resume` show for a checklist
//! gate, and its `gate lines` lines: one `item:` line per declared item
//! with its standing and the row behind it (and a `looked:` line under it
//! when the run holds observations made for it), and a `next:` line naming
//! the first required item still owed.
use super::View;
use super::judge::Entry;
use crate::emit::{More, Row};
use crate::gate::statement;
use crate::gate::status::{Judged, shown_path};
use crate::gate::subject::Subject;
use writfold_core::checklist::Item;
use writfold_core::gate::Standing;

fn short(text: &str) -> &str {
    let text = text.strip_prefix("sha256:").unwrap_or(text);
    text.get(..12).unwrap_or(text)
}

fn more(label: &'static str, text: String) -> More {
    More {
        label,
        text,
        ..More::default()
    }
}

/// `errors [supported] 2 references by agent`, and its row's path.
fn item_text(e: &Entry<'_>) -> (String, Option<String>) {
    let mut text = format!("{} [{}]", e.spec.id, e.item.name());
    if !e.spec.required {
        text.push_str(" (optional)");
    }
    match e.row {
        None => {
            text.push(' ');
            text.push_str(&e.spec.text);
            (text, None)
        }
        Some((f, c)) => {
            let said = match (&c.reason, c.evidence.len()) {
                (Some(reason), _) => reason.clone(),
                (None, 1) => "1 reference".into(),
                (None, n) => format!("{n} references"),
            };
            text.push_str(&format!(" {said} by {}", c.by));
            if let Some(why) = &e.why {
                text.push_str(&format!("; {why}"));
            }
            (text, Some(f.path.clone()))
        }
    }
}

fn item_line(e: &Entry<'_>) -> More {
    let (text, path) = item_text(e);
    let flags = match e.item {
        Item::Stale | Item::Unsupported => vec![e.item.name().to_owned()],
        _ => Vec::new(),
    };
    More {
        label: "item",
        text,
        line: path.as_ref().map(|_| 1),
        path,
        flags,
    }
}

/// The command that would cover the first required item still owed.
pub fn next(id: &str, view: &View<'_>) -> Option<String> {
    let first = view.rollup.first_owed.and_then(|i| view.entries.get(i))?;
    Some(format!(
        "writfold kb gate cover {id} {} supported REF… --by WHO",
        first.spec.id
    ))
}

/// The row of one checklist gate.
pub fn row(subject: &Subject, judged: &Judged<'_>, view: &View<'_>) -> Row {
    let d = &judged.definition;
    let (covered, required) = view.counts();
    let mut row = Row {
        kind: "gate",
        id: d.id.clone(),
        status: Some(judged.standing.name().into()),
        section: Some("gates"),
        text: format!(
            "checklist {covered}/{required} required covered at {}@{} scope={}",
            subject.name,
            short(&view.commit),
            short(&view.content)
        ),
        path: Some(shown_path(d, subject)),
        line: Some(1),
        ..Row::default()
    };
    for e in &view.entries {
        row.more.push(item_line(e));
        row.more.extend(e.looked.clone());
    }
    if let Some(read) = crate::gate::status::read_line(d, &subject.name) {
        row.more.push(read);
    }
    if judged.standing == Standing::Waived
        && let Some((f, s)) = judged.statements.waived
    {
        row.more.push(statement::more(f, s));
    }
    row.flags.push(judged.standing.name().into());
    if judged.standing.met() {
        return row;
    }
    if judged.statements.waiver_moved {
        row.more.push(more(
            "why",
            "a waiver exists, but for other content or another definition".into(),
        ));
    }
    if let Some(next) = next(&d.id, view) {
        row.more.push(more("next", next));
    }
    row
}

/// The `gate lines` lines of one checklist gate: the gate, then one line
/// per item.
pub fn line(judged: &Judged<'_>, view: &View<'_>, width: usize) -> String {
    let id = format!("{}:", judged.definition.id);
    let (covered, required) = view.counts();
    let standing = judged.standing.name();
    let mut out = match (judged.standing, judged.statements.waived) {
        (Standing::Waived, Some((f, s))) => format!(
            "{id:<w$} {standing} by {}: {} {}\n",
            s.by,
            s.text,
            f.path,
            w = width + 1
        ),
        _ => format!(
            "{id:<w$} {standing} checklist {covered}/{required} required covered\n",
            w = width + 1
        ),
    };
    for e in &view.entries {
        let by = e
            .row
            .map_or(String::new(), |(f, c)| format!(" by {} {}", c.by, f.path));
        out.push_str(&format!(
            "{:<w$}   {} {}{by}\n",
            "",
            e.spec.id,
            e.item.name(),
            w = width + 1
        ));
    }
    out
}
