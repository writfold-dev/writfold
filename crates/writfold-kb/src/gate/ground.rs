//! Grounding: what each cited reference stands on now, for an assessed
//! gate's assumptions and a checklist gate's cover rows alike.
//!
//! | Reference | Grounded when | Standing recorded |
//! |---|---|---|
//! | a KB citation token `⟦cite …⟧` or locator `repo@commit:path[:a-b]` | `stale` says fresh | the verdict |
//! | `gate:<id>` | the gate is satisfied (attested: attested) | the gate's standing |
//! | a gate receipt (`runs/<run>/gates/….json`, or its name) | it passed, about the gate's current definition and bytes, and the gate stands | `satisfied`, `stale`, … |
//! | an attestation (`runs/<run>/statements/….json`) | it is the gate's current attestation | `attested` or `stale` |
//! | a retrieval receipt (`runs/<run>/retrievals/….md`) | every citation in it is fresh | `fresh` or `not-fresh:<n>` |
//! | an edit receipt (`runs/<run>/edits/….md`) | every file still has the bytes the edit wrote | `holds` or `changed:<path>` |
//! | an observation (`runs/<run>/observed/….json`) | its status is `ok` and its citation is fresh | `ok`, `not-fresh:<n>`, or the status (`timeout` …) |
//!
//! Gate receipts and statements are read from the current run only; a
//! waiver is never evidence. Both later-judged classes (assessed and
//! checklist gates) resolve against the other gates only, so neither can
//! cite the other: such a reference is not grounded. Anything else is
//! `unresolved`. The standing is recorded in the assessed receipt or the
//! cover row; when it differs on a later read, the cited evidence has
//! moved and the record is stale.
use super::status::{Filings, Report};
use super::subject::Subject;
use crate::error::Result;
use crate::model::Citation;
use crate::parse::cite::{locator, tokens};
use crate::session::Session;
use crate::stale::{self, Cite, Verdict};
use crate::workspace::{CANCELLED, budgets};
use writfold_core::assess::Grounding;
use writfold_core::gate::{Outcome, Standing, State};
use writfold_core::observe::{self, Status};
use writfold_runtime::git::edit_hash_bytes;

/// What one citation stands on now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolved {
    pub reference: String,
    pub grounding: Grounding,
    /// A short word for what was found; a change means the evidence moved.
    pub standing: String,
    /// The KB file the evidence is in, when there is one.
    pub path: Option<String>,
}

/// Everything grounding reads.
pub struct Context<'s, 'r> {
    pub session: &'s Session<'s>,
    pub subject: &'s Subject,
    pub plain: &'r Report<'r>,
    pub filings: &'r Filings,
    /// `runs/<run>`.
    pub run_dir: &'s str,
}

fn found(reference: &str, grounding: Grounding, standing: &str, path: Option<&str>) -> Resolved {
    Resolved {
        reference: reference.to_owned(),
        grounding,
        standing: standing.to_owned(),
        path: path.map(str::to_owned),
    }
}

fn ungrounded(reference: &str, standing: &str, path: Option<&str>) -> Resolved {
    found(reference, Grounding::Ungrounded, standing, path)
}

/// Resolve one reference, as written in a Weft `citation "…"`.
pub fn resolve(cx: &Context<'_, '_>, reference: &str) -> Result<Resolved> {
    let r = reference.trim();
    if r.starts_with('⟦') {
        return token(cx, reference, None);
    }
    if let Some(id) = r.strip_prefix("gate:") {
        return Ok(gate(cx, reference, id));
    }
    if let Some(path) = file(cx, r) {
        let parts: Vec<&str> = path.split('/').collect();
        return match parts.as_slice() {
            ["runs", _, "gates", _] => Ok(receipt(cx, reference, &path)),
            ["runs", _, "statements", _] => Ok(statement(cx, reference, &path)),
            ["runs", _, "retrievals", _] => retrieval(cx, reference, &path),
            ["runs", _, "edits", _] => edit(cx, reference, &path),
            ["runs", _, "observed", _] => observed(cx, reference, &path),
            _ => Ok(ungrounded(reference, "unresolved", Some(&path))),
        };
    }
    if locator(r, 1).is_some() {
        return token(cx, reference, None);
    }
    Ok(ungrounded(reference, "unresolved", None))
}

