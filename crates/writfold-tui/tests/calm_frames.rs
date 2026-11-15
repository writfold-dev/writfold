//! The calm frame as a whole: the legend moved to help, and the calm and
//! detailed buffers at the sizes the guide shows.
mod support;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use support::projection::{golden, normalize, render, rows, symbols};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Focus, Key};

fn map(buffer: &Buffer, area: Rect) -> String {
    rows(buffer)
        .iter()
        .skip(usize::from(area.y))
        .take(usize::from(area.height))
        .map(|row| {
            row.chars()
                .skip(usize::from(area.x))
                .take(usize::from(area.width))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn legend_lives_in_help() -> TestResult {
    let w = support::map::nested("calm-legend")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.focus = Focus::Detail;
    let buffer = render(&mut app, 136, 38)?;
    let area = writfold_tui::layout::focused(136, 38, true, true).main;
    assert!(!map(&buffer, area).contains("satisfied"));
    app.key(Key::Char('?'));
    let help = symbols(&render(&mut app, 136, 38)?);
    let subject = writfold_tui::poll::subject_text(&app.stamps.subject).replacen('=', " ", 1);
    for word in ["Marks", "not a liveness claim", subject.as_str()] {
        assert!(help.contains(word), "{word}: {help}");
    }
    Ok(())
}

#[test]
fn calm_and_detailed_frame_goldens() -> TestResult {
    let w = support::map::nested("calm-frames")?;
    for (name, width, height, detail) in [
        ("t4-calm-136x38.txt", 136, 38, false),
        ("t4-calm-80x24.txt", 80, 24, false),
        ("t4-detail-136x38.txt", 136, 38, true),
    ] {
        let mut app = App::new(w.source(), Some(RUN));
        app.focus = Focus::Detail;
        if detail {
            app.key(Key::Char('d'));
        }
        let mut buffer = render(&mut app, width, height)?;
        support::buffers::stable(&mut buffer)?;
        golden(name, &normalize(&w, &symbols(&buffer))?)?;
    }
    Ok(())
}
