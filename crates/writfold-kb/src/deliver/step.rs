//! A delivery a plan step owes (`deliver:<mode>`), seen from the run's
//! candidates: which candidates are of the mode (ordered by the
//! manifest's `seq`, then by ID; never by a clock), which a done record
//! binds, which one decides an open step ([`writfold_core::delivery`]),
//! and the judged view of one candidate: every obligation as `deliver
//! status` judges it, one standing for the whole delivery, and the
//! definition digest a done record keeps. Nothing is sent: a delivery
//! that stands met means a candidate was ready, never that it was posted.
use super::candidate::{self, Candidate};
use super::status::{self, Filings, Judged};
use super::{config, recheck};
use crate::emit::Row;
use crate::session::Session;
use writfold_core::gate::{Standing, State};

/// The word a plan step owes a delivery by: `deliver:<mode>`.
pub const PREFIX: &str = "deliver:";

/// The mode a step word owes, when it is a delivery word.
pub fn mode_of(word: &str) -> Option<&str> {
    word.strip_prefix(PREFIX)
}

fn seq(c: &Candidate) -> u64 {
    c.manifest.as_ref().map_or(0, |m| m.seq)
}

/// The candidates whose manifest reads as valid and names `mode`, by
/// `(seq, id)`. A candidate with an invalid manifest belongs to no mode.
pub fn of_mode<'a>(cands: &'a [Candidate], mode: &str) -> Vec<&'a Candidate> {
    let mut out: Vec<&Candidate> = cands
        .iter()
        .filter(|c| c.manifest.as_ref().is_ok_and(|m| m.mode == mode))
        .collect();
    out.sort_by(|a, b| (seq(a), &a.id).cmp(&(seq(b), &b.id)));
    out
}

/// The `seq` a new candidate of `mode` takes: one more than the highest
/// among the run's candidates of that mode.
pub fn next_seq(root: &std::path::Path, run_dir: &str, run: &str, mode: &str) -> u64 {
    let cands = candidate::list(root, run_dir, run);
    of_mode(&cands, mode)
        .iter()
        .map(|c| seq(c))
        .max()
        .unwrap_or(0)
        + 1
}

/// The candidate ID a done record's evidence names
/// (`<run_dir>/deliveries/<id>/candidate.md`).
pub fn bound_id<'a>(run_dir: &str, evidence: &'a str) -> Option<&'a str> {
    let rest = evidence.strip_prefix(run_dir)?.strip_prefix('/')?;
    let rest = rest.strip_prefix(candidate::DIR)?.strip_prefix('/')?;
    let id = rest.strip_suffix(candidate::PAYLOAD)?.strip_suffix('/')?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

/// The candidate that decides an open step: the last of `mine` that no
/// done record binds.
pub fn deciding<'a>(mine: &[&'a Candidate], bound: &[String]) -> Option<&'a Candidate> {
    let flags: Vec<bool> = mine.iter().map(|c| bound.contains(&c.id)).collect();
    writfold_core::delivery::deciding(&flags).and_then(|i| mine.get(i).copied())
}

/// One candidate as a step judges it.
pub struct Seen {
    pub id: String,
    /// The payload's KB path: a done record's evidence.
    pub payload: String,
    /// The payload's present digest.
    pub content: String,
    /// `deliver status`'s row for it, with one `check:` line per obligation.
    pub row: Row,
    /// Each unmet obligation, `<name> [<standing>]`, in the mode's order.
    pub owed: Vec<String>,
    /// The command that meets the first unmet obligation.
    pub next: Option<String>,
    /// Instructions of an owed attested mode gate.
    pub read: Option<String>,
}

/// A delivery as it stands now.
pub struct View {
    pub mode: String,
    pub standing: Standing,
    /// Obligations met, of how many.
    pub met: usize,
    pub total: usize,
    /// SHA-256 over the mode, the audience and each obligation's digest.
    pub definition: String,
    pub candidate: Option<Seen>,
    /// Why nothing can meet it: the mode does not load or declares nothing.
    pub why: Option<String>,
}

