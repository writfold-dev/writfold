//! What the three bars say. Every word is a verb field, a verb count or a
//! key; badges appear only for non-zero counts from their own verb.
use crate::app::{App, Detail, Focus, View};
use crate::bars::{Role, Segment, Unit, chip, fit, segment, total};
use crate::cells::Glyphs;
use crate::lines::Tone;
use crate::map_view::glyphs;
use crate::source::{field, flags};
use crate::style::Chip;

/// Title, state chips, project and subject on the left; the run id (or
/// `run N/M` when narrow) on the right. Subject, then project, drop first.
pub fn title(app: &App, width: u16, narrow: bool) -> Vec<Segment> {
    let room = usize::from(width);
    let Some(row) = app.row() else {
        let left = vec![Unit::one(
            "writfold watch — no run selected",
            Role::Title,
            0,
        )];
        return fit((left, Vec::new()), 3, room, glyphs(app));
    };
    let mut left = vec![Unit::one(field(row, "text"), Role::Title, 0)];
    if app.level == Detail::Plain {
        crate::plain_bars::header(app, row, &mut left);
        let right = vec![Unit::one(crate::plain_bars::position(app), Role::Muted, 1)];
        return fit((left, right), 3, room, glyphs(app));
    }
    let status = field(row, "status");
    if !status.is_empty() {
        left.push(Unit::new(vec![chip(app.look, status, Chip::Plain)], 0));
    }
    if flags(row).contains(&"current") {
        left.push(Unit::new(vec![chip(app.look, "current", Chip::Accent)], 0));
    }
    if let Some(project) = app.project() {
        left.push(Unit::new(vec![chip(app.look, &project, Chip::Plain)], 2));
    }
    let subject = match &app.stamps.subject {
        Ok(Some(subject)) => subject.name.clone(),
        Ok(None) => "none".into(),
        Err(_) => "error".into(),
    };
    left.push(Unit::one(format!("subject {subject}"), Role::Muted, 3));
    let right = if narrow {
        format!("run {}/{}", app.selected + 1, app.order.len())
    } else {
        field(row, "id").to_owned()
    };
    let right = vec![Unit::one(right, Role::Muted, 1)];
    fit((left, right), 3, room, glyphs(app))
}

fn badge(text: String, tone: Tone) -> Segment {
    segment(format!(" {text}"), Role::Badge(tone))
}

/// Non-zero counts from the tab's own verb, in the verb's words.
fn badges(app: &App, view: View, glyphs: Glyphs) -> Vec<Segment> {
    let Some(answer) = app.answer(view).filter(|a| a.error.is_none()) else {
        return Vec::new();
    };
    let count = |name| answer.count(name).filter(|n| *n > 0);
    let ascii = glyphs == Glyphs::Ascii;
    let marks: &[(&str, &str, &str, Tone)] = match view {
        View::Gates => &[
            ("stale", "!", "!", Tone::Bad),
            ("failed", "×", "x", Tone::Bad),
            ("open", "○", "o", Tone::Warn),
            ("unknown", "?", "?", Tone::Warn),
        ],
        View::Guards => &[
            ("deny", "deny ", "deny ", Tone::Bad),
            ("ask", "ask ", "ask ", Tone::Warn),
        ],
        View::Inbox => {
            let n = crate::side::inbox_total(answer);
            return (n > 0)
                .then(|| badge(n.to_string(), Tone::Done))
                .into_iter()
                .collect();
        }
        _ => &[],
    };
    marks
        .iter()
        .filter_map(|(name, unicode, plain, tone)| {
            let mark = if ascii { plain } else { unicode };
            count(name).map(|n| badge(format!("{mark}{n}"), *tone))
        })
        .collect()
}

pub(crate) fn tab(app: &App, view: View, glyphs: Glyphs, short: bool) -> (Unit, bool) {
    let name = format!("{} {}", view.digit(), view.name());
    let mut parts = if view == app.view {
        vec![chip(app.look, &name, Chip::Active)]
    } else {
        vec![
            segment(view.digit().to_string(), Role::Digit),
            segment(format!(" {}", view.name()), Role::Tab),
        ]
    };
    let badges = if app.level == Detail::Plain {
        crate::plain_bars::badge(app, view, short)
    } else {
        badges(app, view, glyphs)
    };
    let marked = !badges.is_empty();
    parts.extend(badges);
    (Unit::new(parts, 0), marked)
}