/// A KB-relative path under `runs/`, or a receipt name in this run.
fn file(cx: &Context<'_, '_>, r: &str) -> Option<String> {
    let root = &cx.session.ws.root;
    if r.starts_with("runs/") && crate::gate::places::contained(r) {
        return root.join(r).is_file().then(|| r.to_owned());
    }
    if r.contains('/') || r.is_empty() {
        return None;
    }
    // A bare name with or without its extension (`<stamp>-<hex>` or
    // `<stamp>-<hex>.json`), as `ls` prints it.
    let stem = r
        .strip_suffix(".json")
        .or_else(|| r.strip_suffix(".md"))
        .unwrap_or(r);
    ["gates", "statements", "retrievals", "edits", "observed"]
        .iter()
        .flat_map(|dir| ["json", "md"].map(|ext| format!("{}/{dir}/{stem}.{ext}", cx.run_dir)))
        .find(|p| root.join(p).is_file())
}

fn verdicts(cx: &Context<'_, '_>, found: Vec<Citation>, path: &str) -> Result<Vec<Verdict>> {
    let cites: Vec<Cite> = found
        .into_iter()
        .filter(|c| !c.illustrative)
        .enumerate()
        .map(|(ord, c)| Cite::from_citation(path, ord, c))
        .collect();
    Ok(stale::check(cx.session, &cites, false)?
        .into_iter()
        .map(|c| c.verdict)
        .collect())
}

/// A citation token or locator, checked by `stale`.
fn token(cx: &Context<'_, '_>, reference: &str, path: Option<&str>) -> Result<Resolved> {
    let r = reference.trim();
    let read = match r.starts_with('⟦') {
        true => tokens(r),
        false => locator(r, 1).into_iter().collect(),
    };
    let found_verdicts = verdicts(cx, read, path.unwrap_or("weft"))?;
    Ok(match found_verdicts.as_slice() {
        [Verdict::Fresh] => found(reference, Grounding::Grounded, "fresh", path),
        [verdict] => ungrounded(reference, verdict.name(), path),
        _ => ungrounded(reference, "not-one-citation", path),
    })
}

/// `gate:<id>`: the gate's standing in this run.
fn gate(cx: &Context<'_, '_>, reference: &str, id: &str) -> Resolved {
    let Some(j) = cx.plain.gates.iter().find(|j| j.definition.id == id) else {
        return ungrounded(reference, "no-such-gate", None);
    };
    let path = match j.standing {
        Standing::Attested => j.statements.attested.map(|(f, _)| f.path.as_str()),
        _ => j.receipt.map(|(f, _)| f.path.as_str()),
    };
    match j.standing {
        Standing::Checked(State::Satisfied) => {
            found(reference, Grounding::Grounded, "satisfied", path)
        }
        Standing::Attested => found(reference, Grounding::Attested, "attested", path),
        other => ungrounded(reference, other.name(), path),
    }
}

/// A gate receipt of this run: grounded while it passed about the gate's
/// current definition and bytes and the gate is not failing now.
fn receipt(cx: &Context<'_, '_>, reference: &str, path: &str) -> Resolved {
    let Some(r) = cx
        .filings
        .receipts
        .iter()
        .find(|f| f.path == path)
        .and_then(|f| f.parsed.as_ref().ok())
    else {
        return ungrounded(reference, "not-a-valid-receipt-of-this-run", Some(path));
    };
    let Some(j) = cx.plain.gates.iter().find(|j| j.definition.id == r.gate) else {
        return ungrounded(reference, "no-such-gate", Some(path));
    };
    let present = cx.plain.snapshot.scoped(&j.definition.patterns()).digest;
    let standing = if r.repo != cx.subject.name {
        "other-subject"
    } else if r.definition != j.definition.digest || r.scope != present {
        "stale"
    } else if r.outcome != Outcome::Pass {
        r.outcome.name()
    } else if !j.standing.met() {
        j.standing.name()
    } else {
        "satisfied"
    };
    match standing {
        "satisfied" => found(reference, Grounding::Grounded, standing, Some(path)),
        _ => ungrounded(reference, standing, Some(path)),
    }
}

