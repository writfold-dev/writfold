mod support;

use ratatui::buffer::{Buffer, Cell};
use ratatui::style::{Color, Modifier};
use support::plan::world;
use support::projection::{render, rows};
use support::{RUN, TestResult};
use writfold_kb::{
    Verb,
    gate::{GateVerb, Say, statement::Kind},
};
use writfold_tui::app::{App, View};
use writfold_tui::style::{Depth, Look};
use writfold_tui::theme::{Rgb, ThemeId};

fn rgb(Rgb(r, g, b): Rgb) -> Color {
    Color::Rgb(r, g, b)
}

fn cells<'a>(buffer: &'a Buffer, needle: &str) -> TestResult<Vec<&'a Cell>> {
    for (y, row) in rows(buffer).iter().enumerate() {
        if let Some(byte) = row.find(needle) {
            let x = row[..byte].chars().count();
            return Ok((x..x + needle.chars().count())
                .map(|x| &buffer[(x as u16, y as u16)])
                .collect());
        }
    }
    Err(format!("missing {needle:?}:\n{}", rows(buffer).join("\n")).into())
}

fn foreground(buffer: &Buffer, needle: &str, color: Color, bold: bool) -> TestResult {
    for cell in cells(buffer, needle)? {
        assert_eq!(cell.fg, color, "{needle}");
        if bold {
            assert!(cell.modifier.contains(Modifier::BOLD), "{needle}");
        }
    }
    Ok(())
}

#[test]
fn truecolor_gate_and_step_roles_reach_the_surface() -> TestResult {
    let w = world("look-roles")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.look = Look {
        theme: ThemeId::Hone,
        depth: Depth::Truecolor,
    };
    let palette = ThemeId::Hone.palette();
    let buffer = render(&mut app, 160, 40)?;
    foreground(
        &buffer,
        "change#1/review [current]",
        rgb(palette.accent),
        true,
    )?;
    app.scroll = 0;
    let buffer = render(&mut app, 160, 40)?;
    foreground(
        &buffer,
        "change#1/benchmarks [skipped]",
        rgb(palette.muted),
        false,
    )?;
    app.show(View::Gates);
    let buffer = render(&mut app, 160, 40)?;
    foreground(&buffer, "lint [stale]", Color::Rgb(0xe8, 0x70, 0x7c), true)?;
    foreground(&buffer, "unit [satisfied]", rgb(palette.good), false)?;
    Ok(())
}

#[test]
fn selected_run_uses_text_on_selection_for_every_entry_line() -> TestResult {
    let w = world("look-selection")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.look = Look {
        theme: ThemeId::Hone,
        depth: Depth::Truecolor,
    };
    let buffer = render(&mut app, 160, 40)?;
    let area = writfold_tui::layout::areas(160, 40)
        .runs
        .ok_or("runs area")?;
    let text = rows(&buffer);
    let start = text
        .iter()
        .position(|line| line.contains("> [open]"))
        .ok_or("selected run")?;
    let end = text
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| line.contains("[open] run-"))
        .map(|(y, _)| y)
        .ok_or("next run")?;
    assert!(end - start >= 5, "all selected entry lines must be present");
    for y in start..end {
        for x in area.x + 1..area.right() - 1 {
            let cell = &buffer[(x, y as u16)];
            assert_eq!(cell.fg, rgb(ThemeId::Hone.palette().text), "{x},{y}");
            assert_eq!(cell.bg, rgb(ThemeId::Hone.palette().selection), "{x},{y}");
        }
    }
    Ok(())
}

#[test]
fn recorded_signoff_is_vouched_not_good() -> TestResult {
    let w = world("look-attested")?;
    let (text, exit) = w.verb(
        Verb::Gate(GateVerb::Say(Say {
            kind: Kind::Attested,
            id: "signoff".into(),
            text: "Read the diff against the signoff instructions".into(),
            by: "owner".into(),
            run: Some(RUN.into()),
            repo: None,
        })),
        false,
    )?;
    assert_eq!(exit, 0, "{text}");
    let mut app = App::new(w.source(), Some(RUN));
    app.look = Look {
        theme: ThemeId::Hone,
        depth: Depth::Truecolor,
    };
    app.show(View::Gates);
    let buffer = render(&mut app, 160, 40)?;
    let palette = ThemeId::Hone.palette();
    assert_ne!(palette.vouched, palette.good);
    foreground(&buffer, "signoff [attested]", rgb(palette.vouched), false)?;
    Ok(())
}

#[test]
fn degraded_depths_have_no_background_or_unwanted_colours() -> TestResult {
    let w = world("look-depth")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.show(View::Gates);
    for depth in [Depth::Ansi, Depth::None] {
        app.look = Look {
            theme: ThemeId::Hone,
            depth,
        };
        let buffer = render(&mut app, 160, 40)?;
        let color = if depth == Depth::Ansi {
            Color::Red
        } else {
            Color::Reset
        };
        foreground(&buffer, "lint [stale]", color, true)?;
        for cell in &buffer.content {
            assert_eq!(cell.bg, Color::Reset);
            if depth == Depth::None {
                assert_eq!(cell.fg, Color::Reset);
            }
        }
        assert!(
            cells(&buffer, "> [open]")?
                .iter()
                .all(|cell| cell.modifier.contains(Modifier::REVERSED))
        );
    }
    Ok(())
}
