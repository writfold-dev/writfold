//! Keys only change presentation or re-ask read verbs.
use crate::app::{App, Detail, Focus, Key, View};
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
        if self.mode == crate::kb_state::Mode::Kb {
            self.kb_key(key);
            return true;
        }
        match key {
            Key::Esc | Key::Left => self.focus_move(false, false),
            Key::Right => self.focus_move(true, false),
            Key::Up => self.motion(false),
            Key::Down => self.motion(true),
            Key::Enter => self.focus_move(true, true),
            Key::Tab => self.show(self.view.next(false)),
            Key::BackTab => self.show(self.view.next(true)),
            Key::PageUp => self.scroll_by(false, self.main_height.saturating_sub(1)),
            Key::PageDown => self.scroll_by(true, self.main_height.saturating_sub(1)),
            Key::Char(c) => self.character(c),
        }
        true
    }

    fn character(&mut self, c: char) {
        match c {
            '?' => self.help = true,
            'j' => self.motion(true),
            'k' => self.motion(false),
            '[' => self.select(false),
            'h' => self.focus_move(false, false),
            'l' => self.focus_move(true, false),
            '.' | 'n' | 'N' => self.jump_node(c),
            ']' => self.select(true),
            'r' => self.reload(self.run(), None),
            'T' => self.cycle_theme(),
            'd' => self.toggle_detail(),
            'm' => self.enter_kb(),
            'f' => self.follow_run(),
            'x' if self.view == View::Progress && !self.runs_view => self.toggle_map(),
            'X' if self.view == View::Progress && !self.map && !self.runs_view => {
                self.expand_all = !self.expand_all;
                self.anchor = true;
            }
            'o' | 'P' => self.filter(c),
            '0' if layout::class(self.width, self.height) == Class::Narrow => {
                self.runs_view = true;
                self.focus = Focus::Runs;
                self.scroll = 0;
                self.anchor = true;
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

    fn toggle_map(&mut self) {
        self.map = !self.map;
        self.scroll = 0;
        self.anchor = true;
        if self.focus == Focus::Inspector {
            self.focus = Focus::Detail;
        }
        self.load();
    }

    pub(crate) fn toggle_detail(&mut self) {
        self.level = match self.level {
            Detail::Calm => Detail::Full,
            Detail::Full => Detail::Calm,
        };
        self.inspect_scroll = 0;
    }

    pub(crate) fn cycle_theme(&mut self) {
        if self.look.depth == crate::style::Depth::Truecolor {
            self.look.theme = self.look.theme.next();
            self.notice = Some(format!("theme={} (not written)", self.look.theme.name()));
        } else {
            self.notice =
                Some("themes need 24-bit colour: COLORTERM=truecolor or --color truecolor".into());
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
