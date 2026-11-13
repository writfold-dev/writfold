//! A decision's state change: the transition table and its requirements
//! come from [`writfold_core::decision`]; this module checks them against
//! the record, the cited repository and the current run, then writes the
//! fields and one Decision log line.
//!
//! - `accepted` sets `decided_by`, `acceptor` and `accepted_revision` (the
//!   record's design revision from the material codec).
//! - `applied` appends `<repo>@<full sha>` to `landed`; the commit must be
//!   in a local clone and in the history of its target ref (`origin/HEAD`,
//!   else `HEAD`, as `stale` uses).
//! - `verified` records, in the log line, which gate receipt grounded it
//!   and on which subject commit. That is a fact about the moment of the
//!   write; nothing re-derives it on read.
use super::status::SetStatus;
use super::{Op, atomic::digest};
use crate::error::{KbError, Result};
use crate::gate::ground::{self, Context};
use crate::gate::status::{self, Filings};
use crate::index::query::{self, Rec};
use crate::session::Session;
use crate::stale::repos;
use crate::time;
use crate::workspace::{CANCELLED, budgets};
use std::path::Path;
use writfold_core::assess::Grounding;
use writfold_core::decision::{Needs, State, step};
use writfold_runtime::git::{KbRepository, kb_commit, kb_is_ancestor};
use writfold_sources::documents::MarkdownDocument;
use writfold_sources::material::material_revision;

fn refused(text: String) -> KbError {
    KbError::Refused(text)
}

/// The change to `rec` (read as `text` and `doc`), as one replacement.
pub fn op(
    session: &Session<'_>,
    (rec, text, doc): (&Rec, &str, &MarkdownDocument),
    to: &str,
    s: &SetStatus,
    cwd: &Path,
) -> Result<Op> {
    let from = rec.status.as_deref().unwrap_or("draft");
    if doc.field("workflow_version").map(str::trim) != Some("3") {
        return Err(refused(format!(
            "{} is not a workflow 3 record (workflow_version: 3); v1 writes only those",
            rec.id
        )));
    }
    let needs = step(State::parse(from), State::parse(to)).ok_or_else(|| {
        refused(format!(
            "{} cannot go from {from} to {to} with v1 writes",
            rec.id
        ))
    })?;
    let by = s.by.as_deref().map(str::trim).unwrap_or_default();
    given(needs, to, by, s)?;
    if needs.ready {
        ready(doc)?;
    }
    let date = time::date(session.now);
    let mut changes: Vec<(&str, Option<String>)> = vec![
        ("status", Some(to.into())),
        ("updated", Some(date.clone())),
        ("last_actor", Some(by.into())),
    ];
    let mut fact = None;
    match State::parse(to) {
        State::Rejected if doc.field("decided_by").is_none_or(|v| v.trim().is_empty()) => {
            changes.push(("decided_by", Some(by.into())));
        }
        State::Accepted => {
            let revision = material_revision(doc)
                .map_err(|e| refused(format!("{} has no design revision: {e}", rec.id)))?;
            changes.push(("decided_by", Some(by.into())));
            changes.push(("acceptor", Some(by.into())));
            changes.push(("accepted_revision", Some(revision.as_str().into())));
        }
        State::Applied => {
            let landed = landed(session, s.commit.as_deref().unwrap_or_default())?;
            let mut list = crate::parse::list(doc.field("landed").unwrap_or_default());
            if !list.contains(&landed) {
                list.push(landed.clone());
            }
            changes.push(("landed", Some(super::list(&list))));
            if !by.is_empty() {
                changes.push(("executor", Some(by.into())));
            }
            fact = Some(format!("landed {landed}"));
        }
        State::Verified => {
            let evidence = s.evidence.as_deref().unwrap_or_default();
            fact = Some(grounded(session, s, evidence.trim(), cwd)?);
            if !by.is_empty() {
                changes.push(("verifier", Some(by.into())));
            }
        }
        State::Superseded => {
            let successor = successor_of(session, rec, s.superseded_by.as_deref())?;
            changes.push(("superseded_by", Some(successor)));
        }
        _ => {}
    }
    let line = log_line(&date, to, by, fact.as_deref(), s.note.as_deref());
    let borrowed: Vec<(&str, Option<&str>)> =
        changes.iter().map(|(k, v)| (*k, v.as_deref())).collect();
    let edited = doc
        .update_fields(&borrowed)
        .and_then(|d| d.append_section("Decision log", &line))
        .map_err(|e| KbError::Usage(format!("cannot edit {}: {e}", rec.path)))?;
    Ok(Op::Replace {
        path: rec.path.clone(),
        expected: digest(text.as_bytes()),
        bytes: edited.as_bytes().to_vec(),
    })
}

