//! Wrap each logical line inside its own hanging-indent rectangle.
use crate::lines::Line;
use crate::style::{self, Look};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, Wrap},
};

fn parts(line: &Line, width: u16) -> (u16, &str) {
    let spaces = line.text.bytes().take_while(|b| *b == b' ').count();
    let indent = spaces.min(usize::from(width.saturating_sub(1)));
    (indent as u16, &line.text[indent..])
}

pub fn height(line: &Line, width: u16) -> usize {
    let (indent, text) = parts(line, width);
    Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .line_count(width - indent)
}

pub fn draw(frame: &mut Frame<'_>, lines: &[Line], area: Rect, look: Look, scroll: u16) {
    let mut skip = usize::from(scroll);
    let mut y = area.y;
    for line in lines {
        let count = height(line, area.width);
        if skip >= count {
            skip -= count;
            continue;
        }
        if y >= area.bottom() || area.width == 0 {
            break;
        }
        let visible = (count - skip).min(usize::from(area.bottom() - y)) as u16;
        let (indent, text) = parts(line, area.width);
        let pane = Paragraph::new(text)
            .style(style::style(look, line.tone))
            .wrap(Wrap { trim: false })
            .scroll((skip as u16, 0));
        frame.render_widget(
            pane,
            Rect::new(area.x + indent, y, area.width - indent, visible),
        );
        y += visible;
        skip = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lines::{Tone, line};

    #[test]
    fn commands_and_evidence_keep_their_indent_when_wrapped_and_scrolled()
    -> Result<(), Box<dyn std::error::Error>> {
        let lines = vec![line(
            "     next: writfold kb gate attest findings --by WHO",
            Tone::Info,
        )];
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(24, 8))?;
        for scroll in [0, 1] {
            terminal.draw(|frame| draw(frame, &lines, frame.area(), Look::default(), scroll))?;
            let buffer = terminal.backend().buffer();
            let mut shown = String::new();
            for y in 0..8 {
                let row: String = (0..24).map(|x| buffer[(x, y)].symbol()).collect();
                if !row.trim().is_empty() {
                    assert!(row.starts_with("     "), "{row:?}");
                    shown.push_str(row.trim());
                }
            }
            assert!(shown.contains("--by WHO"), "{shown}");
        }
        Ok(())
    }
}