/// An attestation of this run: attested while it is the gate's current one.
fn statement(cx: &Context<'_, '_>, reference: &str, path: &str) -> Resolved {
    let current = cx.plain.gates.iter().any(|j| {
        j.standing == Standing::Attested
            && j.statements.attested.is_some_and(|(f, _)| f.path == path)
    });
    match current {
        true => found(reference, Grounding::Attested, "attested", Some(path)),
        false => ungrounded(reference, "not-a-current-attestation", Some(path)),
    }
}

/// A retrieval receipt: grounded while every citation in it is fresh.
fn retrieval(cx: &Context<'_, '_>, reference: &str, path: &str) -> Result<Resolved> {
    let text = cx.session.text(path).unwrap_or_default();
    let all = verdicts(cx, tokens(&text), path)?;
    let moved = all.iter().filter(|v| **v != Verdict::Fresh).count();
    Ok(match (all.is_empty(), moved) {
        (true, _) => ungrounded(reference, "no-citations", Some(path)),
        (false, 0) => found(reference, Grounding::Grounded, "fresh", Some(path)),
        (false, n) => ungrounded(reference, &format!("not-fresh:{n}"), Some(path)),
    })
}

/// An observation: grounded only while it is `ok` and its embedded
/// citation is fresh ([`writfold_core::observe::grounds`]). Any other
/// status never grounds; its standing is the status.
fn observed(cx: &Context<'_, '_>, reference: &str, path: &str) -> Result<Resolved> {
    let parsed = cx
        .session
        .text(path)
        .ok_or_else(|| "unreadable".to_owned())
        .and_then(|text| crate::observe::parse(&text));
    let Ok(o) = parsed else {
        return Ok(ungrounded(reference, "not-a-valid-observation", Some(path)));
    };
    let all = match &o.cite {
        Some(cite) => verdicts(cx, tokens(cite), path)?,
        None => Vec::new(),
    };
    let moved = all.iter().filter(|v| **v != Verdict::Fresh).count();
    let fresh = !all.is_empty() && moved == 0;
    Ok(match (observe::grounds(o.status, fresh), o.status, moved) {
        (true, _, _) => found(reference, Grounding::Grounded, "ok", Some(path)),
        (false, Status::Ok, 0) => ungrounded(reference, "no-citations", Some(path)),
        (false, Status::Ok, n) => ungrounded(reference, &format!("not-fresh:{n}"), Some(path)),
        (false, status, _) => ungrounded(reference, status.name(), Some(path)),
    })
}

/// An edit receipt about this subject: grounded while every edited file
/// still has the blob the edit wrote.
fn edit(cx: &Context<'_, '_>, reference: &str, path: &str) -> Result<Resolved> {
    let text = cx.session.text(path).unwrap_or_default();
    let repo = text
        .lines()
        .find_map(|l| l.strip_prefix("repo: "))
        .unwrap_or("");
    if repo.trim() != cx.subject.name {
        return Ok(ungrounded(reference, "other-subject", Some(path)));
    }
    let Some(git) = cx.session.git.as_ref() else {
        return Ok(ungrounded(reference, "no-git", Some(path)));
    };
    for line in text.lines().filter(|l| l.starts_with("- `")) {
        let body = line.trim_start_matches("- `");
        let (file, rest) = body.split_once('`').unwrap_or((body, ""));
        let after = rest
            .split_whitespace()
            .find_map(|w| w.strip_prefix("after="));
        let bytes = std::fs::read(cx.subject.top.join(file)).ok();
        let now = match bytes {
            Some(b) => Some(edit_hash_bytes(
                git,
                &cx.subject.git_dir,
                &cx.subject.top,
                file,
                &b,
                budgets()?,
                &CANCELLED,
            )?),
            None => None,
        };
        if after.is_none() || now.as_deref() != after {
            return Ok(ungrounded(
                reference,
                &format!("changed:{file}"),
                Some(path),
            ));
        }
    }
    Ok(found(reference, Grounding::Grounded, "holds", Some(path)))
}
