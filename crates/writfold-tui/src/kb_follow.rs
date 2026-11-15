//! `f`: from a run to the first record its checkpoints name (`timeline
//! RUN`), and from a record to a listed run whose checkpoint names it
//! (`timeline ID`'s `checkpoint` rows, whose id is the run). Runs and
//! records meet only through checkpoint text, so the frame says `named`.
//! Also the two in-mode jumps: `n`/`N` to a marked entry, `a` to impact.
use crate::app::{App, Focus, View};
use crate::kb_state::{KbView, Mode, Scope};
use crate::source::field;

impl App {
    /// Run mode `f`: KB mode on the first record the run's checkpoints
    /// name: in `why` when it is listed there, else `impact` rooted on it.
    pub(crate) fn follow_run(&mut self) {
        self.load_view(View::Timeline);
        let named = self.answer(View::Timeline).and_then(|timeline| {
            timeline
                .rows
                .iter()
                .find(|r| !matches!(field(r, "kind"), "run" | "checkpoint" | "plan"))
                .map(|r| field(r, "id").to_owned())
        });
        let project = self.project().map(|name| format!("project:{name}"));
        self.enter_kb();
        let Some(id) = named else {
            self.notice = Some("this run's checkpoints name no record".into());
            return;
        };
        let listed = self
            .kb_projects()
            .iter()
            .any(|r| Some(field(r, "id")) == project.as_deref());
        if let Some(project) = project.filter(|_| listed) {
            self.kb_scope_to(Scope::Project(project));
        }
        self.kb.focus = Focus::Detail;
        self.kb_show(KbView::Why);
        if self.kb_entries().iter().any(|e| !e.header && e.id == id) {
            self.kb.selected.insert(KbView::Why, id);
        } else {
            self.kb.subject = Some(id);
            self.kb.selected.remove(&KbView::Impact);
            self.kb_show(KbView::Impact);
        }
        self.kb_load();
    }

    /// KB mode `f`: run mode on a listed run whose checkpoint names the
    /// selected record (its Timeline), or on a `run` entry (its Progress).
    pub(crate) fn follow_record(&mut self) {
        let Some(entry) = self.kb_entry() else {
            self.notice = Some("no listed run names this record".into());
            return;
        };
        let (runs, view) = if entry.kind == "run" {
            (vec![entry.id.clone()], View::Progress)
        } else {
            self.kb_inspect_load(&entry.id, &entry.kind);
            let named = self
                .kb
                .records
                .get(&entry.id)
                .and_then(|r| r.timeline.as_ref())
                .map(|timeline| {
                    timeline
                        .rows
                        .iter()
                        .filter(|r| field(r, "kind") == "checkpoint")
                        .map(|r| field(r, "id").to_owned())
                        .collect()
                })
                .unwrap_or_default();
            (named, View::Timeline)
        };
        let listed = runs.iter().find_map(|id| {
            self.order
                .iter()
                .position(|i| self.runs.rows.get(*i).is_some_and(|r| field(r, "id") == id))
        });
        let Some(at) = listed else {
            self.notice = Some("no listed run names this record".into());
            return;
        };
        self.mode = Mode::Runs;
        if at != self.selected {
            self.selected = at;
            self.scroll = 0;
            self.anchor = true;
            self.node = None;
            self.refresh_subject();
        }
        self.show(view);
    }

    /// The next (or previous) entry carrying a mark, wrapping.
    pub(crate) fn kb_marked(&mut self, forward: bool) {
        let stale = self.kb.stale.clone();
        let entries: Vec<_> = self
            .kb_entries()
            .into_iter()
            .filter(|e| !e.header)
            .collect();
        let count = entries.len();
        let at = self
            .kb_entry()
            .and_then(|e| entries.iter().position(|x| x.key == e.key))
            .unwrap_or(0);
        let found = (1..=count)
            .map(|delta| {
                let index = if forward {
                    at + delta
                } else {
                    at + count - delta
                };
                &entries[index % count]
            })
            .find(|e| {
                let marks = crate::kb_entry::marks(e, stale.as_ref());
                marks.moved || marks.unknown
            });
        if let Some(entry) = found {
            self.kb.selected.insert(self.kb.view, entry.key.clone());
            self.kb.inspect_scroll = 0;
            self.kb_load();
        }
    }

    /// The impact of the selected entry (the project in `status`).
    pub(crate) fn kb_impact(&mut self) {
        let id = match self.kb_entry() {
            Some(entry) if self.kb.view == KbView::Stale && entry.kind == "cite" => {
                code_file(&entry.id).to_owned()
            }
            Some(entry) => entry.id,
            None => match self.kb_scope().id() {
                Some(id) => id.to_owned(),
                None => return,
            },
        };
        self.kb.subject = Some(id);
        self.kb.selected.remove(&KbView::Impact);
        self.kb_show(KbView::Impact);
    }
}

/// A cited code range's file node: `code:REPO:PATH:A-B` without `:A-B`,
/// which is how `impact` names the file the citation points into.
fn code_file(id: &str) -> &str {
    if !id.starts_with("code:") {
        return id;
    }
    match id.rsplit_once(':') {
        Some((file, range))
            if range.split('-').count() == 2
                && range
                    .split('-')
                    .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) =>
        {
            file
        }
        _ => id,
    }
}
