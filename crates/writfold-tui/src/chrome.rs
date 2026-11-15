//! The header, tabs, footer and help surfaces.
use crate::app::App;
use crate::bars;
use crate::layout::Areas;
use crate::lines::Tone;
use crate::paint::{block, text};
use crate::style::{self, Look};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Clear, Paragraph, Wrap};

pub fn bars(frame: &mut Frame<'_>, app: &App, areas: &Areas, narrow: bool) {
    let rows = [
        (bars::title(app, areas.header.width, narrow), areas.header),
        (bars::tabs(app, areas.tabs.width, narrow), areas.tabs),
        (bars::footer(app, areas.footer.width, narrow), areas.footer),
    ];
    for (parts, area) in rows {
        let line = bars::line(parts, app.look);
        frame.render_widget(Paragraph::new(line).style(style::bar(app.look)), area);
    }
}

pub fn help(frame: &mut Frame<'_>, look: Look) {
    let area = frame.area();
    let width = area.width.min(100);
    let body = Paragraph::new(text(crate::frame::help(), look))
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
