//! The interactive loop: raw mode and the alternate screen through
//! ratatui's crossterm backend, a redraw only after a key, a resize or a
//! moved KB pin, and a pin poll every two seconds.
use crate::app::{App, Key};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::{Duration, Instant};

/// How often the KB pin is observed.
pub const POLL: Duration = Duration::from_secs(2);

fn key(code: KeyCode, modifiers: KeyModifiers) -> Option<Key> {
    Some(match code {
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => Key::Char('q'),
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::BackTab => Key::BackTab,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Esc => Key::Esc,
        KeyCode::Char(c) => Key::Char(c),
        _ => return None,
    })
}

/// Take the terminal, run until `q`, and give the terminal back, also
/// when drawing fails.
pub fn run(app: &mut App) -> std::io::Result<()> {
    let mut terminal = ratatui::try_init()?;
    let result = (|| -> std::io::Result<()> {
        let mut polled = Instant::now();
        let mut dirty = true;
        loop {
            if dirty {
                terminal.draw(|frame| crate::view::draw(frame, app))?;
                dirty = false;
            }
            if event::poll(POLL.saturating_sub(polled.elapsed()))? {
                match event::read()? {
                    Event::Key(k) if k.kind == KeyEventKind::Press => {
                        if let Some(k) = key(k.code, k.modifiers) {
                            if !app.key(k) {
                                return Ok(());
                            }
                            dirty = true;
                        }
                    }
                    Event::Resize(..) => dirty = true,
                    _ => {}
                }
            }
            if polled.elapsed() >= POLL {
                dirty |= app.poll();
                polled = Instant::now();
            }
        }
    })();
    ratatui::try_restore()?;
    result
}
