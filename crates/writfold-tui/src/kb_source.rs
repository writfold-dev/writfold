//! The read verbs knowledge-base mode asks, called in-process like every
//! other pane: each method is one `writfold kb` verb with `--json`.
use crate::source::{Reply, Source};
use writfold_kb::Verb;
use writfold_kb::verbs::find::Filter;
use writfold_kb::verbs::impact::Direction;

/// The kinds the tracker view lists.
pub const TRACKER: [&str; 3] = ["ticket", "investigation", "signature"];

impl Source {
    /// `writfold kb brief --all`: the projects and the KB's stamps.
    pub fn projects(&self) -> Reply {
        self.call(Verb::Brief {
            project: None,
            all: true,
        })
    }

    /// `writfold kb find --kind K[,K] [--project P]` with no text.
    pub fn listed(&self, kinds: &[&str], project: Option<&str>) -> Reply {
        self.call(Verb::Find {
            text: String::new(),
            filter: Filter {
                kinds: kinds.iter().map(|k| (*k).to_owned()).collect(),
                project: project.map(str::to_owned),
            },
            signatures: false,
        })
    }

    /// `writfold kb status ID`.
    pub fn record(&self, id: &str) -> Reply {
        self.call(Verb::Status { target: id.into() })
    }

    /// `writfold kb why ID`.
    pub fn why(&self, id: &str) -> Reply {
        self.call(Verb::Why {
            text: id.into(),
            sections: Vec::new(),
        })
    }

    /// `writfold kb uses ID`, for nodes outside the KB such as `code:`.
    pub fn uses(&self, id: &str) -> Reply {
        self.call(Verb::Uses { target: id.into() })
    }

    /// `writfold kb impact ID --direction D --depth 2`.
    pub fn walk(&self, id: &str, direction: Direction) -> Reply {
        self.call(Verb::Impact {
            target: id.into(),
            depth: 2,
            direction,
            inferred: false,
        })
    }
}
