//! The header, tabs, footer and help surfaces.
use crate::app::App;
use crate::frame as content;
use crate::layout::Areas;
use crate::lines::Tone;
use crate::paint::{block, text};
use crate::style::{self, Look};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph, Wrap};

pub fn bars(frame: &mut Frame<'_>, app: &App, areas: &Areas, narrow: bool) {
    let header = Line::styled(
        content::header(app, narrow),
        style::style(app.look, Tone::Head),
    );
    frame.render_widget(
        Paragraph::new(header).style(style::bar(app.look)),
        areas.header,
    );
    tabs(frame, app, areas.tabs, narrow);
    if let Some(area) = areas.summary {
        frame.render_widget(
            Paragraph::new(crate::side::summary(app)).style(style::bar(app.look)),
            area,
        );
    }
    let footer = if app.notice.is_some() || !narrow {
        content::footer(app, true)
    } else {
        format!(
            "kb={} ? q",
            app.runs.trailer_text("kb").unwrap_or_else(|| "?".into())
        )
    };
    let tone = if app.notice.is_some() {
        Tone::Warn
    } else {
        Tone::Dim
    };
    let line = Line::styled(footer.as_str(), style::style(app.look, tone));
    frame.render_widget(
        Paragraph::new(line).style(style::bar(app.look)),
        areas.footer,
    );
    if app.notice.is_none() {
        for marker in [" ?", " q"] {
            if let Some(at) = footer.rfind(marker) {
                mark(
                    frame,
                    areas.footer,
                    &footer,
                    at + 1,
                    at + 2,
                    style::style(app.look, Tone::Key),
                );
            }
        }
    }
}

fn tabs(frame: &mut Frame<'_>, app: &App, area: Rect, narrow: bool) {
    let tabs = content::tabs(app, narrow);
    let role = if narrow { Tone::Key } else { Tone::Plain };
    let line = Line::styled(tabs.as_str(), style::style(app.look, role));
    frame.render_widget(Paragraph::new(line).style(style::bar(app.look)), area);
    if !narrow {
        if let (Some(start), Some(end)) = (tabs.find('['), tabs.find(']')) {
            mark(
                frame,
                area,
                &tabs,
                start,
                end + 1,
                style::raised(app.look).add_modifier(Modifier::BOLD),
            );
        }
        for (at, ch) in tabs.char_indices().filter(|(_, c)| c.is_ascii_digit()) {
            mark(
                frame,
                area,
                &tabs,
                at,
                at + ch.len_utf8(),
                style::style(app.look, Tone::Key),
            );
        }
    }
}

fn mark(frame: &mut Frame<'_>, area: Rect, text: &str, start: usize, end: usize, style: Style) {
    let x = u16::try_from(Line::raw(&text[..start]).width()).unwrap_or(u16::MAX);
    if x >= area.width {
        return;
    }
    let width = u16::try_from(Line::raw(&text[start..end]).width()).unwrap_or(u16::MAX);
    frame.buffer_mut().set_style(
        Rect::new(area.x + x, area.y, width.min(area.width - x), 1),
        style,
    );
}

pub fn help(frame: &mut Frame<'_>, look: Look) {
    let area = frame.area();
    let width = area.width.min(100);
    let body = Paragraph::new(text(content::help(), look))
        .style(style::raised(look))
        .wrap(Wrap { trim: false });
    let rows = body.line_count(width.saturating_sub(2)).saturating_add(2);
    let height = area.height.min(u16::try_from(rows).unwrap_or(u16::MAX));
    let popup = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    let border = block("help".into(), true, look)
        .style(style::raised(look))
        .border_style(style::raised(look).patch(style::style(look, Tone::Current)));
    frame.render_widget(Clear, popup);
    frame.render_widget(body.block(border), popup);
}
