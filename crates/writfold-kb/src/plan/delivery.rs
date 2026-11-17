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
use crate::deliver::step::{self, PREFIX, Seen, View};
use crate::emit::{More, Row};
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
    /// The candidate's `posted:` line, when a post record exists.
    pub posted: Option<More>,
}

/// The run's candidates and the ones done records bind.
pub struct Deliveries {
    pub(super) run_dir: String,
    pub(super) run: String,
    pub(super) cands: Vec<Candidate>,
    pub(super) bound: Vec<String>,
    pub(super) withdrawn: Vec<String>,
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
        let withdrawn = crate::deliver::withdraw::ids(root, &cands, run);
        Self {
            run_dir: run_dir.to_owned(),
            run: run.to_owned(),
            cands,
            bound,
            withdrawn,
        }
    }

    /// The candidate that decides `mode` at `s`: a done step's own, else
    /// the deciding one.
    pub(super) fn decides(&self, s: &Step, mode: &str) -> Option<&Candidate> {
        let Some((_, done)) = &s.done else {
            let mine = step::of_mode(&self.cands, mode);
            return step::deciding(&mine, &self.bound, &self.withdrawn);
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
        let mut g = build(v, seq, detail);
        let posted = c.and_then(|c| {
            let posts = crate::deliver::post::all(&session.ws.root, c, &self.run);
            crate::deliver::post::line(&posts, &c.digest())
        });
        if let Some(d) = g.delivery.as_mut() {
            d.posted = posted;
        }
        g
    }
}

pub(super) fn line(label: &'static str, text: String, path: Option<&String>) -> More {
    More {
        label,
        text,
        path: path.cloned(),
        line: path.map(|_| 1),
        ..More::default()
    }
}

/// The `candidate:` line of `plan status`: what the candidate still owes.
fn owing(s: &Seen) -> More {
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
}

/// The `candidate:` line of `plan status --gates`, and one `item` per
/// obligation (the `check:` lines of `deliver status`).
fn detailed(s: &Seen, seq: u64) -> (More, Vec<More>) {
    let status = s.row.status.as_deref().unwrap_or_default();
    let text = format!("{} [{status}] {} seq={seq}", s.id, s.row.text);
    let candidate = More {
        status: s.row.status.clone(),
        flags: s.row.flags.clone(),
        ..line("candidate", text, Some(&s.payload))
    };
    let items = s.row.more.iter().filter(|m| m.label == "check");
    let items = items.map(|m| More {
        label: "item",
        ..m.clone()
    });
    (candidate, items.collect())
}

fn build(v: View, seq: u64, detail: bool) -> Gate {
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
    // Under `--gates` the detailed candidate line takes the plain one's place.
    let (candidate, (detail_line, items)) = match (seen, detail) {
        (Some(s), true) => {
            let (line, items) = detailed(s, seq);
            (None, (Some(line), items))
        }
        (Some(s), false) => (Some(owing(s)), (None, Vec::new())),
        (None, _) => (None, (None, Vec::new())),
    };
    Gate {
        // The plan meets the delivery exactly when its candidate is ready
        // (`delivery::fact`): every word taken, or none.
        accept: (!v.taken).then(Vec::new),
        id: format!("{PREFIX}{}", v.mode),
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
            posted: None,
        }),
    }
}

/// The current step's `candidate:` line for each delivery a candidate
/// decides, or `why:` when nothing can meet it.
pub fn lines(row: &mut Row, gates: &[Gate]) {
    for d in gates.iter().filter_map(|g| g.delivery.as_ref()) {
        let said = |row: &Row, m: &More| {
            row.more
                .iter()
                .any(|x| x.label == m.label && x.text == m.text)
        };
        match (&d.candidate, &d.why) {
            (Some(c), _) if said(row, c) => {}
            (Some(c), _) => row.more.push(c.clone()),
            (None, Some(why)) => row.more.push(line("why", why.clone(), None)),
            (None, None) => {}
        }
        let shown = row.more.iter().any(|m| m.label == "posted");
        if let (Some(p), false) = (&d.posted, shown) {
            row.more.push(p.clone());
        }
    }
}

pub use super::delivery_mark::{bound_steps, mark};
