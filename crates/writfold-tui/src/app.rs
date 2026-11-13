//! Selection, cached verb answers, and the KB, local-state and subject stamps.
use crate::poll::{Stamps, Subject};
use crate::source::{Reply, Source, field};
use serde_json::Value;
use std::collections::BTreeMap;
use writfold_kb::workspace::GitTool;

/// What the right-hand pane shows for the selected run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum View {
    /// `kb resume <run>` with `kb gate status --run <run>`'s counts.
    Resume,
    /// `kb stale <project>`.
    Stale,
    /// `kb brief <project>`.
    Brief,
}

impl View {
    fn next(self) -> Self {
        match self {
            Self::Resume => Self::Stale,
            Self::Stale => Self::Brief,
            Self::Brief => Self::Resume,
        }
    }
}

/// Which pane the arrow keys move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Runs,
    Detail,
}

/// A key, independent of the terminal library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Enter,
    Tab,
    Esc,
    Char(char),
}

pub struct App {
    pub source: Source,
    pub stamps: Stamps,
    /// `kb runs`.
    pub runs: Reply,
    /// Indices into `runs.rows`: runs that are not closed first, each
    /// group in the verb's order (newest activity first).
    pub order: Vec<usize>,
    /// A position in `order`.
    pub selected: usize,
    pub view: View,
    pub focus: Focus,
    pub scroll: u16,
    pub help: bool,
    answers: BTreeMap<(String, View), Reply>,
    gates: BTreeMap<String, Reply>,
    tool: Option<GitTool>,
    ticks: u8,
    subjects: BTreeMap<String, Subject>,
}

impl App {
    /// Ask the verbs once; select `run`, else the current run, else the
    /// first listed.
    pub fn new(source: Source, run: Option<&str>) -> Self {
        let mut app = Self {
            source,
            stamps: Stamps::default(),
            runs: Reply::default(),
            order: Vec::new(),
            selected: 0,
            view: View::Resume,
            focus: Focus::Runs,
            scroll: 0,
            help: false,
            answers: BTreeMap::new(),
            gates: BTreeMap::new(),
            tool: None,
            ticks: 0,
            subjects: BTreeMap::new(),
        };
        app.reload(run.map(str::to_owned));
        app
    }

    fn reload(&mut self, keep: Option<String>) {
        self.stamps.pin = self.source.pin(&mut self.tool);
        self.stamps.local = self.source.local();
        self.runs = self.source.runs();
        self.answers.clear();
        self.gates.clear();
        self.subjects.clear();
        let closed = |i: &usize| field(&self.runs.rows[*i], "status") == "closed";
        let (mut order, last): (Vec<usize>, Vec<usize>) =
            (0..self.runs.rows.len()).partition(|i| !closed(i));
        order.extend(last);
        self.order = order;
        let position = |pick: &dyn Fn(&Value) -> bool| {
            self.order.iter().position(|i| pick(&self.runs.rows[*i]))
        };
        self.selected = keep
            .and_then(|id| position(&|row| field(row, "id") == id))
            .or_else(|| position(&|row| crate::source::flags(row).contains(&"current")))
            .unwrap_or(0);
        self.refresh_subject();
        self.load();
    }

    /// Re-ask after a KB/local change, or a subject change on its 10 s tick.
    pub fn poll(&mut self) -> bool {
        self.ticks = (self.ticks + 1) % 5;
        let subject_moved = self.ticks == 0 && {
            let recorded = self.worktree();
            self.source.subject(&mut self.tool, recorded.as_deref()) != self.stamps.subject
        };
        if self.source.pin(&mut self.tool) == self.stamps.pin
            && self.source.local() == self.stamps.local
            && !subject_moved
        {
            return false;
        }
        self.reload(self.run());
        true
    }

