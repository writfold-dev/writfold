//! Which verbs each knowledge-base view asks, and asking them once per
//! stamp: the projects, the view's answers, `stale --cached` for the whole
//! KB and the selected record's inspector.
use crate::app::App;
use crate::kb_source::TRACKER;
use crate::kb_state::{Inspected, KbView, Scope};
use crate::source::{Reply, Source, field};
use serde_json::Value;
use writfold_kb::verbs::impact::Direction;

/// One verb call a view needs, named by its cache key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    Listed(&'static [&'static str], Option<String>),
    Record(String),
    Brief(Option<String>),
    Walk(String, Direction),
}

impl Ask {
    pub fn key(&self) -> String {
        match self {
            Self::Listed(kinds, project) => {
                format!(
                    "find|{}|{}",
                    kinds.join(","),
                    project.as_deref().unwrap_or("*")
                )
            }
            Self::Record(id) => format!("status|{id}"),
            Self::Brief(name) => format!("brief|{}", name.as_deref().unwrap_or("*")),
            Self::Walk(id, direction) => format!("walk|{id}|{}", direction.name()),
        }
    }

    fn call(&self, source: &Source) -> Reply {
        match self {
            Self::Listed(kinds, project) => source.listed(kinds, project.as_deref()),
            Self::Record(id) => source.record(id),
            Self::Brief(Some(name)) => source.brief(Some(name)),
            Self::Brief(None) => source.projects(),
            Self::Walk(id, direction) => source.walk(id, *direction),
        }
    }
}

impl App {
    /// The project rows of `brief --all`, in verb order.
    pub fn kb_projects(&self) -> Vec<&Value> {
        self.kb
            .projects
            .iter()
            .flat_map(|reply| &reply.rows)
            .filter(|row| field(row, "kind") == "project")
            .collect()
    }

    pub fn kb_scope(&self) -> Scope {
        self.kb.scope.clone().unwrap_or(Scope::All)
    }

    /// The impact view's root: the record `a` chose, else the project.
    pub fn kb_subject(&self) -> Option<String> {
        self.kb
            .subject
            .clone()
            .or_else(|| self.kb_scope().id().map(str::to_owned))
    }

    /// The verb calls behind one view.
    pub fn kb_asks(&self, view: KbView) -> Vec<Ask> {
        let scope = self.kb_scope();
        let name = scope.name().map(str::to_owned);
        match (view, &scope) {
            (KbView::Why, Scope::Project(id)) => {
                vec![Ask::Listed(&["decision"], name), Ask::Record(id.clone())]
            }
            (KbView::Why, Scope::All) => vec![Ask::Listed(&["decision"], None)],
            (KbView::Status, _) => vec![Ask::Brief(name)],
            (KbView::Tracker, Scope::Project(id)) => vec![Ask::Walk(id.clone(), Direction::In)],
            (KbView::Tracker, Scope::All) => vec![Ask::Listed(&TRACKER, None)],
            (KbView::Impact, _) => self
                .kb_subject()
                .map(|subject| Ask::Walk(subject, self.kb.direction))
                .into_iter()
                .collect(),
            (KbView::Stale, _) => Vec::new(),
        }
    }

    /// Every cached KB answer: what the frame may quote.
    pub fn kb_replies(&self) -> Vec<&Reply> {
        let records = self.kb.records.values().flat_map(|r| {
            [Some(&r.status), r.why.as_ref(), r.timeline.as_ref()]
                .into_iter()
                .flatten()
        });
        let runs = self.answers.values().flat_map(|views| views.values());
        self.kb
            .answers
            .values()
            .chain(self.kb.projects.iter())
            .chain(self.kb.stale.iter())
            .chain(records)
            .chain(runs)
            .chain(std::iter::once(&self.runs))
            .collect()
    }

    pub fn kb_reply(&self, ask: &Ask) -> Option<&Reply> {
        self.kb.answers.get(&ask.key())
    }

    fn kb_ask(&mut self, ask: &Ask) {
        if !self.kb.answers.contains_key(&ask.key()) {
            let reply = ask.call(&self.source);
            self.kb.answers.insert(ask.key(), reply);
            self.asked = self.source.clock();
        }
    }

    /// Ask what the KB frame shows and is not cached: the projects, the
    /// view, the KB-wide `stale --cached` (marks and badge) and the
    /// selected entry's inspector.
    pub fn kb_load(&mut self) {
        if self.kb.projects.is_none() {
            self.kb.projects = Some(self.source.projects());
            self.asked = self.source.clock();
        }
        if self.kb.scope.is_none() {
            self.kb.scope = Some(self.first_scope());
        }
        if self.kb.stale.is_none() {
            self.kb.stale = Some(self.source.stale(None));
        }
        for ask in self.kb_asks(self.kb.view) {
            self.kb_ask(&ask);
        }
        if let Some(entry) = self.kb_entry() {
            self.kb_inspect_load(&entry.id, &entry.kind);
        } else if let Some(id) = self.kb_scope().id().map(str::to_owned) {
            self.kb_inspect_load(&id, "project");
        }
    }

    /// The selected run's project when it is listed, else the first project.
    fn first_scope(&self) -> Scope {
        let ids: Vec<String> = self
            .kb_projects()
            .iter()
            .map(|row| field(row, "id").to_owned())
            .collect();
        let run = self.project().map(|name| format!("project:{name}"));
        run.filter(|id| ids.contains(id))
            .or_else(|| ids.first().cloned())
            .map_or(Scope::All, Scope::Project)
    }

    pub(crate) fn kb_inspect_load(&mut self, id: &str, kind: &str) {
        if self.kb.records.contains_key(id) {
            return;
        }
        // A citation's target and a code node are not records: who cites
        // them is `uses` (for code); other targets have only their row.
        let code = matches!(kind, "code" | "cite");
        let inspected = Inspected {
            status: match (code, id.starts_with("code:")) {
                (true, true) => self.source.uses(id),
                (true, false) => Reply::default(),
                (false, _) => self.source.record(id),
            },
            why: (kind == "decision").then(|| self.source.why(id)),
            timeline: (!code).then(|| self.source.timeline(id)),
        };
        self.kb.records.insert(id.to_owned(), inspected);
        self.asked = self.source.clock();
    }
}
