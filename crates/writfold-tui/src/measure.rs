//! Display measures the command line shares with the view: a character's
//! width in cells (the same function the view lays text out with) and
//! the terminal's width, for diagnostics drawn on stderr.
use ratatui::text::Span;

/// The cells `c` takes in a terminal (wide characters take two).
pub fn cell(c: char) -> usize {
    let mut bytes = [0; 4];
    Span::raw(&*c.encode_utf8(&mut bytes)).width()
}

/// The terminal's width in columns, when one is attached.
pub fn columns() -> Option<usize> {
    ratatui::crossterm::terminal::size()
        .ok()
        .map(|(w, _)| usize::from(w))
        .filter(|w| *w > 0)
}
