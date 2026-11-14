//! Cached projections and stamp-driven refresh, independent of rendering.
use crate::app::{App, View};
use crate::layout::{self, Class};
use crate::poll::Stamps;
use crate::source::{field, flags};

impl App {
    pub(crate) fn reload(&mut self, keep: Option<String>, observed: Option<Stamps>) {
        let old_recorded = self.worktree();
        let reuse = observed.is_some();
        if let Some(stamps) = observed {
            self.stamps = stamps;
        } else {
            self.stamps.pin = self.source.pin(&mut self.tool);
            self.stamps.local = self.source.local();
        }
        self.runs = self
            .source
            .runs(self.open_only, self.project_only.as_deref());
        self.asked = self.source.clock();
        self.answers.clear();
        self.guards = None;
        self.subjects.clear();
        self.order_runs(keep.as_deref());
        if self.run_id() != keep.as_deref() {
            self.scroll = 0;
            self.anchor = true;
        }
        if reuse && self.run_id() == keep.as_deref() && old_recorded == self.worktree() {
            self.remember_subject();
        } else {
            self.refresh_subject();
        }
        self.load();
    }

    fn order_runs(&mut self, keep: Option<&str>) {
        let (mut order, closed): (Vec<_>, Vec<_>) = (0..self.runs.rows.len())
            .partition(|i| field(&self.runs.rows[*i], "status") != "closed");
        order.extend(closed);
        self.order = order;
        self.selected = keep
            .and_then(|id| {
                self.order
                    .iter()
                    .position(|i| field(&self.runs.rows[*i], "id") == id)
            })
            .or_else(|| {
                self.order
                    .iter()
                    .position(|i| flags(&self.runs.rows[*i]).contains(&"current"))
            })
            .unwrap_or(0);
    }

    pub fn poll(&mut self) -> bool {
        self.ticks = (self.ticks + 1) % 5;
        let pin = self.source.pin(&mut self.tool);
        let local = self.source.local();
        let moved = pin != self.stamps.pin || local != self.stamps.local;
        if !moved && self.ticks != 0 {
            return false;
        }
        let recorded = self.worktree();
        let subject = self.source.subject(&mut self.tool, recorded.as_deref());
        if !moved && subject == self.stamps.subject {
            return false;
        }
        self.reload(
            self.run(),
            Some(Stamps {
                pin,
                local,
                subject,
            }),
        );
        true
    }

    pub(crate) fn refresh_subject(&mut self) {
        let recorded = self.worktree();
        self.stamps.subject = self.source.subject(&mut self.tool, recorded.as_deref());
        self.remember_subject();
    }

    fn remember_subject(&mut self) {
        if let Some(run) = self.run() {
            if self
                .subjects
                .get(&run)
                .is_some_and(|s| s != &self.stamps.subject)
            {
                self.answers.remove(&run);
            }
            self.subjects.insert(run, self.stamps.subject.clone());
        }
    }

    pub fn load_view(&mut self, view: View) {
        if self.answer(view).is_some() {
            return;
        }
        if view == View::Guards {
            self.guards = Some(self.source.guards());
            self.asked = self.source.clock();
            return;
        }
        let Some(run) = self.run() else {
            return;
        };
        let answer = match view {
            View::Progress => self.source.plan(&run),
            View::Gates => self.source.gates(&run),
            View::Timeline => self.source.timeline(&run),
            View::Inbox | View::Resume => self.source.resume(&run),
            View::Stale => self.source.stale(self.project().as_deref()),
            View::Brief => self.source.brief(self.project().as_deref()),
            View::Guards => self.source.guards(),
        };
        let no_plan = view == View::Progress && field(&answer.trailer, "reason") == "no-plan";
        self.answers
            .entry(run)
            .or_default()
            .insert(view.cache_key(), answer);
        self.asked = self.source.clock();
        if no_plan {
            self.load_view(View::Gates);
        }
    }

    pub fn load(&mut self) {
        if !self.runs_view {
            self.load_view(self.view);
        }
        match layout::class(self.width, self.height) {
            Class::Wide => {
                self.load_view(View::Gates);
                self.load_view(View::Inbox);
                self.load_view(View::Timeline);
                if self.height >= 38 {
                    self.load_view(View::Guards);
                }
            }
            Class::Medium => {
                self.load_view(View::Gates);
                self.load_view(View::Inbox);
                self.load_view(View::Guards);
            }
            _ => {}
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        if (width, height) == (self.width, self.height) {
            return;
        }
        self.width = width;
        self.height = height;
        self.main_height = layout::areas(width, height)
            .main
            .height
            .saturating_sub(2)
            .max(1);
        if layout::class(width, height) != Class::Narrow {
            self.runs_view = false;
        }
        self.load();
    }

    pub fn show(&mut self, view: View) {
        self.view = view;
        self.runs_view = false;
        self.scroll = 0;
        self.anchor = true;
        self.load();
    }

    pub(crate) fn select(&mut self, down: bool) {
        let last = self.order.len().saturating_sub(1);
        let next = if down {
            (self.selected + 1).min(last)
        } else {
            self.selected.saturating_sub(1)
        };
        if next == self.selected {
            return;
        }
        self.selected = next;
        self.scroll = 0;
        self.anchor = true;
        self.refresh_subject();
        self.load();
    }
}
