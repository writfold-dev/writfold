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
    Stale,
    Brief,
    Resume,
}

impl View {
    pub const ALL: [Self; 7] = [
        Self::Progress,
        Self::Gates,
        Self::Timeline,
        Self::Inbox,
        Self::Stale,
        Self::Brief,
        Self::Resume,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Progress => "progress",
            Self::Gates => "gates",
            Self::Timeline => "timeline",
            Self::Inbox => "inbox",
            Self::Stale => "stale",
            Self::Brief => "brief",
            Self::Resume => "resume",
        }
    }

    pub fn key(self) -> char {
        match self {
            Self::Progress => 'p',
            Self::Gates => 'g',
            Self::Timeline => 't',
            Self::Inbox => 'i',
            Self::Stale => 's',
            Self::Brief => 'b',
            Self::Resume => 'e',
        }
    }

    pub fn digit(self) -> char {
        match self {
            Self::Progress => '1',
            Self::Gates => '2',
            Self::Timeline => '3',
            Self::Inbox => '4',
            Self::Stale => '6',
            Self::Brief => '7',
            Self::Resume => '8',
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.name() == name)
    }

    pub(crate) fn next(self, back: bool) -> Self {
        let at = Self::ALL.iter().position(|v| *v == self).unwrap_or(0);
        let offset = if back { Self::ALL.len() - 1 } else { 1 };
        Self::ALL[(at + offset) % Self::ALL.len()]
    }

    pub(crate) fn cache_key(self) -> Self {
        if self == Self::Inbox {
            Self::Resume
        } else {
            self
        }
    }
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
        self.answers.get(self.run_id()?)?.get(&view.cache_key())
    }

    pub fn detail(&self) -> Option<&Reply> {
        self.answer(self.view)
    }

    pub fn gate_status(&self) -> Option<&Reply> {
        self.answer(View::Gates)
    }
}
