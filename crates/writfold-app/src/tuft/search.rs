//! Lowers Tuft's `find` onto the B0 storage boundary at one pinned commit.
//!
//! `find text` is a B0 content-substring lookup, one matching file per page,
//! so the measured coverage of each page says exactly how many files were
//! fully searched when the hit limit stops the search. `find rx` has no B0
//! match mode: it enumerates every record in scope with an exact kind lookup
//! and reads each one exactly. Positions inside a file are located by the
//! pure `tuft::locate` over the bytes `read_exact` returned, bound to the
//! digest the lookup reported.
use super::CANCELLED;
use crate::trial::Result;
use std::collections::BTreeMap;
use tuft::ir::{FindPlan, MatcherPlan};
use tuft::locate::{line_at, text_matches};
use tuft::view::{SearchReport, Stop};
use writfold_domain::GitObjectId;
use writfold_domain::citation::Coverage;
use writfold_store::backend::git::GitTreeBackend;
use writfold_store::backend::{
    ExactRead, LookupField, LookupRequest, MatchMode, Order, RecordKey, StorageBackend,
};

/// One match: a file range and the 1-based line of its first byte.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hit {
    pub path: String,
    pub range: (usize, usize),
    pub line: u64,
}

pub struct Found {
    pub hits: Vec<Hit>,
    pub report: SearchReport,
    /// Exact bytes of every file with a hit, as `read_exact` returned them.
    pub files: BTreeMap<String, Vec<u8>>,
}

struct Collector<'a> {
    plan: &'a FindPlan,
    hits: Vec<Hit>,
    files: BTreeMap<String, Vec<u8>>,
}

impl Collector<'_> {
    /// Add one file's matches. Returns `false` when the limit cut this file.
    fn add(&mut self, key: &RecordKey, content: &[u8], matches: &[(usize, usize)]) -> bool {
        let room = self.plan.budget.positions() - self.hits.len();
        let take = matches.len().min(room);
        for &(lo, hi) in matches.iter().take(take) {
            self.hits.push(Hit {
                path: key.as_str().to_owned(),
                range: (lo, hi),
                line: line_at(content, lo),
            });
        }
        if take > 0 {
            self.files.insert(key.as_str().to_owned(), content.to_vec());
        }
        take == matches.len()
    }

    fn finish(self, coverage: Coverage, stop: Option<Stop>) -> Found {
        Found {
            report: SearchReport {
                coverage,
                stop,
                hits: self.hits.len(),
            },
            hits: self.hits,
            files: self.files,
        }
    }
}

pub fn find(backend: &GitTreeBackend<'_>, pin: &GitObjectId, plan: &FindPlan) -> Result<Found> {
    let collector = Collector {
        plan,
        hits: Vec::new(),
        files: BTreeMap::new(),
    };
    match &plan.matcher {
        MatcherPlan::Text(needle) => text(backend, pin, needle, collector),
        MatcherPlan::Rx(_) => rx(backend, pin, collector),
    }
}

fn text(
    backend: &GitTreeBackend<'_>,
    pin: &GitObjectId,
    needle: &str,
    mut collector: Collector<'_>,
) -> Result<Found> {
    let request = LookupRequest::new(
        LookupField::Content,
        MatchMode::Substring,
        Order::Key,
        needle.to_owned(),
        1,
    )?;
    let mut cursor = None;
    loop {
        let page = backend.lookup(pin, &request, cursor.as_ref(), &CANCELLED)?;
        for hit in page.hits() {
            let read = ExactRead::new(hit.key().clone(), Some(hit.digest().clone()));
            let record = backend.read_exact(pin, &read, &CANCELLED)?;
            let matches = text_matches(record.content(), needle.as_bytes());
            if matches.is_empty() {
                return Err("backend reported a file without the needle".into());
            }
            if !collector.add(hit.key(), record.content(), &matches) {
                let coverage = match page.coverage() {
                    Coverage::Measured { examined, total } => Coverage::Measured {
                        examined: examined.saturating_sub(1),
                        total,
                    },
                    Coverage::Unavailable => Coverage::Unavailable,
                };
                return Ok(collector.finish(coverage, Some(Stop::HitLimit)));
            }
        }
        match page.next() {
            Some(next) => cursor = Some(next.clone()),
            None => return Ok(collector.finish(page.coverage(), None)),
        }
    }
}

fn rx(
    backend: &GitTreeBackend<'_>,
    pin: &GitObjectId,
    mut collector: Collector<'_>,
) -> Result<Found> {
    let MatcherPlan::Rx(pattern) = &collector.plan.matcher else {
        return Err("rx lowering needs a regex".into());
    };
    let pattern = pattern.clone();
    backend.preload(pin, &CANCELLED)?;
    let request = LookupRequest::new(
        LookupField::Kind,
        MatchMode::Exact,
        Order::Key,
        "blob".to_owned(),
        1024,
    )?;
    let mut cursor = None;
    let mut examined = 0;
    loop {
        let page = backend.lookup(pin, &request, cursor.as_ref(), &CANCELLED)?;
        let total = match page.coverage() {
            Coverage::Measured { total, .. } => total,
            Coverage::Unavailable => return Err("enumeration coverage unavailable".into()),
        };
        for hit in page.hits() {
            let read = ExactRead::new(hit.key().clone(), Some(hit.digest().clone()));
            let record = backend.read_exact(pin, &read, &CANCELLED)?;
            let matches = pattern.matches(record.content()).map_err(|zero| {
                format!(
                    "rx matched zero bytes in {} at byte {}; refused (no empty ranges)",
                    hit.key().as_str(),
                    zero.offset
                )
            })?;
            if !collector.add(hit.key(), record.content(), &matches) {
                let coverage = Coverage::Measured { examined, total };
                return Ok(collector.finish(coverage, Some(Stop::HitLimit)));
            }
            examined += 1;
        }
        match page.next() {
            Some(next) => cursor = Some(next.clone()),
            None => return Ok(collector.finish(page.coverage(), None)),
        }
    }
}
