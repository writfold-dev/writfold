mod support;

use ratatui::{buffer::Buffer, layout::Rect};
use support::plan::world;
use support::projection::{golden, normalize, render, rows, symbols};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Key, View};
use writfold_tui::layout::{self, Class};

#[test]
fn classes_cover_every_boundary() {
    for (width, height, expected) in [
        (140, 32, Class::Wide),
        (139, 32, Class::Medium),
        (140, 31, Class::Medium),
        (140, 37, Class::Wide),
        (140, 38, Class::Wide),
        (90, 16, Class::Medium),
        (89, 16, Class::Narrow),
        (90, 15, Class::Narrow),
        (60, 12, Class::Narrow),
        (59, 12, Class::TooSmall),
        (60, 11, Class::TooSmall),
        (0, 0, Class::TooSmall),
    ] {
        assert_eq!(layout::class(width, height), expected, "{width}x{height}");
    }
}

fn title(buffer: &Buffer, area: Rect, expected: &str) {
    let text: String = (area.x..area.right())
        .map(|x| buffer[(x, area.y)].symbol())
        .collect();
    assert!(text.contains(expected), "{expected}: {text}");
}

#[test]
fn wide_and_medium_buffer_goldens() -> TestResult {
    let w = world("frames")?;
    for (width, height, name) in [(160, 40, "frame-160x40.txt"), (100, 30, "frame-100x30.txt")] {
        let mut app = App::new(w.source(), Some(RUN));
        let buffer = render(&mut app, width, height)?;
        golden(name, &normalize(&w, &symbols(&buffer))?)?;
        let a = layout::areas(width, height);
        title(&buffer, a.runs.ok_or("runs area")?, "runs");
        title(&buffer, a.main, "progress");
        assert!(rows(&buffer)[0].contains(RUN));
        assert!(rows(&buffer)[usize::from(height - 1)].contains("kb="));
    }
    Ok(())
}

#[test]
fn all_sizes_render_their_real_panes() -> TestResult {
    let w = world("sizes")?;
    for (width, height) in [(160, 40), (150, 34), (100, 30), (70, 20), (50, 10)] {
        let mut app = App::new(w.source(), Some(RUN));
        let buffer = render(&mut app, width, height)?;
        let a = layout::areas(width, height);
        let screen = rows(&buffer);
        match layout::class(width, height) {
            Class::Wide => {
                assert_eq!(a.runs.ok_or("runs")?, Rect::new(0, 2, 40, height - 3));
                assert_eq!(a.main, Rect::new(40, 2, width - 88, height - 3));
                assert_eq!(a.gates.ok_or("gates")?, Rect::new(width - 48, 2, 48, 9));
                assert_eq!(a.inbox.ok_or("inbox")?, Rect::new(width - 48, 11, 48, 9));
                for (area, name) in [
                    (a.gates, "gates"),
                    (a.inbox, "inbox"),
                    (a.guards.or(a.timeline), "timeline"),
                ] {
                    title(&buffer, area.ok_or(name)?, name);
                }
                assert!(!symbols(&buffer).contains(" guards "));
            }
            Class::Medium => {
                assert_eq!(a.runs.ok_or("runs")?, Rect::new(0, 3, 30, height - 4));
                assert_eq!(a.main, Rect::new(30, 3, width - 30, height - 4));
                let summary = &screen[usize::from(a.summary.ok_or("summary")?.y)];
                for label in ["gates:", "inbox:", "drafts=1", "proposals=2"] {
                    assert!(summary.contains(label));
                }
            }
            Class::Narrow => {
                assert!(a.runs.is_none() && a.summary.is_none());
                assert!(screen[0].contains("run 1/3"));
                title(&buffer, a.main, "progress");
            }
            Class::TooSmall => assert!(symbols(&buffer).contains("needs 60x12")),
        }
    }
    Ok(())
}

#[test]
fn initial_current_head_uses_wrapped_lines_and_manual_scroll_stays() -> TestResult {
    let w = world("scroll")?;
    let mut app = App::new(w.source(), Some(RUN));
    let buffer = render(&mut app, 100, 30)?;
    let a = layout::areas(100, 30).main;
    let current_y = a.y + 3;
    let line: String = (a.x + 1..a.right() - 1)
        .map(|x| buffer[(x, current_y)].symbol())
        .collect();
    assert!(
        line.contains("change#1/review"),
        "third visible line: {line}"
    );
    assert!(!app.anchor);
    app.key(Key::Enter);
    app.key(Key::Down);
    let manual = app.scroll;
    render(&mut app, 100, 30)?;
    assert_eq!(app.scroll, manual);
    app.show(View::Timeline);
    app.show(View::Progress);
    assert!(app.anchor);
    Ok(())
}

#[test]
fn narrow_runs_view_and_short_strip_are_real_content() -> TestResult {
    let w = world("narrow")?;
    let mut app = App::new(w.source(), Some(RUN));
    render(&mut app, 70, 20)?;
    app.key(Key::Char('0'));
    assert!(app.runs_view);
    let text = symbols(&render(&mut app, 70, 20)?);
    assert!(text.contains("runs") && text.contains(RUN));
    app.key(Key::Char('p'));
    assert!(!app.runs_view);
    for (width, present) in [(28, false), (29, true)] {
        let detail = writfold_tui::frame::detail(&app, View::Progress, Some(width), true, false);
        assert_eq!(
            detail
                .lines
                .iter()
                .any(|l| l.text.contains("[x] [!] [-] [>] [ ] [ ]")),
            present
        );
    }
    Ok(())
}
