//! ANSI roles, pane borders, and ratatui's wrapped-line measurements.
use crate::lines::{self, Tone};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub fn style(tone: Tone) -> Style {
    match tone {
        Tone::Plain | Tone::Done => Style::default(),
        Tone::Dim | Tone::Skipped | Tone::Later => Style::default().fg(Color::DarkGray),
        Tone::Head | Tone::Key => Style::default().add_modifier(Modifier::BOLD),
        Tone::Good => Style::default().fg(Color::Green),
        Tone::Vouched => Style::default().fg(Color::Magenta),
        Tone::Info => Style::default().fg(Color::Blue),
        Tone::Warn => Style::default().fg(Color::Yellow),
        Tone::Current => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        Tone::Bad => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    }
}

pub fn text(lines: Vec<lines::Line>) -> Text<'static> {
    Text::from(
        lines
            .into_iter()
            .map(|line| Line::styled(line.text, style(line.tone)))
            .collect::<Vec<_>>(),
    )
}

pub fn block(title: String, focused: bool) -> Block<'static> {
    let border = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(Line::styled(format!(" {title} "), style(Tone::Head)))
}

/// Put the current head on the third visible row only when scrolling is needed.
pub fn initial_scroll(
    lines: &[lines::Line],
    current: Option<usize>,
    width: u16,
    height: u16,
) -> u16 {
    let Some(current) = current.filter(|index| *index < lines.len()) else {
        return 0;
    };
    let mut total = 0;
    let mut prefix = 0;
    for (index, line) in lines.iter().enumerate() {
        let count = Paragraph::new(Line::raw(line.text.as_str()))
            .wrap(Wrap { trim: false })
            .line_count(width);
        total += count;
        if index < current {
            prefix += count;
        }
    }
    if total <= usize::from(height) {
        return 0;
    }
    u16::try_from(prefix.saturating_sub(2)).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::{initial_scroll, style};
    use crate::lines::{Line, Tone};
    use ratatui::style::{Color, Modifier};
    use ratatui::widgets::{Paragraph, Wrap};

    #[test]
    fn anchor_uses_rendered_unicode_and_indented_prefix() -> Result<(), Box<dyn std::error::Error>>
    {
        let prefix = "  界界 e\u{301} words with a long continuation";
        let lines = [prefix, "", "> current", "more", "last"]
            .into_iter()
            .map(|text| Line {
                text: text.into(),
                tone: Tone::Plain,
            })
            .collect::<Vec<_>>();
        let offset = initial_scroll(&lines, Some(2), 10, 3);
        assert_eq!(initial_scroll(&lines, Some(2), 80, 20), 0);
        assert_eq!(initial_scroll(&lines, None, 10, 3), 0);
        assert_eq!(initial_scroll(&lines, Some(0), 10, 3), 0);
        let pane = Paragraph::new(super::text(lines))
            .wrap(Wrap { trim: false })
            .scroll((offset, 0));
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(10, 3))?;
        terminal.draw(|frame| frame.render_widget(pane, frame.area()))?;
        let bottom: String = (0..10)
            .map(|x| terminal.backend().buffer()[(x, 2)].symbol())
            .collect();
        assert_eq!(bottom.trim_end(), "> current");
        Ok(())
    }

    #[test]
    fn ansi_roles_do_not_paint_backgrounds() {
        for tone in [
            Tone::Bad,
            Tone::Warn,
            Tone::Good,
            Tone::Vouched,
            Tone::Current,
            Tone::Done,
            Tone::Skipped,
            Tone::Later,
            Tone::Dim,
            Tone::Info,
            Tone::Head,
            Tone::Plain,
            Tone::Key,
        ] {
            assert_eq!(style(tone).bg, None);
        }
        assert_eq!(style(Tone::Vouched).fg, Some(Color::Magenta));
        assert_eq!(style(Tone::Bad).fg, Some(Color::Red));
        assert!(style(Tone::Bad).add_modifier.contains(Modifier::BOLD));
    }
}
