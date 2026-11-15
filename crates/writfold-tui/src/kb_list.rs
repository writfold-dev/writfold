//! The current view's entries, projected from the cached answers, and the
//! selected one.
use crate::app::App;
use crate::kb_entry::{Entry, from_row, listed, stale};
use crate::kb_state::{KbView, Scope};
use crate::source::{Reply, field};

impl App {
    fn kb_view_reply(&self, at: usize) -> Option<&Reply> {
        self.kb_asks(self.kb.view)
            .get(at)
            .and_then(|ask| self.kb_reply(ask))
    }

    /// The `why` list: `find --kind decision --project P`, then any
    /// decision `status project:P` names that the listing did not.
    fn why_entries(&self) -> Vec<Entry> {
        let Some(found) = self.kb_view_reply(0) else {
            return Vec::new();
        };
        let mut out = listed(found, "decisions");
        if let Some(status) = self.kb_view_reply(1) {
            for row in &status.rows {
                let id = field(row, "id");
                if field(row, "kind") == "decision" && !out.iter().any(|e| e.id == id) {
                    out.push(from_row(row, field(row, "text")));
                }
            }
        }
        out
    }

    /// The current view's entries, headers included.
    pub fn kb_entries(&self) -> Vec<Entry> {
        match self.kb.view {
            KbView::Why => self.why_entries(),
            KbView::Status => Vec::new(),
            KbView::Tracker => match (self.kb_scope(), self.kb_view_reply(0)) {
                (Scope::All, Some(reply)) => listed(reply, "tracker items"),
                (Scope::Project(id), Some(reply)) => crate::kb_tree::tracker(reply, &id),
                _ => Vec::new(),
            },
            KbView::Impact => match (self.kb_subject(), self.kb_view_reply(0)) {
                (Some(subject), Some(reply)) => crate::kb_tree::tree(reply, &subject),
                _ => Vec::new(),
            },
            KbView::Stale => self.kb.stale.as_ref().map(stale).unwrap_or_default(),
        }
    }

    /// The reply behind the current list, for its notes, refusal and trailer.
    pub fn kb_list_reply(&self) -> Option<&Reply> {
        match self.kb.view {
            KbView::Stale => self.kb.stale.as_ref(),
            _ => self.kb_view_reply(0),
        }
    }

    /// The selected entry: the remembered key, else the first entry.
    pub fn kb_entry(&self) -> Option<Entry> {
        let all: Vec<Entry> = self
            .kb_entries()
            .into_iter()
            .filter(|e| !e.header)
            .collect();
        let wanted = self.kb.selected.get(&self.kb.view);
        wanted
            .and_then(|key| all.iter().find(|e| &e.key == key))
            .or_else(|| all.first())
            .cloned()
    }
}
