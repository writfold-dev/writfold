//! The header, tab bar and footer are laid out from whole segments: no
//! token is cut, the right group is complete, chips degrade to brackets.
mod support;

use ratatui::style::Color;
use std::collections::BTreeSet;
use support::projection::{render, rows};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Detail, Focus};
use writfold_tui::bars::{self, Segment};
use writfold_tui::layout::{self, Class};
use writfold_tui::style::{Depth, Look};
use writfold_tui::theme::ThemeId;

fn words(parts: &[Segment]) -> BTreeSet<String> {
    let text: String = parts.iter().map(|p| p.text.as_str()).collect();
    text.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn bars_never_cut_a_token() -> TestResult {
    let w = support::map::nested("bars-tokens")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.focus = Focus::Detail;
    for (width, exact) in (60..=200).flat_map(|w| [(w, false), (w, true)]) {
        app.level = if exact { Detail::Exact } else { Detail::Plain };
        let buffer = render(&mut app, width, 40)?;
        let narrow = layout::class(width, 40) == Class::Narrow;
        let screen = rows(&buffer);
        let header = words(&bars::title(&app, 1000, narrow));
        let footer = words(&bars::footer(&app, 1000, narrow));
        for (row, whole) in [(&screen[0], &header), (&screen[39], &footer)] {
            for word in row.split_whitespace() {
                assert!(whole.contains(word), "{width}: {word:?} cut in {row:?}");
            }
        }
        let right = match (exact, narrow) {
            (false, _) => "run 1 of 3",
            (true, true) => "run 1/3",
            (true, false) => RUN,
        };
        assert!(
            screen[0].trim_end().ends_with(right),
            "{width}: {}",
            screen[0]
        );
        let hints = if narrow {
            "l inspect   d detail   ? help   q quit"
        } else {
            "j/k step   l inspect   d detail   ? help   q quit"
        };
        assert!(
            screen[39].trim_end().ends_with(hints),
            "{width}: {}",
            screen[39]
        );
    }
    Ok(())
}

#[test]
fn chips_degrade_to_brackets() -> TestResult {
    let w = support::map::nested("bars-chips")?;
    let mut app = App::new(w.source(), Some(RUN));
    for depth in [Depth::Ansi, Depth::None] {
        app.look = Look {
            theme: ThemeId::Clay,
            depth,
        };
        let buffer = render(&mut app, 136, 38)?;
        let screen = rows(&buffer);
        for y in [0, 1, 37] {
            for x in 0..136 {
                assert_eq!(buffer[(x, y)].bg, Color::Reset, "{depth:?} {x},{y}");
            }
        }
        let bars = [&screen[0], &screen[1], &screen[37]]
            .map(|r| r.as_str())
            .join("\n");
        for chip in [
            "[open]",
            "[current run]",
            "[harbor]",
            "[1 progress]",
            "[read-only]",
        ] {
            assert!(bars.contains(chip), "{depth:?} {chip}: {bars}");
        }
    }
    truecolor_chips_are_filled_not_bracketed(&mut app)
}

/// At truecolor the active tab is the one `on_accent` on `accent` chip.
fn truecolor_chips_are_filled_not_bracketed(app: &mut App) -> TestResult {
    app.look = Look {
        theme: ThemeId::Clay,
        depth: Depth::Truecolor,
    };
    let buffer = render(app, 136, 38)?;
    let screen = rows(&buffer);
    let palette = ThemeId::Clay.palette();
    let accent = Color::Rgb(palette.accent.0, palette.accent.1, palette.accent.2);
    let on = Color::Rgb(
        palette.on_accent.0,
        palette.on_accent.1,
        palette.on_accent.2,
    );
    let start = screen[1].find(" 1 progress ").ok_or("active tab")?;
    let x = screen[1][..start].chars().count() as u16;
    for dx in 0..12 {
        let cell = &buffer[(x + dx, 1)];
        assert_eq!((cell.fg, cell.bg), (on, accent), "{}", x + dx);
    }
    for y in [0, 1, 37] {
        assert!(!screen[y].contains('['), "{}", screen[y]);
    }
    Ok(())
}
