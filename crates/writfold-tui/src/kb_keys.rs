//! Keys in knowledge-base mode, and the two keys that cross modes: `m`
//! switches, `f` follows a run to the record its checkpoints name and a
//! record back to a run that names it. Letters keep their meaning: a run
//! view's letter leaves KB mode for that view.
use crate::app::{App, Focus, Key, View};
use crate::kb_state::{KbView, Mode, Scope};
use crate::layout::{self, Class};
use crate::source::field;
use writfold_kb::verbs::impact::Direction;

impl App {
    pub(crate) fn kb_key(&mut self, key: Key) {
        let page = self.main_height.saturating_sub(1).max(1);
        match key {
            Key::Esc | Key::Left => self.kb_focus(false),
            Key::Right => self.kb_focus(true),
            Key::Enter => self.kb_focus_cycle(),
            Key::Up => self.kb_motion(false, 1),
            Key::Down => self.kb_motion(true, 1),
            Key::PageUp => self.kb_motion(false, page),
            Key::PageDown => self.kb_motion(true, page),
            Key::Tab => self.kb_show(self.kb.view.next(false)),
            Key::BackTab => self.kb_show(self.kb.view.next(true)),
            Key::Char(c) => self.kb_char(c),
        }
    }

    fn kb_char(&mut self, c: char) {
        match c {
            '?' => self.help = true,
            'j' => self.kb_motion(true, 1),
            'k' => self.kb_motion(false, 1),
            'h' => self.kb_focus(false),
            'l' => self.kb_focus(true),
            '[' => self.kb_project(false),
            ']' => self.kb_project(true),
            'n' | 'N' => self.kb_marked(c == 'n'),
            'r' => self.reload(self.run(), None),
            'T' => self.cycle_theme(),
            'd' => {
                self.toggle_detail();
                self.kb.inspect_scroll = 0;
            }
            'm' => self.mode = Mode::Runs,
            'f' => self.follow_record(),
            'a' => self.kb_impact(),
            'v' if self.kb.view == KbView::Impact => {
                self.kb.direction = match self.kb.direction {
                    Direction::In => Direction::Out,
                    Direction::Out => Direction::Both,
                    Direction::Both => Direction::In,
                };
                self.kb.offset = 0;
                self.kb_load();
            }
            '0' if layout::class(self.width, self.height) == Class::Narrow => {
                self.kb.projects_view = true;
                self.kb.focus = Focus::Runs;
            }
            _ => {
                if let Some(view) = KbView::ALL.into_iter().find(|v| c == v.digit()) {
                    self.kb_show(view);
                } else if let Some(view) = View::ALL.into_iter().find(|v| c == v.key()) {
                    self.mode = Mode::Runs;
                    self.show(view);
                }
            }
        }
    }

    /// Enter KB mode; the first time on the selected run's project.
    pub fn enter_kb(&mut self) {
        self.mode = Mode::Kb;
        self.kb_load();
    }

    pub fn kb_show(&mut self, view: KbView) {
        self.kb.view = view;
        self.kb.scroll = 0;
        self.kb.offset = 0;
        self.kb.inspect_scroll = 0;
        self.kb.projects_view = false;
        if self.kb.focus == Focus::Inspector {
            self.kb.focus = Focus::Detail;
        }
        self.kb_load();
    }

    fn kb_focus(&mut self, right: bool) {
        self.kb.focus = match (self.kb.focus, right) {
            (Focus::Runs, true) => Focus::Detail,
            (Focus::Detail, true) if self.kb.view != KbView::Status => Focus::Inspector,
            (Focus::Inspector, false) => Focus::Detail,
            (Focus::Detail, false) => Focus::Runs,
            (focus, _) => focus,
        };
        self.kb_narrow_projects();
    }

    fn kb_focus_cycle(&mut self) {
        let next = match self.kb.focus {
            Focus::Runs => Focus::Detail,
            Focus::Detail if self.kb.view != KbView::Status => Focus::Inspector,
            _ => Focus::Runs,
        };
        self.kb.focus = next;
        self.kb_narrow_projects();
    }

    fn kb_narrow_projects(&mut self) {
        let narrow = layout::class(self.width, self.height) == Class::Narrow;
        self.kb.projects_view = narrow && self.kb.focus == Focus::Runs;
    }

    fn kb_motion(&mut self, down: bool, amount: u16) {
        let step = |value: u16| {
            if down {
                value.saturating_add(amount)
            } else {
                value.saturating_sub(amount)
            }
        };
        match self.kb.focus {
            Focus::Runs => {
                for _ in 0..amount.min(64) {
                    self.kb_project(down);
                }
            }
            Focus::Inspector => self.kb.inspect_scroll = step(self.kb.inspect_scroll),
            Focus::Detail if self.kb.view == KbView::Status => {
                self.kb.scroll = step(self.kb.scroll)
            }
            Focus::Detail => self.kb_select(down, usize::from(amount)),
        }
    }

    fn kb_select(&mut self, down: bool, amount: usize) {
        let keys: Vec<String> = self
            .kb_entries()
            .into_iter()
            .filter(|e| !e.header)
            .map(|e| e.key)
            .collect();
        let at = self
            .kb_entry()
            .and_then(|e| keys.iter().position(|k| *k == e.key))
            .unwrap_or(0);
        let next = if down {
            at.saturating_add(amount).min(keys.len().saturating_sub(1))
        } else {
            at.saturating_sub(amount)
        };
        if let Some(key) = keys.get(next) {
            self.kb.selected.insert(self.kb.view, key.clone());
            self.kb.inspect_scroll = 0;
            self.kb_load();
        }
    }

    /// The previous or next project; `all records` is the last entry.
    pub(crate) fn kb_project(&mut self, down: bool) {
        let mut scopes: Vec<Scope> = self
            .kb_projects()
            .iter()
            .map(|r| Scope::Project(field(r, "id").to_owned()))
            .collect();
        scopes.push(Scope::All);
        let at = scopes
            .iter()
            .position(|s| *s == self.kb_scope())
            .unwrap_or(0);
        let next = if down {
            (at + 1).min(scopes.len() - 1)
        } else {
            at.saturating_sub(1)
        };
        if next != at {
            self.kb_scope_to(scopes.swap_remove(next));
        }
    }

    pub(crate) fn kb_scope_to(&mut self, scope: Scope) {
        if self.kb.scope.as_ref() == Some(&scope) {
            return;
        }
        self.kb.scope = Some(scope);
        self.kb.selected.clear();
        self.kb.subject = None;
        self.kb.scroll = 0;
        self.kb.offset = 0;
        self.kb.inspect_scroll = 0;
        self.kb_load();
    }
}
