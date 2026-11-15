mod support;
use ratatui::style::Color;
use support::projection::{render, rows, symbols};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Focus};
use writfold_tui::cells::{Glyphs, State};
use writfold_tui::graph;
use writfold_tui::style::{Depth, Environment, depth};

fn honest(text: &str) -> TestResult {
    let forbidden = regex::Regex::new(
        r"(?i)\b(running|working|active|idle|stalled|live|in\s+progress|elapsed|relative|ago)\b|\d+(?:\.\d+)?\s*%",
    )?;
    assert!(
        !forbidden.is_match(text),
        "forbidden language in frame: {text}"
    );
    Ok(())
}

/// Calm rows show an id after its group name: its last component.
fn tail(id: &str) -> String {
    format!(" {} ", id.rsplit('/').next().unwrap_or(id))
}

fn ascii_buffer(app: &mut App, width: u16, height: u16) -> TestResult {
    let buffer = render(app, width, height)?;
    let text = symbols(&buffer);
    honest(&text)?;
    for cell in &buffer.content {
        assert_eq!(cell.fg, Color::Reset);
        assert_eq!(cell.bg, Color::Reset);
    }
    let area = writfold_tui::layout::focused(width, height, true, true).main;
    let rendered: Vec<String> = rows(&buffer)
        .iter()
        .map(|line| {
            line.chars()
                .skip(usize::from(area.x))
                .take(usize::from(area.width))
                .collect()
        })
        .collect();
    for line in rendered
        .iter()
        .skip(usize::from(area.y) + 1)
        .take(usize::from(area.height.saturating_sub(2)))
    {
        let interior: String = line
            .chars()
            .skip(1)
            .take(usize::from(area.width.saturating_sub(2)))
            .collect();
        for glyph in ['│', '┬', '┼', '├', '─', '╮', '╯', '┴'] {
            assert!(!interior.contains(glyph), "Unicode rail in {interior}");
        }
    }
    for node in graph::nodes(app.detail().ok_or("plan")?) {
        // Node lines, unlike pane borders, must use only ASCII map glyphs.
        let line = rendered
            .iter()
            .find(|line| line.contains(&tail(node.id)))
            .ok_or("visible node")?;
        assert!(
            line.contains(node.state.cell(Glyphs::Ascii).glyph),
            "{line}"
        );
        for glyph in ['✓', '–', '▶', '○', '×', '●', '◆', '·', '╮', '╯'] {
            assert!(!line.contains(glyph), "Unicode map glyph in {line}");
        }
    }
    for state in ["[x]", "[!]", "[-]", "[>]", "[ ]"] {
        assert!(text.contains(state), "missing {state}: {text}");
    }
    Ok(())
}

#[test]
fn no_color_and_dumb_terminal_render_ascii_state_evidence() -> TestResult {
    let w = support::plan::world("map-ascii")?;
    let dumb = Environment {
        term: Some("dumb".into()),
        ..Environment::default()
    };
    for selected_depth in [Depth::None, depth(None, &dumb)?] {
        assert_eq!(selected_depth, Depth::None);
        let mut app = App::new(w.source(), Some(RUN));
        app.focus = Focus::Detail;
        app.look.depth = selected_depth;
        ascii_buffer(&mut app, 160, 40)?;
    }
    let glyphs: std::collections::BTreeSet<_> = [
        State::Done,
        State::Stale,
        State::Skipped,
        State::Current,
        State::Later,
        State::Invalid,
    ]
    .map(|state| state.cell(Glyphs::Ascii).glyph)
    .into_iter()
    .collect();
    assert_eq!(glyphs.len(), 6);
    assert!(glyphs.contains("[#]"));
    Ok(())
}

#[test]
fn selected_map_row_sits_on_the_selection_surface_across_the_pane() -> TestResult {
    let w = support::plan::world("map-selected")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.focus = Focus::Detail;
    app.look = writfold_tui::style::Look {
        theme: writfold_tui::theme::ThemeId::Clay,
        depth: Depth::Truecolor,
    };
    let buffer = render(&mut app, 160, 40)?;
    let area = writfold_tui::layout::focused(160, 40, true, true).main;
    // Selection starts on the recorded current step, the only ▶ row.
    let y = rows(&buffer)
        .iter()
        .position(|line| line.contains('▶'))
        .ok_or("selected row")?;
    let selection = writfold_tui::theme::ThemeId::Clay.palette().selection;
    let expected = Color::Rgb(selection.0, selection.1, selection.2);
    for x in area.x + 1..area.right() - 1 {
        let cell = &buffer[(x, y as u16)];
        assert_eq!(cell.bg, expected, "{x},{y}");
        assert!(!cell.modifier.contains(ratatui::style::Modifier::REVERSED));
    }
    Ok(())
}

#[test]
fn rail_bars_never_take_a_state_colour() -> TestResult {
    let w = support::map::nested("map-dim-rail")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.focus = Focus::Detail;
    let theme = writfold_tui::theme::ThemeId::Clay;
    app.look = writfold_tui::style::Look {
        theme,
        depth: Depth::Truecolor,
    };
    for detail in [false, true] {
        if detail {
            app.key(writfold_tui::app::Key::Char('d'));
        }
        let buffer = render(&mut app, 160, 40)?;
        let area = writfold_tui::layout::focused(160, 40, true, true).main;
        let muted = theme.palette().muted;
        let dim = Color::Rgb(muted.0, muted.1, muted.2);
        let mut seen = 0;
        for y in area.y + 1..area.bottom() - 1 {
            for x in area.x + 1..area.right() - 1 {
                let cell = &buffer[(x, y)];
                if ["│", "├", "╮", "╯", "┬", "┼", "┴"].contains(&cell.symbol()) {
                    assert_eq!(cell.fg, dim, "{x},{y} detail={detail}");
                    seen += 1;
                }
            }
        }
        assert!(seen > 20, "the nested rail has bars: {seen}");
    }
    Ok(())
}
