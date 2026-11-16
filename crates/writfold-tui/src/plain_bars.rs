//! What the bars say at the plain level: positions instead of ids, badge
//! words instead of marks, and a footer that speaks only when something
//! is not the normal case. The exact level keeps `bar_content`'s words.
use crate::app::{App, View};
use crate::bars::{Role, Segment, Unit, chip, fit, segment, total};
use crate::cells::Glyphs;
use crate::lines::Tone;
use crate::style::Chip;

/// Header units after the title: `current run`, the project, and a
/// subject warning only when no repository is judged.
pub fn header(app: &App, row: &serde_json::Value, left: &mut Vec<Unit>) {
    let status = crate::source::field(row, "status");
    if !status.is_empty() {
        left.push(Unit::new(vec![chip(app.look, status, Chip::Plain)], 0));
    }
    if crate::source::flags(row).contains(&"current") {
        left.push(Unit::new(
            vec![chip(app.look, "current run", Chip::Accent)],
            0,
        ));
    }
    if let Some(project) = app.project() {
        left.push(Unit::new(vec![chip(app.look, &project, Chip::Plain)], 2));
    }
    let warning = match &app.stamps.subject {
        Ok(Some(_)) => None,
        Ok(None) => Some("no repository: gates not judged"),
        Err(_) => Some("repository unreadable"),
    };
    if let Some(text) = warning {
        left.push(Unit::one(text, Role::Badge(Tone::Warn), 3));
    }
}

/// `run 1 of 3`.
pub fn position(app: &App) -> String {
    format!("run {} of {}", app.selected + 1, app.order.len())
}

/// A tab's badge: the long words, or the short number (`short`).
pub fn badge(app: &App, view: View, short: bool) -> Vec<Segment> {
    let Some(answer) = app.answer(view).filter(|a| a.error.is_none()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut push = |n: u64, words: &str, tone: Tone| {
        if n > 0 {
            let text = if short {
                format!(" {n}")
            } else {
                format!(" {n} {words}")
            };
            out.push(segment(text, Role::Badge(tone)));
        }
    };
    match view {
        View::Gates => {
            if let Some(n) = crate::say_gate::not_met(answer) {
                push(n, "not met", crate::say_gate::badge_tone(answer));
            }
        }
        View::Inbox => {
            let n = crate::side::inbox_total(answer) as u64;
            push(n, "pending", Tone::Done);
        }
        View::Guards => {
            push(answer.count("deny").unwrap_or(0), "would block", Tone::Bad);
            push(answer.count("ask").unwrap_or(0), "would ask", Tone::Warn);
        }
        _ => {}
    }
    out
}

/// The footer's facts: uncommitted KB changes and an index that is not
/// fresh; nothing at all in the normal case.
pub fn footer(kb: Option<String>, index: Option<String>, left: &mut Vec<Unit>) {
    let dirty = kb.as_deref().and_then(crate::say_shape::stamp);
    if let Some(n) = dirty.filter(|n| *n > 0) {
        let s = if n == 1 { "" } else { "s" };
        left.push(Unit::one(
            format!("KB has {n} uncommitted change{s}"),
            Role::Muted,
            2,
        ));
    }
    let index = index.unwrap_or_else(|| "unknown".into());
    if index != "fresh" {
        left.push(Unit::one(
            format!("index {index}"),
            Role::Badge(Tone::Warn),
            3,
        ));
    }
}

/// The active tab and the tabs with badges; plain badges shorten from the
/// right, one tab at a time, before `fit` drops a tab.
pub(crate) fn narrow_tabs(
    app: &App,
    forms: &[Vec<(View, (Unit, bool))>],
    room: usize,
    glyphs: Glyphs,
) -> Vec<Segment> {
    let index = View::ALL
        .iter()
        .position(|v| *v == app.view)
        .map_or(1, |i| i + 1);
    let right = vec![
        crate::kb_bars::mode_unit(false),
        Unit::one(format!("{index}/{}", View::ALL.len()), Role::Muted, 0),
    ];
    let pick = |all: &[(View, (Unit, bool))]| -> Vec<(View, Unit)> {
        all.iter()
            .filter(|(view, (_, marked))| *view == app.view || *marked)
            .map(|(view, (unit, _))| {
                let mut unit = unit.clone();
                unit.drop = u8::from(*view != app.view);
                (*view, unit)
            })
            .collect()
    };
    let long = forms.first().map(|f| pick(f)).unwrap_or_default();
    let short = forms.get(1).map(|f| pick(f));
    let mut left: Vec<(View, Unit)> = long;
    if let Some(short) = short {
        let mut at = left.len();
        let units = |l: &[(View, Unit)]| l.iter().map(|(_, u)| u.clone()).collect::<Vec<_>>();
        while at > 0 && total(&units(&left), &right, 4) > room {
            at -= 1;
            left[at] = short[at].clone();
        }
    }
    let left = left.into_iter().map(|(_, unit)| unit).collect();
    fit((left, right), 4, room, glyphs)
}
