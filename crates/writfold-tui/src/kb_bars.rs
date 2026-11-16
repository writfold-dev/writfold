//! Knowledge-base mode's three bars, built from the same units, chips and
//! drop rules as the run mode's; and the mode switch both tab rows end in.
use crate::app::{App, Detail, Focus};
use crate::bars::{self, Role, Segment, Unit, chip, fit, segment, total};
use crate::kb_state::{KbView, Scope};
use crate::layout::Areas;
use crate::lines::Tone;
use crate::map_view::glyphs;
use crate::source::field;
use crate::style::{self, Chip, Look};
use ratatui::Frame;
use ratatui::widgets::Paragraph;

/// `m kb` in run mode, `m runs` in KB mode: the first unit a short tab
/// row drops.
pub fn mode_unit(kb: bool) -> Unit {
    let label = if kb { " runs" } else { " kb" };
    Unit::new(
        vec![segment("m", Role::Key), segment(label, Role::Muted)],
        9,
    )
}

/// Project heading, phase chip and `knowledge base`; the project id (or
/// `project N/M` when narrow) on the right.
pub fn title(app: &App, width: u16, narrow: bool) -> Vec<Segment> {
    let projects = app.kb_projects();
    let scope = app.kb_scope();
    let row = scope
        .id()
        .and_then(|id| projects.iter().find(|r| field(r, "id") == id));
    let mut left = Vec::new();
    match row {
        Some(row) => {
            left.push(Unit::one(field(row, "text"), Role::Title, 0));
            let phase = field(row, "status");
            if !phase.is_empty() {
                left.push(Unit::new(vec![chip(app.look, phase, Chip::Plain)], 0));
            }
        }
        None => left.push(Unit::one("All records", Role::Title, 0)),
    }
    left.push(Unit::one("knowledge base", Role::Muted, 3));
    let at = match &scope {
        Scope::Project(id) => projects.iter().position(|r| field(r, "id") == id),
        Scope::All => None,
    }
    .unwrap_or(projects.len());
    let right = if narrow {
        format!("project {}/{}", at + 1, projects.len() + 1)
    } else {
        scope.id().unwrap_or("all records").to_owned()
    };
    fit(
        (left, vec![Unit::one(right, Role::Muted, 1)]),
        3,
        usize::from(width),
        glyphs(app),
    )
}

fn tab(app: &App, view: KbView) -> (Unit, bool) {
    let name = format!("{} {}", view.digit(), view.name());
    let mut parts = if view == app.kb.view {
        vec![chip(app.look, &name, Chip::Active)]
    } else {
        vec![
            segment(view.digit().to_string(), Role::Digit),
            segment(format!(" {}", view.name()), Role::Tab),
        ]
    };
    let mut marked = false;
    if view == KbView::Stale
        && let Some(reply) = app.kb.stale.as_ref().filter(|r| r.error.is_none())
    {
        for (name, mark, tone) in [("stale", "!", Tone::Bad), ("unverifiable", "?", Tone::Warn)] {
            if let Some(n) = reply.count(name).filter(|n| *n > 0) {
                parts.push(segment(format!(" {mark}{n}"), Role::Badge(tone)));
                marked = true;
            }
        }
    }
    (Unit::new(parts, 0), marked)
}

/// Five tabs as the run mode lays out eight, and `m runs` at the end.
pub fn tabs(app: &App, width: u16, narrow: bool) -> Vec<Segment> {
    let room = usize::from(width);
    let all: Vec<_> = KbView::ALL.iter().map(|v| (*v, tab(app, *v))).collect();
    if !narrow {
        for gap in [4, 3, 2] {
            let units: Vec<_> = all.iter().map(|(_, (u, _))| u.clone()).collect();
            if total(&units, &[], gap) <= room {
                return fit((units, vec![mode_unit(true)]), gap, room, glyphs(app));
            }
        }
    }
    let index = KbView::ALL
        .iter()
        .position(|v| *v == app.kb.view)
        .map_or(1, |i| i + 1);
    let left = all
        .into_iter()
        .filter(|(view, (_, marked))| *view == app.kb.view || *marked)
        .map(|(view, (mut unit, _))| {
            unit.drop = u8::from(view != app.kb.view);
            unit
        })
        .collect();
    let right = vec![
        mode_unit(true),
        Unit::one(format!("{index}/{}", KbView::ALL.len()), Role::Muted, 0),
    ];
    fit((left, right), 4, room, glyphs(app))
}

fn hints(app: &App, narrow: bool) -> Vec<(&'static str, &'static str)> {
    if app.kb_projects().is_empty() && app.kb_entries().is_empty() {
        return vec![("m", "runs"), ("?", "help"), ("q", "quit")];
    }
    let mut out = match app.kb.focus {
        Focus::Runs => vec![("j/k", "project"), ("l", "records")],
        Focus::Inspector => vec![("j/k", "scroll"), ("h", "back"), ("d", "detail")],
        Focus::Detail if app.kb.view == KbView::Status => vec![("j/k", "scroll"), ("Tab", "view")],
        Focus::Detail => vec![("j/k", "record"), ("l", "inspect"), ("a", "impact")],
    };
    if narrow {
        out.remove(0);
    }
    out.extend([("?", "help"), ("q", "quit")]);
    out
}

/// The run mode's left group, with `kb` and `index` from `brief --all`.
pub fn footer(app: &App, width: u16, narrow: bool) -> Vec<Segment> {
    let right = hints(app, narrow)
        .into_iter()
        .map(|(key, label)| {
            Unit::new(
                vec![
                    segment(key, Role::Key),
                    segment(format!(" {label}"), Role::Muted),
                ],
                0,
            )
        })
        .collect();
    let left = if let Some(notice) = &app.notice {
        vec![Unit::one(notice.clone(), Role::Notice, 0)]
    } else {
        let at = |key| {
            app.kb
                .projects
                .as_ref()
                .and_then(|r| r.trailer_text(key))
                .unwrap_or_else(|| "unknown".into())
        };
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

pub(crate) fn draw(frame: &mut Frame<'_>, app: &App, areas: &Areas, narrow: bool) {
    let look: Look = app.look;
    let rows = [
        (title(app, areas.header.width, narrow), areas.header),
        (tabs(app, areas.tabs.width, narrow), areas.tabs),
        (footer(app, areas.footer.width, narrow), areas.footer),
    ];
    for (parts, area) in rows {
        let line = bars::line(parts, look);
        frame.render_widget(Paragraph::new(line).style(style::bar(look)), area);
    }
}
