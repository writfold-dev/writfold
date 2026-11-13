//! Keys only change presentation or re-ask read verbs.
use crate::app::{App, Focus, Key, View};
use crate::layout::{self, Class};

impl App {
    pub fn key(&mut self, key: Key) -> bool {
        self.notice = None;
        if key == Key::Char('q') {
            return false;
        }
        if self.help {
            self.help = false;
            return true;
        }
        match key {
            Key::Esc => self.focus = Focus::Runs,
            Key::Up => self.motion(false),
            Key::Down => self.motion(true),
            Key::Enter => self.focus_next(),
            Key::Tab => self.show(self.view.next(false)),
            Key::BackTab => self.show(self.view.next(true)),
            Key::PageUp => self.scroll_by(false, self.main_height.saturating_sub(1)),
            Key::PageDown => self.scroll_by(true, self.main_height.saturating_sub(1)),
            Key::Char(c) => self.character(c),
        }
        true
    }

    fn focus_next(&mut self) {
        self.focus = match self.focus {
            Focus::Runs => Focus::Detail,
            Focus::Detail => Focus::Runs,
        };
        if self.focus == Focus::Detail && self.runs_view {
            self.runs_view = false;
            self.load();
        }
    }

    fn motion(&mut self, down: bool) {
        if self.focus == Focus::Detail && !self.runs_view {
            self.scroll_by(down, 1);
        } else {
            self.select(down);
        }
    }

    fn scroll_by(&mut self, down: bool, amount: u16) {
        self.anchor = false;
        self.scroll = if down {
            self.scroll.saturating_add(amount)
        } else {
            self.scroll.saturating_sub(amount)
        };
    }

    fn character(&mut self, c: char) {
        match c {
            '?' => self.help = true,
            'j' => self.motion(true),
            'k' => self.motion(false),
            '[' => self.select(false),
            ']' => self.select(true),
            'r' => self.reload(self.run(), None),
            'x' if self.view == View::Progress && !self.runs_view => {
                self.expand_all = !self.expand_all;
                self.anchor = true;
            }
            'o' | 'P' => self.filter(c),
            '0' if layout::class(self.width, self.height) == Class::Narrow => {
                self.runs_view = true;
                self.focus = Focus::Runs;
            }
            _ => {
                if let Some(view) = View::ALL
                    .into_iter()
                    .find(|v| c == v.key() || c == v.digit())
                {
                    self.show(view);
                }
            }
        }
    }

    fn filter(&mut self, key: char) {
        if key == 'o' {
            self.open_only = !self.open_only;
        } else if self.project_only.is_some() {
            self.project_only = None;
        } else if let Some(project) = self.project() {
            self.project_only = Some(project);
        } else {
            self.notice = Some("selected run has no project; filter unchanged".into());
            return;
        }
        self.reload(self.run(), None);
    }
}