/// The definition digest of a delivery judged with `judged`.
fn definition(mode: &str, audience: &str, judged: &[Judged<'_>]) -> String {
    let mut text = format!("writfold-delivery-gate/1\n{mode}\n{audience}\n");
    for j in judged {
        text.push_str(&format!("{}\t{}\n", j.obligation.name, j.obligation.digest));
    }
    candidate::sha(text.as_bytes())
}

fn empty(mode: &str) -> String {
    format!(
        "mode {mode} declares nothing to check; a delivery is not met by an empty set: add checks, gates or attest to {}/{mode}.toml",
        config::DIR
    )
}

fn open_view(mode: &str, why: Option<String>) -> View {
    View {
        mode: mode.to_owned(),
        standing: Standing::Checked(State::Open),
        met: 0,
        total: 0,
        definition: String::new(),
        candidate: None,
        why,
    }
}

/// Judge the delivery of `mode` on candidate `c` (none: open).
pub fn view(session: &Session<'_>, mode: &str, c: Option<&Candidate>, run: &str) -> View {
    let root = &session.ws.root;
    let declared = match config::mode(root, mode) {
        Err(e) => return open_view(mode, Some(e.to_string())),
        Ok(m) => !(m.checks.is_empty() && m.gates.is_empty() && m.attest.is_empty()),
    };
    if !declared {
        return open_view(mode, Some(empty(mode)));
    }
    let Some((c, audience)) = c.and_then(|c| Some((c, c.manifest.as_ref().ok()?.audience.clone())))
    else {
        return open_view(mode, None);
    };
    let obligations = match status::obligations(root, c) {
        Ok(o) => o,
        Err(e) => return open_view(mode, Some(e.to_string())),
    };
    let filings = Filings::read(root, c, run);
    let mut judged = status::judge(obligations, c, &filings);
    recheck::apply(session, c, &mut judged);
    let standings: Vec<Standing> = judged.iter().map(|j| j.standing).collect();
    let owed: Vec<&Judged<'_>> = judged.iter().filter(|j| !j.standing.met()).collect();
    let next = owed.first().map(|j| match j.obligation.class {
        "attested" => format!(
            "writfold kb deliver attest {} --check {} --statement TEXT --by WHO",
            c.id, j.obligation.name
        ),
        _ => format!("writfold kb deliver check {}", c.id),
    });
    let read = owed
        .iter()
        .filter(|j| j.obligation.class == "attested")
        .filter_map(|j| j.obligation.gate.as_ref())
        .find_map(|g| crate::gate::status::read_line(g, "").and_then(|m| m.path));
    View {
        mode: mode.to_owned(),
        standing: writfold_core::delivery::standing(&standings),
        met: judged.len() - owed.len(),
        total: judged.len(),
        definition: definition(mode, &audience, &judged),
        why: None,
        candidate: Some(Seen {
            id: c.id.clone(),
            payload: c.payload_path(),
            content: c.digest(),
            owed: owed
                .iter()
                .map(|j| format!("{} [{}]", j.obligation.name, j.standing.name()))
                .collect(),
            row: status::row(c, &judged),
            next,
            read,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::bound_id;

    #[test]
    fn bound_ids_come_from_the_payload_path_alone() {
        let run = "runs/run-a";
        let ok = "runs/run-a/deliveries/20261112T211045Z-ee94cc/candidate.md";
        assert_eq!(bound_id(run, ok), Some("20261112T211045Z-ee94cc"));
        for bad in [
            "runs/run-b/deliveries/x/candidate.md",
            "runs/run-a/deliveries/x/delivery.json",
            "runs/run-a/deliveries//candidate.md",
            "runs/run-a/deliveries/x/y/candidate.md",
            "runs/run-a/checks/x/candidate.md",
        ] {
            assert_eq!(bound_id(run, bad), None, "{bad}");
        }
    }
}
