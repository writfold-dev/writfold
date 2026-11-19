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
    let choices = (e.spec.required && !e.covers())
        .then(|| super::allowed::choices(&e.spec, e.restricted))
        .flatten()
        .map_or_else(String::new, |c| format!(" ({c})"));
    match e.row {
        None => {
            text.push(' ');
            text.push_str(&e.spec.text);
            text.push_str(&choices);
            (text, None)
        }
        Some((f, c)) if c.outcome == writfold_core::checklist::Outcome::NoneFound => {
            let looks = match c.evidence.len() {
                1 => "1 look found nothing".to_owned(),
                n => format!("{n} looks found nothing"),
            };
            let reason = c.reason.clone().unwrap_or_default();
            text.push_str(&format!(" {looks}: {reason} by {}", c.by));
            if let Some(why) = &e.why {
                text.push_str(&format!("; {why}"));
            }
            text.push_str(&choices);
            (text, Some(f.path.clone()))
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
            text.push_str(&choices);
            (text, Some(f.path.clone()))
        }
    }
}

pub(crate) fn item_line(e: &Entry<'_>) -> More {
    let (text, path) = item_text(e);
    let mut flags = match e.item {
        Item::Stale | Item::Unsupported | Item::NoneFound => vec![e.item.name().to_owned()],
        _ => Vec::new(),
    };
    if e.not_accepted() {
        flags.push("not-accepted".to_owned());
    }
    if e.recorded > 0 {
        flags.push("recorded".to_owned());
    }
    for flag in e.bound.iter().flat_map(|n| &n.flags) {
        if !flags.contains(flag) {
            flags.push(flag.clone());
        }
    }
    let mut extra = Vec::new();
    if e.restricted {
        let allowed = super::allowed::choices(&e.spec, true).unwrap_or_default();
        let words: Vec<&str> = allowed.split(" or ").collect();
        extra.push(("allowed", serde_json::json!(words)));
    }
    if e.not_accepted() {
        extra.push(("accepted", serde_json::json!(false)));
    }
    More {
        extra,
        status: Some(e.item.name().into()),
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
        row.more.extend(e.bound.iter().cloned());
        row.more.extend(e.looked.clone());
    }
    crate::gate::policy::json_keys(&mut row, judged);
    if let Some(read) = crate::gate::status::read_line(d, &subject.name) {
        row.more.push(read);
    }
    if judged.standing == Standing::Waived
        && let Some((f, s)) = judged.statements.waived
    {
        row.more.push(statement::more(f, s));
    }
    row.flags.push(judged.standing.name().into());
    if judged.met() {
        return row;
    }
    if judged.not_accepted() {
        row.flags.push("not-accepted".into());
        let shown = super::super::status::shown_path(&judged.definition, subject);
        let why = super::super::policy::why_not_accepted(
            &judged.definition,
            judged.standing.name(),
            &shown,
        );
        row.more.push(more("why", why));
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
        let said = match (e.item, e.row) {
            (Item::NoneFound, Some((_, c))) => {
                let n = c.evidence.len();
                let looks = if n == 1 {
                    "1 look".to_owned()
                } else {
                    format!("{n} looks")
                };
                format!(
                    ": none-found ({looks}): {}",
                    c.reason.clone().unwrap_or_default()
                )
            }
            _ => format!(" {}", e.item.name()),
        };
        out.push_str(&format!(
            "{:<w$}   {}{said}{by}\n",
            "",
            e.spec.id,
            w = width + 1
        ));
    }
    out
}
