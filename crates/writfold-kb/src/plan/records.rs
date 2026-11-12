//! Run-record checks: gates of class `run-record`, judged over the run's
//! own records instead of code. What a run has written is read from three
//! places: its checkpoint entries (`checkpoint.md`), its retrieval
//! receipts (`retrievals/`), and the KB's files outside `runs/` (from the
//! index, as one digest), plus the pending proposals that recorded it
//! ([`crate::proposal::for_run`]).
//!
//! A check looks at one window: from the event that began the current
//! plan step to now (the whole run when it has no plan). Its receipt binds
//! the window's run-record digest ([`digest`]), so it goes stale when the
//! run's records change, like a receipt whose scope moved. A done step's
//! window is closed by its `done-` record, so its receipts stay standing.
//!
//! These checks show that something was written, never that it is good:
//! pair one with an attested gate whose instructions say what a good
//! backfill or reflection covers.
use super::event::Baseline;
use crate::error::Result;
use crate::index::query::Rec;
use crate::session::Session;
use serde_json::json;
use sha2::{Digest, Sha256};

/// The built-in requirements a `require` list may name.
pub const REQUIREMENTS: [&str; 2] = ["research.backfilled", "run.reflected"];

/// Refuse an unknown or repeated requirement.
pub fn known(require: &[String]) -> std::result::Result<(), String> {
    for (n, name) in require.iter().enumerate() {
        if !REQUIREMENTS.contains(&name.as_str()) {
            return Err(format!(
                "require {name:?} is not a built-in requirement ({})",
                REQUIREMENTS.join(", ")
            ));
        }
        if require[..n].contains(name) {
            return Err(format!("require names {name} twice"));
        }
    }
    Ok(())
}

/// The run's records now.
#[derive(Clone, Debug, Default)]
pub struct Records {
    /// The text of each checkpoint entry, in file order.
    pub entries: Vec<String>,
    pub retrievals: Vec<String>,
    pub kb: String,
    pub proposals: u64,
}

impl Records {
    pub fn baseline(&self) -> Baseline {
        Baseline {
            entries: u64::try_from(self.entries.len()).unwrap_or(u64::MAX),
            retrievals: self.retrievals.clone(),
            kb: self.kb.clone(),
            proposals: self.proposals,
        }
    }
}

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{}", crate::index::hex(&Sha256::digest(bytes)))
}

/// Read the run's records: its checkpoint entries, retrieval receipts,
/// the KB digest (files outside `runs/`, from the index) and proposals.
pub fn observe(session: &Session<'_>, rec: &Rec) -> Result<Records> {
    let run_dir = crate::runs::dir(rec);
    let (path, sections) = crate::runs::entries(session, rec);
    let entries = sections
        .iter()
        // Trimmed: appending an entry moves the blank line that ends the
        // one before it, which must not change that entry.
        .map(|s| {
            session
                .lines(&path, s.first + 1, s.last)
                .join("\n")
                .trim()
                .to_owned()
        })
        .collect();
    let dir = session
        .ws
        .root
        .join(&run_dir)
        .join(crate::write::retrieval::DIR);
    let mut retrievals: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".md") && !n.starts_with('.'))
        .collect();
    retrievals.sort();
    let mut statement = session.conn.prepare(
        "SELECT path, hex(sha256) FROM files WHERE path NOT LIKE 'runs/%' ORDER BY path",
    )?;
    let mut hash = Sha256::new();
    let rows = statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (path, digest) = row?;
        hash.update(format!("{path}\0{digest}\n").as_bytes());
    }
    // Pending proposals in the local queue that recorded this run. An
    // accepted one is a KB record by then, which `run.reflected` counts as
    // a change of the KB digest within the window.
    let proposals = crate::proposal::for_run(session.ws, &rec.id);
    Ok(Records {
        entries,
        retrievals,
        kb: format!("sha256:{}", crate::index::hex(&hash.finalize())),
        proposals: u64::try_from(proposals).unwrap_or(u64::MAX),
    })
}

fn upto(n: u64) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

/// The run-record digest of a window: where it began (`None`: the start
/// of the run), where it ends, and every checkpoint entry up to its end.
/// Any change to those records changes it; nothing after the end does.
pub fn digest(begin: Option<&Baseline>, end: &Baseline, records: &Records) -> String {
    let edge = |b: &Baseline| json!({"entries": b.entries, "retrievals": b.retrievals, "kb": b.kb, "proposals": b.proposals});
    let shown: Vec<String> = records
        .entries
        .iter()
        .take(upto(end.entries))
        .map(|e| sha(e.as_bytes()))
        .collect();
    let value = json!({"begin": begin.map(edge), "end": edge(end), "entries": shown});
    sha(crate::gate::receipt::canonical(&value).as_bytes())
}

/// What a run-record check found.
#[derive(Clone, Debug, Default)]
pub struct Verdict {
    pub pass: bool,
    /// Records that count as evidence.
    pub evidence: usize,
    /// One line per unmet requirement.
    pub findings: Vec<String>,
}

