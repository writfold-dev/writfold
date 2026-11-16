//! Knowledge-base mode: its own view, focus, selection and scroll, and the
//! verb answers it asked, cached until a stamp moves (`reload` clears them).
use crate::app::Focus;
use crate::source::Reply;
use std::collections::BTreeMap;
use writfold_kb::verbs::impact::Direction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Runs,
    Kb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KbView {
    Why,
    Status,
    Tracker,
    Impact,
    Stale,
}

impl KbView {
    pub const ALL: [Self; 5] = [
        Self::Why,
        Self::Status,
        Self::Tracker,
        Self::Impact,
        Self::Stale,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Why => "why",
            Self::Status => "status",
            Self::Tracker => "tracker",
            Self::Impact => "impact",
            Self::Stale => "stale",
        }
    }

    pub fn digit(self) -> char {
        match self {
            Self::Why => '1',
            Self::Status => '2',
            Self::Tracker => '3',
            Self::Impact => '4',
            Self::Stale => '5',
        }
    }

    /// `why`, `status`, … as `--view kb:NAME` names them (without `kb:`).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.name() == name)
    }

    pub(crate) fn next(self, back: bool) -> Self {
        let at = Self::ALL.iter().position(|v| *v == self).unwrap_or(0);
        let offset = if back { Self::ALL.len() - 1 } else { 1 };
        Self::ALL[(at + offset) % Self::ALL.len()]
    }
}

/// What the views are about: one project record, or every record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    Project(String),
    All,
}

impl Scope {
    /// The project's name as `brief` and `find --project` take it.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Project(id) => Some(id.strip_prefix("project:").unwrap_or(id)),
            Self::All => None,
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Project(id) => Some(id),
            Self::All => None,
        }
    }
}

/// The answers the inspector quotes for one record.
#[derive(Clone, Debug, Default)]
pub struct Inspected {
    pub status: Reply,
    pub why: Option<Reply>,
    pub timeline: Option<Reply>,
}

pub struct KbState {
    pub scope: Option<Scope>,
    pub view: KbView,
    pub focus: Focus,
    pub selected: BTreeMap<KbView, String>,
    pub subject: Option<String>,
    pub direction: Direction,
    pub scroll: u16,
    pub inspect_scroll: u16,
    pub projects_view: bool,
    pub(crate) offset: usize,
    pub(crate) projects: Option<Reply>,
    pub(crate) answers: BTreeMap<String, Reply>,
    pub(crate) records: BTreeMap<String, Inspected>,
    pub stale: Option<Reply>,
}

impl Default for KbState {
    fn default() -> Self {
        Self {
            scope: None,
            view: KbView::Why,
            focus: Focus::Detail,
            selected: BTreeMap::new(),
            subject: None,
            direction: Direction::In,
            scroll: 0,
            inspect_scroll: 0,
            projects_view: false,
            offset: 0,
            projects: None,
            answers: BTreeMap::new(),
            records: BTreeMap::new(),
            stale: None,
        }
    }
}

impl KbState {
    /// The cached inspector answers for one record.
    pub fn record(&self, id: &str) -> Option<&Inspected> {
        self.records.get(id)
    }

    /// Drop every cached answer; selection, view and focus stay.
    pub(crate) fn clear(&mut self) {
        self.projects = None;
        self.answers.clear();
        self.records.clear();
        self.stale = None;
    }
}

pub use crate::kb_load::Ask;
