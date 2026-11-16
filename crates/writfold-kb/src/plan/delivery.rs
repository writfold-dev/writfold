//! A step's deliveries (`deliver:<mode>`), judged as gates. A done step's
//! delivery is judged on the candidate its own done record names, and
//! only that one, for ever: that candidate is bound and decides no other
//! step. The current step's delivery is judged on the newest candidate of
//! the mode that no done record binds ([`crate::deliver::step`]). The
//! delivery stands as one standing, the weakest word that is true
//! ([`writfold_core::delivery::standing`]), and reaches the proved plan
//! kernel through [`writfold_core::gate::Standing::met`] like any gate.
use super::fold::{Graph, Step};
use super::judge::Gate;
use crate::deliver::candidate::{self, Candidate};
use crate::deliver::step::{self, PREFIX, View};
use crate::emit::More;
use crate::session::Session;

/// What a delivery adds to its gate's lines.
pub struct Delivered {
    /// `delivery <met>/<total> checks met`, or `delivery no candidate`.
    pub summary: String,
    /// The `candidate:` line of `plan status`, when a candidate decides.
    pub candidate: Option<More>,
    /// The `candidate:` line of `plan status --gates`.
    pub detail: Option<More>,
    /// Why nothing can meet it now (the mode does not load or declares
    /// nothing): shown in place of the candidate line, with no `next:`.
    pub why: Option<String>,
}

/// The run's candidates and the ones done records bind.
pub struct Deliveries {
    run_dir: String,
    run: String,
    cands: Vec<Candidate>,
    bound: Vec<String>,
}

impl Deliveries {
    pub fn read(root: &std::path::Path, run_dir: &str, run: &str, graph: &Graph) -> Self {
        let any = graph
            .steps
            .iter()
            .any(|s| s.gates.iter().any(|g| step::mode_of(g).is_some()));
        let cands = match any {
            true => candidate::list(root, run_dir, run),
            false => Vec::new(),
        };
        let bound = graph
            .steps
            .iter()
            .filter_map(|s| s.done.as_ref())
            .flat_map(|(_, d)| &d.met)
            .filter(|m| step::mode_of(&m.gate).is_some())
            .filter_map(|m| m.evidence.as_deref())
            .filter_map(|e| step::bound_id(run_dir, e))
            .map(str::to_owned)
            .collect();
        Self {
            run_dir: run_dir.to_owned(),
            run: run.to_owned(),
            cands,
            bound,
        }
    }

    /// The candidate that decides `mode` at `s`: a done step's own, else
    /// the deciding one.
    fn decides(&self, s: &Step, mode: &str) -> Option<&Candidate> {
        let Some((_, done)) = &s.done else {
            return step::deciding(&step::of_mode(&self.cands, mode), &self.bound);
        };
        let word = format!("{PREFIX}{mode}");
        let met = done.met.iter().find(|m| m.gate == word)?;
        let id = step::bound_id(&self.run_dir, met.evidence.as_deref()?)?;
        self.cands.iter().find(|c| c.id == id)
    }

    /// The delivery `mode` of step `s`, as a gate.
    pub fn gate(&self, session: &Session<'_>, s: &Step, mode: &str, detail: bool) -> Gate {
        let c = self.decides(s, mode);
        let v = step::view(session, mode, c, &self.run);
        let seq = c
            .and_then(|c| c.manifest.as_ref().ok())
            .map_or(0, |m| m.seq);
        build(v, seq, detail)
    }
}

fn line(label: &'static str, text: String, path: Option<&String>) -> More {
    More {
        label,
        text,
        path: path.cloned(),
        line: path.map(|_| 1),
        ..More::default()
    }
}

fn build(v: View, seq: u64, detail: bool) -> Gate {
    let word = format!("{PREFIX}{}", v.mode);
    let seen = v.candidate.as_ref();
    let command = match (&v.why, seen) {
        (Some(_), _) => None,
        (None, Some(seen)) => seen.next.clone(),
        (None, None) => Some(format!(
            "writfold kb deliver prepare --mode {} --from FILE",
            v.mode
        )),
    };
    let summary = match (seen, &v.why) {
        (Some(_), _) => format!("delivery {}/{} checks met", v.met, v.total),
        (None, Some(why)) => format!("delivery: {why}"),
        (None, None) => "delivery no candidate".into(),
    };
    let candidate = seen.map(|s| {
        let state = s.row.status.clone().unwrap_or_default();
        let owes = match s.owed.is_empty() {
            true => String::new(),
            false => format!(" owes {}", s.owed.join(", ")),
        };
        line(
            "candidate",
            format!("{} [{state}]{owes}", s.id),
            Some(&s.payload),
        )
    });
    let detail_line = seen.filter(|_| detail).map(|s| More {
        status: s.row.status.clone(),
        flags: s.row.flags.clone(),
        ..line(
            "candidate",
            format!(
                "{} [{}] {} seq={seq}",
                s.id,
                s.row.status.as_deref().unwrap_or_default(),
                s.row.text
            ),
            Some(&s.payload),
        )
    });
    let items = match (detail, seen) {
        (true, Some(s)) => s
            .row
            .more
            .iter()
            .filter(|m| m.label == "check")
            .map(|m| More {
                label: "item",
                ..m.clone()
            })
            .collect(),
        _ => Vec::new(),
    };
    Gate {
        id: word,
        standing: Some(v.standing),
        evidence: seen.map(|s| s.payload.clone()),
        definition: v.definition,
        content: seen.map_or_else(String::new, |s| s.content.clone()),
        read: seen.and_then(|s| s.read.clone()),
        attested: false,
        run_record: false,
        command,
        class: if detail {
            "delivery".into()
        } else {
            String::new()
        },
        counts: None,
        items,
        delivery: Some(Delivered {
            summary,
            candidate,
            detail: detail_line,
            why: v.why,
        }),
    }
}
