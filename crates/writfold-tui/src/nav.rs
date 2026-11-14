//! Focus and step selection refer only to cached verb rows.
use crate::app::{App, Focus, View};
use crate::layout::{self, Class};
use crate::source::{field, flags};

impl App {
    pub fn map_visible(&self) -> bool {
        self.view == View::Progress
            && self.map
            && !self.runs_view
            && self
                .answer(View::Progress)
                .is_some_and(|a| a.rows.iter().any(|r| field(r, "kind") == "step"))
    }

    pub(crate) fn sync_node(&mut self) {
        let Some(answer) = self.answer(View::Progress) else {
            return;
        };
        let steps = || answer.rows.iter().filter(|r| field(r, "kind") == "step");
        if steps().any(|r| Some(field(r, "id")) == self.node.as_deref()) {
            return;
        }
        self.node = steps()
            .find(|r| field(r, "status") == "current")
            .or_else(|| steps().next())
            .map(|r| field(r, "id").to_owned());
        self.inspect_scroll = 0;
        self.rail_offset = 0;
    }

    pub(crate) fn focus_move(&mut self, right: bool, cycle: bool) {
        let mapped =
            self.map_visible() || (self.runs_view && self.map && self.view == View::Progress);
        self.focus = match (self.focus, right, mapped, cycle) {
            (Focus::Runs, true, _, _) => Focus::Detail,
            (Focus::Detail, true, true, _) => Focus::Inspector,
            (Focus::Inspector, false, _, _) => Focus::Detail,
            (Focus::Detail, false, _, _) | (_, true, _, true) => Focus::Runs,
            (focus, _, _, _) => focus,
        };
        if self.focus != Focus::Runs && self.runs_view {
            self.runs_view = false;
            self.scroll = 0;
            self.anchor = true;
            self.load();
        } else if self.focus == Focus::Runs
            && layout::class(self.width, self.height) == Class::Narrow
        {
            self.runs_view = true;
        }
    }

    pub(crate) fn motion(&mut self, down: bool) {
        if self.focus == Focus::Inspector && self.map_visible() {
            self.inspect_scroll = if down {
                self.inspect_scroll.saturating_add(1)
            } else {
                self.inspect_scroll.saturating_sub(1)
            };
        } else if self.focus == Focus::Detail && self.map_visible() {
            self.move_node(down, 1);
        } else if self.focus == Focus::Detail && !self.runs_view {
            self.scroll_by(down, 1);
        } else {
            self.select(down);
        }
    }

    pub(crate) fn move_node(&mut self, down: bool, amount: usize) {
        let Some(answer) = self.answer(View::Progress) else {
            return;
        };
        let steps: Vec<_> = answer
            .rows
            .iter()
            .filter(|r| field(r, "kind") == "step")
            .collect();
        let at = steps
            .iter()
            .position(|r| Some(field(r, "id")) == self.node.as_deref())
            .unwrap_or(0);
        let next = if down {
            at.saturating_add(amount).min(steps.len().saturating_sub(1))
        } else {
            at.saturating_sub(amount)
        };
        self.node = steps.get(next).map(|r| field(r, "id").to_owned());
        self.inspect_scroll = 0;
    }

    pub(crate) fn jump_node(&mut self, key: char) {
        if !self.map_visible() {
            return;
        }
        let Some(answer) = self.answer(View::Progress) else {
            return;
        };
        let steps: Vec<_> = answer
            .rows
            .iter()
            .filter(|r| field(r, "kind") == "step")
            .collect();
        let at = steps
            .iter()
            .position(|r| Some(field(r, "id")) == self.node.as_deref())
            .unwrap_or(0);
        let next = if key == '.' {
            steps
                .iter()
                .find(|r| field(r, "status") == "current")
                .copied()
        } else {
            (1..=steps.len())
                .map(|delta| {
                    let index = if key == 'n' {
                        (at + delta) % steps.len()
                    } else {
                        (at + steps.len() - delta) % steps.len()
                    };
                    steps[index]
                })
                .find(|r| !flags(r).is_empty())
        };
        if let Some(row) = next {
            self.node = Some(field(row, "id").into());
            self.inspect_scroll = 0;
        }
    }

    pub(crate) fn scroll_by(&mut self, down: bool, amount: u16) {
        self.anchor = false;
        if self.map_visible() {
            if self.focus == Focus::Inspector {
                self.inspect_scroll = if down {
                    self.inspect_scroll.saturating_add(amount)
                } else {
                    self.inspect_scroll.saturating_sub(amount)
                };
            } else {
                self.move_node(down, usize::from(amount));
            }
        } else {
            self.scroll = if down {
                self.scroll.saturating_add(amount)
            } else {
                self.scroll.saturating_sub(amount)
            };
        }
    }
}