/// `- <date>: <state>[ by WHO][; <fact>][ — note]`.
fn log_line(date: &str, to: &str, by: &str, fact: Option<&str>, note: Option<&str>) -> String {
    let mut line = format!("- {date}: {to}");
    if !by.is_empty() {
        line.push_str(&format!(" by {by}"));
    }
    if let Some(fact) = fact {
        line.push_str(&format!("; {fact}"));
    }
    if let Some(note) = note.map(str::trim).filter(|n| !n.is_empty()) {
        line.push_str(&format!(" \u{2014} {}", note.replace(['\n', '\r'], " ")));
    }
    line
}

/// Each flag the change needs is given, and no flag it does not take.
fn given(needs: Needs, to: &str, by: &str, s: &SetStatus) -> Result<()> {
    let usage = |text: &str| Err(KbError::Usage(text.into()));
    if needs.actor && by.is_empty() {
        let verb = if to == "accepted" {
            "accepting"
        } else {
            "rejecting"
        };
        return Err(KbError::Usage(format!("{verb} a decision needs --by WHO")));
    }
    let blank = |v: &Option<String>| v.as_deref().is_none_or(|v| v.trim().is_empty());
    match (needs.commit, blank(&s.commit)) {
        (true, true) => return usage("--set applied needs --commit REPO@SHA"),
        (false, false) => return usage("--commit goes with --set applied"),
        _ => {}
    }
    match (needs.evidence, blank(&s.evidence)) {
        (true, true) => {
            return usage(
                "--set verified needs --evidence REF (a gate receipt of the run, or gate:<id>)",
            );
        }
        (false, false) => return usage("--evidence goes with --set verified"),
        _ => {}
    }
    if !needs.evidence && (s.run.is_some() || s.repo.is_some()) {
        return usage("--run and --repo go with --set verified");
    }
    if !needs.successor && s.superseded_by.is_some() {
        return usage("--superseded-by goes with --set superseded");
    }
    Ok(())
}

/// `<repo>@<full sha>` for `--commit REPO@SHA`: a local clone holds the
/// commit (a full ID or a unique prefix) and its target ref contains it.
fn landed(session: &Session<'_>, commit: &str) -> Result<String> {
    let (repo, sha) = commit
        .trim()
        .rsplit_once('@')
        .ok_or_else(|| KbError::Usage(format!("--commit {commit:?} is not REPO@SHA")))?;
    let sha = sha.to_ascii_lowercase();
    if repo.is_empty()
        || !(4..=64).contains(&sha.len())
        || !sha.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(KbError::Usage(format!(
            "--commit {commit:?} is not REPO@SHA (a commit ID or a prefix of at least 4 hex digits)"
        )));
    }
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| refused("--set applied needs Git".into()))?;
    let Some(dir) = repos::resolve(session, repo, None) else {
        return Err(refused(format!(
            "repository {repo} has no local clone: name it in <git dir>/writfold/repos or under $WRITFOLD_KB_REPOS"
        )));
    };
    let reads = KbRepository {
        git_dir: &dir,
        work_tree: None,
    };
    let full = kb_commit(git, reads, &sha, budgets()?, &CANCELLED)?
        .filter(|id| id.as_str().starts_with(&sha))
        .ok_or_else(|| {
            refused(format!(
                "{repo} has no commit {sha} (or the prefix names more than one)"
            ))
        })?;
    let target = match kb_commit(
        git,
        reads,
        "refs/remotes/origin/HEAD",
        budgets()?,
        &CANCELLED,
    )? {
        Some(id) => Some((id, "origin/HEAD")),
        None => kb_commit(git, reads, "HEAD", budgets()?, &CANCELLED)?.map(|id| (id, "HEAD")),
    };
    let Some((target, named)) = target else {
        return Err(refused(format!("{repo} has no commit on HEAD")));
    };
    if !kb_is_ancestor(git, reads, &full, &target, budgets()?, &CANCELLED)? {
        return Err(refused(format!(
            "{repo}@{} is not in the history of {named} ({}); land it there first",
            short(full.as_str()),
            short(target.as_str())
        )));
    }
    Ok(format!("{repo}@{}", full.as_str()))
}

