//! What `deliver status` and `resume` read of a run's candidates: one row
//! per candidate with its `check:` lines, the trailer's counts. A
//! withdrawn candidate is left out unless it is named or `--all` is given,
//! and is never counted in the standing counts or the exit code.
use super::{candidate, config, post, recheck, status, withdraw};
use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::index::query::Rec;
use crate::runs;
use crate::session::Session;
use candidate::Candidate;

/// The status answer for a run's candidates (or one of them); `all`
/// shows withdrawn candidates too.
pub(super) fn status_answer(
    session: &Session<'_>,
    rec: &Rec,
    named: Option<&str>,
    all: bool,
) -> Result<Answer> {
    let root = &session.ws.root;
    let cands = candidate::list(root, &runs::dir(rec), &rec.id);
    let chosen: Vec<&Candidate> = match named {
        Some(_) => vec![candidate::pick(&cands, named, &rec.id)?],
        None => cands.iter().collect(),
    };
    let mut answer = Answer::default();
    let mut tally = Tally::default();
    let mut modes: Vec<&str> = Vec::new();
    for c in chosen {
        tally.add(
            &mut answer.rows,
            session,
            c,
            (&rec.id, all || named.is_some()),
        )?;
        if let Ok(m) = &c.manifest
            && !modes.contains(&m.mode.as_str())
        {
            modes.push(&m.mode);
        }
    }
    // What a mode file says about itself (its limits) travels with it.
    for name in modes {
        if let Ok(config::Mode {
            description: Some(d),
            ..
        }) = config::mode(root, name)
        {
            answer.notes.push(format!("mode {name}: {d}"));
        }
    }
    crate::plan::delivery::mark(root, rec, &mut answer.rows);
    if answer.rows.is_empty() && tally.withdrawn == 0 {
        answer.unknown = Some("no-candidates");
        answer.notes.push(format!(
            "run {} has no delivery candidate: writfold kb deliver prepare --mode M --from FILE",
            rec.id
        ));
    }
    if tally.hidden > 0 {
        answer.notes.push(format!(
            "{} withdrawn candidate(s) not shown: writfold kb deliver status --all",
            tally.hidden
        ));
    }
    if answer
        .rows
        .iter()
        .any(|r| r.flags.iter().any(|f| f == "edited"))
    {
        answer.notes.push(
            "!edited: the candidate differs from the file it was prepared from; checks are about the candidate".into(),
        );
    }
    answer.counts = tally.counts();
    answer.drawn = (!tally.drawn.is_empty()).then(|| tally.drawn.clone());
    // A statement the policy does not take holds its candidate.
    answer.flagged |= tally.rejected > 0;
    answer.notes.push(
        "standing is derived on every read; checks are mechanical, attestations are statements, and nothing is sent".into(),
    );
    Ok(answer)
}

/// Counts for the trailer.
#[derive(Default)]
struct Tally {
    candidates: usize,
    ready: usize,
    by: [usize; 7],
    invalid: usize,
    withdrawn: usize,
    /// Withdrawn candidates left out of the rows.
    hidden: usize,
    /// Statements the policy does not take (`!not-accepted`).
    rejected: usize,
    /// Findings drawn against the payloads ([`super::draw`]).
    drawn: String,
}

const NAMES: [&str; 7] = [
    "satisfied",
    "attested",
    "waived",
    "open",
    "stale",
    "failed",
    "unknown",
];

/// The row of a withdrawn candidate.
fn withdrawn_row(c: &Candidate, w: &withdraw::Withdrawn) -> Row {
    let mut row = status::row(c, &[]);
    row.status = Some("withdrawn".into());
    row.flags = vec!["withdrawn".into()];
    row.more = vec![More {
        label: "withdrawn",
        text: format!("by {}: {}", w.by, w.reason),
        path: Some(w.path.clone()),
        line: Some(1),
        ..More::default()
    }];
    row
}

