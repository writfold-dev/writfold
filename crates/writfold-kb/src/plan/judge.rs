//! Each plan step's gates, judged now. A repository or assessed gate
//! stands as `gate status` would judge it on the present bytes; a run-record gate is
//! judged against the run-record digest of the step's own window, so a
//! done step's run-record evidence keeps standing after later steps add
//! records. A delivery (`deliver:<mode>`) is judged on its candidate
//! ([`super::delivery`]). Only done steps and the current step are judged:
//! later steps owe nothing yet.
use super::fold::{Graph, Step};
use super::records::{self, Records};
use crate::error::Result;
use crate::gate::status::{self, Filings, Judged};
use crate::gate::subject::Subject;
use crate::session::Session;
use writfold_core::gate::Standing;
use writfold_core::plan::{GateFact, fact_met};

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
    /// The command that meets it, when it is not the gate's own (a
    /// checklist's first owed item, a delivery's first owed obligation).
    pub command: Option<String>,
    pub class: String,
    pub counts: Option<(usize, usize)>,
    pub items: Vec<crate::emit::More>,
    /// Set when the gate is a delivery (`deliver:<mode>`).
    pub delivery: Option<super::delivery::Delivered>,
    /// The definition's `accept`, when written ([`crate::gate::def_accept`]).
    pub accept: Option<Vec<String>>,
}

impl Gate {
    /// What the plan kernel decides on: no usable definition, or the standing.
    pub fn fact(&self) -> GateFact {
        match self.standing {
            None => GateFact::Undefined,
            Some(standing) => GateFact::Stands(
                standing,
                crate::gate::def_accept::policy(self.accept.as_deref()),
            ),
        }
    }

    pub fn met(&self) -> bool {
        fact_met(&self.fact())
    }

    pub fn shown(&self) -> &'static str {
        self.standing.map_or("undefined", Standing::name)
    }

    /// The standing as shown in a list of what is owed: `waived, not
    /// accepted` when the gate's `accept` leaves out its met word.
    pub fn display(&self) -> String {
        match self.standing {
            Some(s) if s.met() && !self.met() => format!("{}, not accepted", s.name()),
            _ => self.shown().to_owned(),
        }
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
        command: None,
        class: String::new(),
        counts: None,
        items: Vec::new(),
        delivery: None,
        accept: None,
    }
}

/// Why an owed gate has no usable definition, when its file has errors:
/// `reason: <file> cannot be read: line 6: unknown key timout_seconds`.
fn unreadable(problems: &[crate::gate::def::Unusable], id: &str) -> Option<crate::emit::More> {
    let file = format!("{}/{id}.toml", crate::gate::def::DIR);
    let (_, problem) = problems.iter().find(|(_, p)| p.path == file)?;
    let first = problem.report.diagnostics.first()?;
    let line = first.at.map(|a| a.line);
    let at = line.map_or_else(String::new, |l| format!("line {l}: "));
    Some(crate::emit::More {
        label: "reason",
        text: format!(
            "{} cannot be read: {at}{}",
            first.file,
            first.message.replace('`', "")
        ),
        path: Some(first.file.clone()),
        line,
        ..crate::emit::More::default()
    })
}

fn gate(j: &Judged<'_>, repo: &str, detail: bool) -> Gate {
    let (evidence, content) = evidence(j);
    let d = &j.definition;
    Gate {
        accept: d.accept.clone(),
        id: d.id.clone(),
        standing: Some(j.standing),
        evidence,
        definition: d.digest.clone(),
        content,
        read: status::read_line(d, repo).and_then(|m| m.path),
        attested: d.attested(),
        run_record: d.run_record(),
        command: (j.checklist.as_ref()).and_then(|v| crate::gate::checklist::next(&d.id, v)),
        class: if detail {
            d.class.clone()
        } else {
            String::new()
        },
        counts: detail
            .then(|| j.checklist.as_ref().map(|v| v.counts()))
            .flatten(),
        items: if detail {
            j.checklist.as_ref().map_or_else(Vec::new, |v| {
                v.entries
                    .iter()
                    .map(crate::gate::checklist::item_line)
                    .collect()
            })
        } else {
            Vec::new()
        },
        delivery: None,
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
    judge_steps(session, run_dir, run, graph, subject, records, false)
}

/// Like `steps`, with checklist continuations for detailed plan status.
pub fn steps_detailed(
    session: &Session<'_>,
    run_dir: &str,
    run: &str,
    graph: &Graph,
    subject: &Subject,
    records: &Records,
) -> Result<Vec<Vec<Gate>>> {
    judge_steps(session, run_dir, run, graph, subject, records, true)
}

fn judge_steps(
    session: &Session<'_>,
    run_dir: &str,
    run: &str,
    graph: &Graph,
    subject: &Subject,
    records: &Records,
    detail: bool,
) -> Result<Vec<Vec<Gate>>> {
    let mut filings = Filings::read(&session.ws.root, run_dir, run);
    filings.owed.gates = Some(graph.gates());
    let report = status::judge(session, subject, &filings)?;
    let valid = filings.parsed();
    let deliveries = super::delivery::Deliveries::read(&session.ws.root, run_dir, run, graph);
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
            if let Some(mode) = crate::deliver::step::mode_of(id) {
                gates.push(deliveries.gate(session, step, mode, detail));
                continue;
            }
            let Some(j) = report.gates.iter().find(|j| &j.definition.id == id) else {
                let mut g = undefined(id);
                g.items.extend(unreadable(&report.problems, id));
                gates.push(g);
                continue;
            };
            if !j.definition.run_record() {
                gates.push(gate(j, &subject.name, detail));
                continue;
            }
            let d = j.definition.clone();
            let judged = status::judge_gate(d, &valid, &filings, &subject.name, &digest);
            gates.push(gate(&judged, &subject.name, detail));
            // A run-record gate's content is its window, receipt or not.
            if let Some(last) = gates.last_mut() {
                last.content = digest.clone();
            }
        }
        out.push(gates);
    }
    Ok(out)
}
