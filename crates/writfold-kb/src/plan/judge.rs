//! Each plan step's gates, judged now. A repository or assessed gate
//! stands as `gate status` would judge it on the present bytes; a run-record gate is
//! judged against the run-record digest of the step's own window, so a
//! done step's run-record evidence keeps standing after later steps add
//! records. Only done steps and the current step are judged: later steps
//! owe nothing yet.
use super::fold::{Graph, Step};
use super::records::{self, Records};
use crate::error::Result;
use crate::gate::status::{self, Filings, Judged};
use crate::gate::subject::Subject;
use crate::session::Session;
use writfold_core::gate::Standing;

/// One gate of one step, as it stands now.
pub struct Gate {
    pub id: String,
    /// `None` when the gate has no usable definition.
    pub standing: Option<Standing>,
    /// The receipt or statement that stands for it, if any.
    pub evidence: Option<String>,
    pub definition: String,
    pub content: String,
    /// Where to read the gate's instructions, when it names some.
    pub read: Option<String>,
    pub attested: bool,
    pub run_record: bool,
    /// A checklist gate's command for its first owed item.
    pub cover: Option<String>,
}

impl Gate {
    pub fn met(&self) -> bool {
        self.standing.is_some_and(Standing::met)
    }

    pub fn shown(&self) -> &'static str {
        self.standing.map_or("undefined", Standing::name)
    }
}

/// The window digest of `step`: from where it began to where it ended,
/// or to now while it is open.
pub fn window(step: &Step, records: &Records) -> String {
    let now = records.baseline();
    records::digest(Some(&step.begin), step.end().unwrap_or(&now), records)
}

/// The evidence path of a judged gate: the statement that stands, else
/// the receipt (or Weft assessment) that decides its state, or a
/// checklist's newest covering row; and the content digest it is about.
fn evidence(j: &Judged<'_>) -> (Option<String>, String) {
    let said = match j.standing {
        Standing::Attested => j.statements.attested,
        Standing::Waived => j.statements.waived,
        Standing::Checked(_) => None,
    };
    if let Some((f, s)) = said {
        return (Some(f.path.clone()), s.content.clone());
    }
    if let Some((f, r)) = j.receipt {
        return (Some(f.path.clone()), r.scope.clone());
    }
    if let Some(view) = &j.checklist {
        return (view.newest().map(|f| f.path.clone()), view.content.clone());
    }
    match j.assessment.as_ref().and_then(|a| a.receipt) {
        Some((f, r)) => (Some(f.path.clone()), r.source.clone()),
        None => (None, String::new()),
    }
}

fn undefined(id: &str) -> Gate {
    Gate {
        id: id.to_owned(),
        standing: None,
        evidence: None,
        definition: String::new(),
        content: String::new(),
        read: None,
        attested: false,
        run_record: false,
        cover: None,
    }
}

fn gate(j: &Judged<'_>, repo: &str) -> Gate {
    let (evidence, content) = evidence(j);
    let d = &j.definition;
    Gate {
        id: d.id.clone(),
        standing: Some(j.standing),
        evidence,
        definition: d.digest.clone(),
        content,
        read: status::read_line(d, repo).and_then(|m| m.path),
        attested: d.attested(),
        run_record: d.run_record(),
        cover: (j.checklist.as_ref()).and_then(|v| crate::gate::checklist::next(&d.id, v)),
    }
}

/// Judge the gates of every done step and of the current one; later steps
/// get an empty list. Repository and assessed gates are judged once, as
/// `gate status` judges them; run-record gates once per step, against the
/// step's own window. Needs the subject for any gate to stand.
pub fn steps(
    session: &Session<'_>,
    run_dir: &str,
    run: &str,
    graph: &Graph,
    subject: &Subject,
    records: &Records,
) -> Result<Vec<Vec<Gate>>> {
    let mut filings = Filings::read(&session.ws.root, run_dir, run);
    filings.owed.gates = Some(graph.gates());
    let report = status::judge(session, subject, &filings)?;
    let valid = filings.valid(&subject.name);
    let current = graph.current();
    let mut out = Vec::new();
    for (i, step) in graph.steps.iter().enumerate() {
        if step.done.is_none() && current != Some(i) {
            out.push(Vec::new());
            continue;
        }
        let digest = window(step, records);
        let mut gates = Vec::new();
        for id in &step.gates {
            let Some(j) = report.gates.iter().find(|j| &j.definition.id == id) else {
                gates.push(undefined(id));
                continue;
            };
            if !j.definition.run_record() {
                gates.push(gate(j, &subject.name));
                continue;
            }
            let d = j.definition.clone();
            let judged = status::judge_gate(d, &valid, &filings, &subject.name, &digest);
            gates.push(gate(&judged, &subject.name));
            // A run-record gate's content is its window, receipt or not.
            if let Some(last) = gates.last_mut() {
                last.content = digest.clone();
            }
        }
        out.push(gates);
    }
    Ok(out)
}