impl Tally {
    fn add(
        &mut self,
        rows: &mut Vec<Row>,
        session: &Session<'_>,
        c: &Candidate,
        (run, show_withdrawn): (&str, bool),
    ) -> Result<()> {
        let root = &session.ws.root;
        let filings = status::Filings::read(root, c, run);
        self.candidates += 1;
        if let Some(w) = withdraw::of(root, c, run) {
            self.withdrawn += 1;
            match show_withdrawn {
                true => rows.push(withdrawn_row(c, &w)),
                false => self.hidden += 1,
            }
            return Ok(());
        }
        let invalid = status::invalid(&filings);
        self.invalid += invalid.len();
        if c.manifest.is_err() {
            self.invalid += 1;
            rows.push(status::row(c, &[]));
            rows.extend(invalid);
            return Ok(());
        }
        let judged = recheck::judged(session, c, &filings)?;
        if status::ready(&judged) {
            self.ready += 1;
        }
        self.rejected += judged.iter().filter(|j| j.rejected()).count();
        let mut standings: Vec<&'static str> = judged.iter().map(|j| j.standing.name()).collect();
        if judged.is_empty() {
            // A mode that declares nothing: the delivery itself stands open.
            standings.push(writfold_core::delivery::standing(&[]).name());
        }
        for name in standings {
            if let Some(i) = NAMES.iter().position(|n| *n == name) {
                self.by[i] += 1;
            }
        }
        let drawn = super::draw::draw(root, c, &judged);
        if !drawn.is_empty() {
            if !self.drawn.is_empty() {
                self.drawn.push('\n');
            }
            self.drawn.push_str(&drawn);
        }
        let mut row = status::row(c, &judged);
        let posts = post::all(root, c, run);
        if let Some(line) = post::line(&posts, &c.digest()) {
            if line.status.as_deref() == Some("attested") {
                row.flags.push("posted".into());
            }
            row.more.push(line);
        }
        row.extra.push(("posts", serde_json::json!(posts.len())));
        let payload = super::render::payload(root, c).map(|p| candidate::sha(&p));
        row.extra
            .push(("payload_digest", serde_json::json!(payload)));
        let current = post::current(&posts, &c.digest()).is_some();
        let next = match (super::next::next(root, c, &judged), status::ready(&judged)) {
            (Some(n), _) => Some(n),
            (None, true) if !current => Some(super::next::post(root, c)),
            _ => None,
        };
        if let Some(next) = next {
            row.more.push(More {
                label: "next",
                text: next.clone(),
                ..More::default()
            });
            row.extra.push(("next", serde_json::json!(next)));
        }
        if payload.as_deref().is_some_and(|p| *p != c.digest()) {
            let short = payload
                .as_deref()
                .unwrap_or("")
                .trim_start_matches("sha256:");
            row.text
                .push_str(&format!(" payload={}", short.get(..12).unwrap_or(short)));
        }
        rows.push(row);
        rows.extend(invalid);
        Ok(())
    }

    fn counts(&self) -> Vec<(&'static str, usize)> {
        let mut out = vec![("candidates", self.candidates), ("ready", self.ready)];
        out.extend(NAMES.iter().copied().zip(self.by));
        out.push(("invalid", self.invalid));
        out.push(("withdrawn", self.withdrawn));
        out
    }
}

/// The rows `resume` shows: the run's newest three live candidates.
pub fn resume_rows(session: &Session<'_>, rec: &Rec) -> (Vec<Row>, Vec<String>) {
    let root = &session.ws.root;
    let cands = candidate::list(root, &runs::dir(rec), &rec.id);
    let withdrawn = withdraw::ids(root, &cands, &rec.id);
    let live: Vec<&Candidate> = cands
        .iter()
        .filter(|c| !withdrawn.contains(&c.id))
        .collect();
    let mut rows = Vec::new();
    let mut notes = Vec::new();
    let shown = live.len().saturating_sub(3);
    for c in &live[shown..] {
        let mut tally = Tally::default();
        if let Err(error) = tally.add(&mut rows, session, c, (&rec.id, false)) {
            notes.push(format!("deliveries: {error}"));
        }
    }
    crate::plan::delivery::mark(root, rec, &mut rows);
    if shown > 0 {
        notes.push(format!(
            "{} live delivery candidates; the newest 3 are shown (writfold kb deliver status)",
            live.len()
        ));
    }
    (rows, notes)
}