fn short(id: &str) -> &str {
    id.get(..12).unwrap_or(id)
}

/// The log fact for `--evidence`: a gate receipt of the run, or
/// `gate:<id>`, that grounds now as a passing check (an attestation or a
/// waiver does not).
fn grounded(session: &Session<'_>, s: &SetStatus, evidence: &str, cwd: &Path) -> Result<String> {
    let rec = crate::gate::target(session, s.run.as_deref())?;
    let subject = crate::gate::locate(session, &rec, s.repo.as_deref(), cwd)?.ok_or_else(|| {
        refused(
            "--set verified needs a subject repository: run inside one, or pass --repo DIR".into(),
        )
    })?;
    let mut filings = Filings::for_run(session, &rec);
    filings.owed.gates = None;
    let (plain, _) = status::judge_plain(session, &subject, &filings)?;
    let cx = Context {
        session,
        subject: &subject,
        plain: &plain,
        filings: &filings,
        run_dir: &filings.run_dir,
    };
    let r = ground::resolve(&cx, evidence)?;
    let receipt = r
        .path
        .as_deref()
        .is_some_and(|p| p.starts_with(&format!("{}/gates/", filings.run_dir)));
    if !evidence.starts_with("gate:") && !receipt {
        return Err(KbError::Usage(format!(
            "--evidence {evidence:?} is not a gate receipt of run {} or gate:<id>",
            rec.id
        )));
    }
    if r.grounding != Grounding::Grounded || !receipt {
        return Err(refused(format!(
            "evidence {evidence} does not ground now ({}): verified needs a passing gate receipt of run {} (an attestation or a waiver is not one)",
            r.standing, rec.id
        )));
    }
    Ok(format!(
        "evidence {evidence} {} ({}) on {}@{}",
        r.standing,
        r.path.unwrap_or_default(),
        subject.name,
        short(plain.snapshot.commit.as_str())
    ))
}

/// The successor must be another decision, and the chain from it must not
/// come back to `rec`.
fn successor_of(session: &Session<'_>, rec: &Rec, named: Option<&str>) -> Result<String> {
    let named =
        named.ok_or_else(|| KbError::Usage("--set superseded needs --superseded-by ID".into()))?;
    let bare = named.strip_prefix("proposal/").unwrap_or(named);
    let mut next = Some(bare.to_owned());
    let mut seen = 0;
    while let Some(id) = next {
        let found = query::record(&session.conn, &format!("proposal/{id}"))?
            .ok_or_else(|| refused(format!("no decision {id} in the KB")))?;
        if found.id == rec.id || seen > 64 {
            return Err(refused(format!(
                "superseding {} by {bare} makes a cycle",
                rec.id
            )));
        }
        seen += 1;
        next = found.field("superseded_by");
    }
    Ok(bare.to_owned())
}

/// Review readiness, for `proposed` and `accepted`: research written, two
/// alternatives, and an acceptance-gates table.
fn ready(doc: &MarkdownDocument) -> Result<()> {
    let section = |name: &str| doc.section(name).unwrap_or_default().trim().to_owned();
    let alternatives = section("Alternatives considered")
        .lines()
        .filter(|l| {
            let l = l.trim_start();
            l.starts_with("- ")
                || l.starts_with("* ")
                || l.split_once(['.', ')'])
                    .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .count();
    let gates = section("Acceptance gates");
    let header = gates.lines().next().unwrap_or_default().to_lowercase();
    let table = ["id", "when", "claim", "check", "by"]
        .iter()
        .all(|c| header.split('|').any(|cell| cell.trim() == *c));
    if section("Research").is_empty() || alternatives < 2 || !table {
        return Err(refused(
            "not ready for review: fill Research, at least two Alternatives considered, and an Acceptance gates table (id | when | claim | check | by)".into(),
        ));
    }
    Ok(())
}