    fn refresh_subject(&mut self) {
        let recorded = self.worktree();
        self.stamps.subject = self.source.subject(&mut self.tool, recorded.as_deref());
        if let Some(run) = self.run() {
            if self
                .subjects
                .get(&run)
                .is_some_and(|s| s != &self.stamps.subject)
            {
                self.answers.retain(|(id, _), _| id != &run);
                self.gates.remove(&run);
            }
            self.subjects.insert(run, self.stamps.subject.clone());
        }
    }

    fn worktree(&self) -> Option<String> {
        self.row()?
            .get("more")?
            .as_array()?
            .iter()
            .filter(|m| field(m, "label") == "field")
            .find_map(|m| field(m, "text").split_once("worktree="))
            .map(|(_, rest)| {
                rest.split_once(" base=")
                    .map_or(rest, |(path, _)| path)
                    .to_owned()
            })
    }

    /// The selected run's ID.
    pub fn run(&self) -> Option<String> {
        self.row().map(|row| field(row, "id").to_owned())
    }

    fn row(&self) -> Option<&Value> {
        self.order
            .get(self.selected)
            .and_then(|i| self.runs.rows.get(*i))
    }

    /// The selected run's project, from its `kb runs` place line.
    pub fn project(&self) -> Option<String> {
        let more = self.row()?.get("more")?.as_array()?;
        more.iter()
            .filter(|m| field(m, "label") == "field")
            .flat_map(|m| field(m, "text").split(' '))
            .find_map(|part| part.strip_prefix("project="))
            .map(str::to_owned)
    }

    /// Ask the verbs the current pane needs, unless already answered at
    /// this pin.
    pub fn load(&mut self) {
        let Some(run) = self.run() else {
            return;
        };
        let key = (run.clone(), self.view);
        if !self.answers.contains_key(&key) {
            let project = self.project();
            let reply = match self.view {
                View::Resume => self.source.resume(&run),
                View::Stale => self.source.stale(project.as_deref()),
                View::Brief => self.source.brief(project.as_deref()),
            };
            self.answers.insert(key, reply);
        }
        if self.view == View::Resume && !self.gates.contains_key(&run) {
            let reply = self.source.gates(&run);
            self.gates.insert(run, reply);
        }
    }

    /// The current pane's answer, if asked.
    pub fn detail(&self) -> Option<&Reply> {
        self.answers.get(&(self.run()?, self.view))
    }

    /// `kb gate status` for the selected run, if asked.
    pub fn gate_status(&self) -> Option<&Reply> {
        self.gates.get(&self.run()?)
    }

    /// Apply one key; false means quit. Every key only moves the view.
    pub fn key(&mut self, key: Key) -> bool {
        if self.help && key != Key::Char('q') {
            self.help = !matches!(key, Key::Esc | Key::Char('?'));
            return true;
        }
        match key {
            Key::Char('q') => return false,
            Key::Esc => self.focus = Focus::Runs,
            Key::Char('?') => self.help = true,
            Key::Up | Key::Char('k') => self.step(false),
            Key::Down | Key::Char('j') => self.step(true),
            Key::Enter => {
                self.focus = match self.focus {
                    Focus::Runs => Focus::Detail,
                    Focus::Detail => Focus::Runs,
                };
            }
            Key::Tab => self.show(self.view.next()),
            Key::Char('b') => self.show(View::Brief),
            Key::Char('s') => self.show(View::Stale),
            Key::Char('g') => self.show(View::Resume),
            Key::Char('r') => self.reload(self.run()),
            Key::Char(_) => {}
        }
        true
    }

    fn show(&mut self, view: View) {
        self.view = view;
        self.scroll = 0;
        self.load();
    }

    fn step(&mut self, down: bool) {
        if self.focus == Focus::Detail {
            self.scroll = if down {
                self.scroll.saturating_add(1)
            } else {
                self.scroll.saturating_sub(1)
            };
            return;
        }
        let last = self.order.len().saturating_sub(1);
        let previous = self.selected;
        self.selected = if down {
            (self.selected + 1).min(last)
        } else {
            self.selected.saturating_sub(1)
        };
        if self.selected == previous {
            return;
        }
        self.scroll = 0;
        self.refresh_subject();
        self.load();
    }
}
