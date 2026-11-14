//! Read-only watch state. Queries and refresh live in `state`, keys in `keys`.
use crate::poll::{Stamps, Subject};
use crate::source::{Reply, Source, field, flags};
use crate::style::Look;
use serde_json::Value;
use std::collections::BTreeMap;
use writfold_kb::workspace::GitTool;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum View {
    Progress,
    Gates,
    Timeline,
    Inbox,
    Guards,
    Stale,
    Brief,
    Resume,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Runs,
    Detail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Enter,
    Tab,
    BackTab,
    PageUp,
    PageDown,
    Esc,
    Char(char),
}

pub struct App {
    pub source: Source,
    pub stamps: Stamps,
    pub runs: Reply,
    pub order: Vec<usize>,
    pub selected: usize,
    pub view: View,
    pub focus: Focus,
    pub scroll: u16,
    pub help: bool,
    pub expand_all: bool,
    pub open_only: bool,
    pub project_only: Option<String>,
    pub notice: Option<String>,
    pub asked: String,
    pub runs_view: bool,
    pub anchor: bool,
    pub look: Look,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) main_height: u16,
    pub(crate) answers: BTreeMap<String, BTreeMap<View, Reply>>,
    pub(crate) guards: Option<Reply>,
    pub(crate) tool: Option<GitTool>,
    pub(crate) ticks: u8,
    pub(crate) subjects: BTreeMap<String, Subject>,
}

impl App {
    pub fn new(source: Source, run: Option<&str>) -> Self {
        let mut app = Self {
            source,
            stamps: Stamps::default(),
            runs: Reply::default(),
            order: Vec::new(),
            selected: 0,
            view: View::Progress,
            focus: Focus::Runs,
            scroll: 0,
            help: false,
            expand_all: false,
            open_only: false,
            project_only: None,
            notice: None,
            asked: String::new(),
            runs_view: false,
            anchor: true,
            look: Look::default(),
            width: 0,
            height: 0,
            main_height: 1,
            answers: BTreeMap::new(),
            guards: None,
            tool: None,
            ticks: 0,
            subjects: BTreeMap::new(),
        };
        app.reload(run.map(str::to_owned), None);
        app
    }

    pub fn row(&self) -> Option<&Value> {
        self.order
            .get(self.selected)
            .and_then(|i| self.runs.rows.get(*i))
    }

    pub fn run_id(&self) -> Option<&str> {
        self.row().map(|row| field(row, "id"))
    }

    pub fn run(&self) -> Option<String> {
        self.run_id().map(str::to_owned)
    }

    pub fn current_run(&self) -> Option<&str> {
        if let Some(current) = self.guards.as_ref().and_then(crate::guards::current) {
            return (current != "none").then_some(current);
        }
        self.runs
            .rows
            .iter()
            .find(|r| flags(r).contains(&"current"))
            .map(|r| field(r, "id"))
    }

    pub fn project(&self) -> Option<String> {
        self.row()?
            .get("more")?
            .as_array()?
            .iter()
            .filter(|m| field(m, "label") == "field")
            .flat_map(|m| field(m, "text").split(' '))
            .find_map(|part| part.strip_prefix("project="))
            .map(str::to_owned)
    }

    pub(crate) fn worktree(&self) -> Option<String> {
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

    pub fn runs_title(&self) -> String {
        let mut title = format!("runs ({})", self.order.len());
        if self.open_only {
            title.push_str(" open");
        }
        if let Some(project) = &self.project_only {
            title.push_str(&format!(" project={project}"));
        }
        title
    }

    pub fn answer(&self, view: View) -> Option<&Reply> {
        if view == View::Guards {
            return self.guards.as_ref();
        }
        self.answers.get(self.run_id()?)?.get(&view.cache_key())
    }

    pub fn detail(&self) -> Option<&Reply> {
        self.answer(self.view)
    }

    pub fn gate_status(&self) -> Option<&Reply> {
        self.answer(View::Gates)
    }
}