/// All eight tabs four cells apart (closer when needed; plain badges
/// shorten to their number first). Narrow, or when even that does not
/// fit: the active tab, the tabs with badges, `i/8`.
pub fn tabs(app: &App, width: u16, narrow: bool) -> Vec<Segment> {
    let glyphs = glyphs(app);
    let room = usize::from(width);
    let row = |short: bool| -> Vec<_> {
        View::ALL
            .iter()
            .map(|v| (*v, tab(app, *v, glyphs, short)))
            .collect()
    };
    let forms = if app.level == Detail::Plain {
        vec![row(false), row(true)]
    } else {
        vec![row(false)]
    };
    // Plain keeps the mode unit while a shorter badge can make room.
    let with_mode = app.level == Detail::Plain;
    let mode = vec![crate::kb_bars::mode_unit(false)];
    for keep in [with_mode, false].into_iter().filter(|_| !narrow) {
        for all in &forms {
            for gap in [4, 3, 2] {
                let units: Vec<_> = all.iter().map(|(_, (u, _))| u.clone()).collect();
                let right: &[Unit] = if keep { &mode } else { &[] };
                if total(&units, right, gap) <= room {
                    return fit((units, mode.clone()), gap, room, glyphs);
                }
            }
        }
    }
    crate::plain_bars::narrow_tabs(app, &forms, room, glyphs)
}

fn hints(app: &App, narrow: bool) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&str, String)> = if app.focus == Focus::Runs || app.runs_view {
        let target = if app.view == View::Progress {
            "plan"
        } else {
            app.view.name()
        };
        vec![("j/k", "run".into()), ("l", target.into())]
    } else if app.focus == Focus::Inspector && app.map_visible() {
        vec![
            ("j/k", "scroll".into()),
            ("h", "back".into()),
            ("d", "detail".into()),
        ]
    } else if app.map_visible() {
        vec![
            ("j/k", "step".into()),
            ("l", "inspect".into()),
            ("d", "detail".into()),
        ]
    } else if app.level == Detail::Plain {
        vec![
            ("j/k", "scroll".into()),
            ("Tab", "view".into()),
            ("d", "detail".into()),
        ]
    } else {
        vec![("j/k", "scroll".into()), ("Tab", "view".into())]
    };
    if narrow {
        out.remove(0);
    }
    out.extend([("?", "help".into()), ("q", "quit".into())]);
    out
}

/// Read-only, KB stamp, index and ask time on the left (dropped from the
/// end); at most three contextual hints and `? help  q quit` on the right.
pub fn footer(app: &App, width: u16, narrow: bool) -> Vec<Segment> {
    let right = hints(app, narrow)
        .into_iter()
        .map(|(key, label)| {
            let parts = vec![
                segment(key, Role::Key),
                segment(format!(" {label}"), Role::Muted),
            ];
            Unit::new(parts, 0)
        })
        .collect();
    let at = |key| {
        app.runs
            .trailer_text(key)
            .unwrap_or_else(|| "unknown".into())
    };
    let left = if let Some(notice) = &app.notice {
        vec![Unit::one(notice.clone(), Role::Notice, 0)]
    } else if app.level == Detail::Plain {
        let mut left = vec![Unit::new(vec![chip(app.look, "read-only", Chip::Plain)], 0)];
        let (kb, index) = (app.runs.trailer_text("kb"), app.runs.trailer_text("index"));
        crate::plain_bars::footer(kb, index, &mut left);
        left
    } else {
        let index = at("index");
        let tone = if index == "fresh" {
            Role::Muted
        } else {
            Role::Badge(Tone::Warn)
        };
        let mut left = vec![
            Unit::new(vec![chip(app.look, "read-only", Chip::Plain)], 0),
            Unit::one(format!("kb {}", at("kb")), Role::Muted, 2),
            Unit::one(format!("index {index}"), tone, 3),
            Unit::one(format!("asked {}", app.asked), Role::Muted, 4),
        ];
        if app.level == Detail::Exact {
            let subject = crate::poll::subject_text(&app.stamps.subject).replacen('=', " ", 1);
            left.push(Unit::one(subject, Role::Muted, 5));
        }
        left
    };
    fit((left, right), 3, usize::from(width), glyphs(app))
}