fn starts(line: &str, word: &str) -> bool {
    line.trim_start()
        .get(..word.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(word))
}

/// Evaluate `require` over a window.
pub fn evaluate(
    require: &[String],
    begin: Option<&Baseline>,
    end: &Baseline,
    records: &Records,
) -> Verdict {
    let upto_end = &records.entries[..upto(end.entries).min(records.entries.len())];
    let from = begin.map_or(0, |b| upto(b.entries)).min(upto_end.len());
    let mut verdict = Verdict {
        pass: true,
        ..Verdict::default()
    };
    for name in require {
        let (met, evidence, missing) = match name.as_str() {
            "research.backfilled" => {
                let retrievals = end
                    .retrievals
                    .iter()
                    .filter(|r| begin.is_none_or(|b| !b.retrievals.contains(r)))
                    .count();
                let cited = upto_end[from..]
                    .iter()
                    .filter(|e| {
                        crate::parse::cite::tokens(e)
                            .iter()
                            .any(|c| !c.illustrative)
                    })
                    .count();
                let kb = usize::from(begin.is_some_and(|b| b.kb != end.kb));
                let n = retrievals + cited + kb;
                (n > 0, n, "research.backfilled: since the step began, no KB record changed, no retrieval receipt was recorded and no checkpoint holds a citation".to_owned())
            }
            _ => {
                let reflected = upto_end.iter().filter(|e| starts(e, "reflection:")).count();
                let answered = end.proposals > 0
                    || begin.is_some_and(|b| b.kb != end.kb)
                    || upto_end
                        .iter()
                        .any(|e| e.lines().any(|l| starts(l, "no proposal")));
                let missing = match (reflected > 0, answered) {
                    (false, _) => "run.reflected: no checkpoint entry starts with \"reflection:\"",
                    (true, _) => {
                        "run.reflected: no proposal (pending, or accepted as a KB record in this step), and no line saying \"no proposal\""
                    }
                };
                (reflected > 0 && answered, reflected, missing.to_owned())
            }
        };
        verdict.evidence += evidence;
        if !met {
            verdict.pass = false;
            verdict.findings.push(missing);
        }
    }
    verdict
}

#[cfg(test)]
mod tests {
    use super::{Records, digest, evaluate, known};

    fn records(entries: &[&str]) -> Records {
        Records {
            entries: entries.iter().map(|e| (*e).to_owned()).collect(),
            retrievals: Vec::new(),
            kb: "sha256:00".into(),
            proposals: 0,
        }
    }

    #[test]
    fn requirements_pass_and_fail() {
        let need = |n: &str| vec![n.to_owned()];
        assert!(known(&need("research.backfilled")).is_ok());
        assert!(known(&need("vibes")).is_err());
        assert!(known(&[need("run.reflected"), need("run.reflected")].concat()).is_err());
        let mut r = records(&["started"]);
        let begin = r.baseline();
        r.entries.push("looked around".into());
        let end = r.baseline();
        assert!(!evaluate(&need("research.backfilled"), Some(&begin), &end, &r).pass);
        r.retrievals.push("20261112T000000Z-aaaaaa.md".into());
        assert!(
            evaluate(
                &need("research.backfilled"),
                Some(&begin),
                &r.baseline(),
                &r
            )
            .pass
        );
        let mut kb = end.clone();
        kb.kb = "sha256:11".into();
        assert!(evaluate(&need("research.backfilled"), Some(&begin), &kb, &r).pass);
        // A retrieval from before the step does not count for it.
        let before = r.baseline();
        assert!(!evaluate(&need("research.backfilled"), Some(&before), &before, &r).pass);
        let reflect = need("run.reflected");
        assert!(!evaluate(&reflect, None, &r.baseline(), &r).pass);
        r.entries.push("Reflection: the plan was too coarse".into());
        let half = evaluate(&reflect, None, &r.baseline(), &r);
        assert!(!half.pass && half.findings[0].contains("no proposal"));
        // A pending proposal of the run answers it; so does a KB record
        // written in the step (an accepted proposal).
        let mut proposed = r.clone();
        proposed.proposals = 1;
        assert!(evaluate(&reflect, None, &proposed.baseline(), &proposed).pass);
        let mut accepted = r.baseline();
        accepted.kb = "sha256:22".into();
        assert!(evaluate(&reflect, Some(&r.baseline()), &accepted, &r).pass);
        r.entries
            .push("reflection: fine\nNo proposal: nothing to change".into());
        assert!(evaluate(&reflect, None, &r.baseline(), &r).pass);
    }

    #[test]
    fn the_digest_moves_with_the_records_and_not_after_the_window() {
        let mut r = records(&["a"]);
        let end = r.baseline();
        let before = digest(None, &end, &r);
        r.entries.push("b".into());
        assert_eq!(digest(None, &end, &r), before, "an entry after the end");
        r.entries[0] = "a, edited".into();
        assert_ne!(digest(None, &end, &r), before);
        assert_ne!(digest(Some(&end), &end, &records(&["a"])), before);
    }
}
