//! Run artifacts: `checkpoint --artifact <store>@<rev>:<path>` (repeatable)
//! records what a run produced in an attached store ([`crate::store`]) as
//! one create-new file `runs/<run>/artifacts/<time>-<hex>.json` per
//! checkpoint, holding an ordinary `code` citation token for each file and
//! the checkpoint it came with. Whether a pointer is recorded is the proved
//! [`writfold_core::store::artifact`]; afterwards it is a citation like any
//! other, judged by `stale` (at its pin, in a pinned store) wherever it is
//! shown. The run's manifest and sidecar are not touched.
//!
//! Reading the store's clone (is the commit there, does a ref reach it,
//! what the path holds), minting the token and rendering rows are tested
//! here and in `tests/stores.rs`.
use super::cite::{Fields, code_token, seen};
use super::json::{Json, object};
use super::span::{Blob, Spanned, blobs};
use crate::emit::{More, Row};
use crate::error::{KbError, Result};
use crate::index::query::Rec;
use crate::runs;
use crate::session::Session;
use crate::stale::{self, Cite};
use crate::store::{Store, Stores};
use crate::workspace::{CANCELLED, budgets};
use std::path::Path;
use writfold_core::store::{self as kernel, Artifact, Commit, Refusal};
use writfold_runtime::git::{GitExecutable, KbRepository, kb_anchored, kb_batch_check};
use writfold_sources::citations::normalize_span;

pub const DIR: &str = "artifacts";
pub const SCHEMA: &str = "writfold-artifacts/1";

/// `<store>@<rev>:<path>`.
fn parse(spec: &str) -> Result<(&str, &str, &str)> {
    let parsed = spec
        .split_once('@')
        .and_then(|(store, rest)| rest.split_once(':').map(|(rev, path)| (store, rev, path)))
        .filter(|(s, r, p)| !s.is_empty() && !r.is_empty() && !p.is_empty());
    let (store, rev, path) = parsed.ok_or_else(|| {
        KbError::Usage(format!("--artifact {spec:?} is not <store>@<rev>:<path>"))
    })?;
    let path = path.trim_start_matches("./");
    if !crate::stale::code_path(path) {
        return Err(KbError::Usage(format!(
            "{path:?} is not a path inside the store"
        )));
    }
    Ok((store, rev, path))
}

/// The cited commit in the clone and, when a ref reaches it, the file at
/// `rev:path`.
fn facts(
    git: &GitExecutable<'_>,
    dir: &Path,
    rev: &str,
    path: &str,
) -> Result<(Commit, Option<Blob>)> {
    let reads = KbRepository {
        git_dir: dir,
        work_tree: None,
    };
    let answer = kb_batch_check(git, reads, &[rev.to_owned()], budgets()?, &CANCELLED)?
        .into_iter()
        .next()
        .flatten()
        .filter(|(_, kind)| kind == "commit");
    let Some((id, _)) = answer else {
        return Ok((Commit::Absent, None));
    };
    if !kb_anchored(git, reads, &id, budgets()?, &CANCELLED)? {
        return Ok((Commit::Unanchored, None));
    }
    Ok((
        Commit::Anchored,
        blobs(git, dir, rev, &[path])?.pop().flatten(),
    ))
}

/// The whole file as one span (an empty file is line 1, with no text).
pub(super) fn whole(path: &str, blob: &Blob) -> Spanned {
    let text = String::from_utf8_lossy(&blob.1);
    let lines: Vec<&str> = text.lines().collect();
    Spanned {
        path: path.to_owned(),
        first: 1,
        last: u32::try_from(lines.len().max(1)).unwrap_or(u32::MAX),
        blob: blob.0.as_str().to_owned(),
        span: normalize_span(&lines),
    }
}

fn refusal(why: &Refusal, spec: &str, name: &str, store: Option<&Store>, rev: &str) -> KbError {
    let url = store.map_or("", |s| s.url.as_str());
    KbError::Refused(match why {
        Refusal::Undeclared => format!(
            "--artifact {spec}: no store {name} is declared ({}/{name}.toml)",
            crate::store::DIR
        ),
        Refusal::NotLocal => {
            format!("--artifact {spec}: store {name} has no clone of {url} here (see kb repos)")
        }
        Refusal::ShortRev => format!(
            "--artifact {spec}: {rev} is not a full commit ID (git -C <store> rev-parse {rev})"
        ),
        Refusal::NoCommit => {
            format!("--artifact {spec}: the clone has no commit {rev}; commit or fetch it first")
        }
        Refusal::Unanchored => format!(
            "--artifact {spec}: no branch or tag reaches {rev}; push or tag it first (an unanchored pin reads gone)"
        ),
        Refusal::NoBlob => format!("--artifact {spec}: not a file at {rev}"),
    })
}

/// The token for one `--artifact`, or the proved refusal.
fn token(session: &Session<'_>, stores: &Stores, spec: &str) -> Result<String> {
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("--artifact needs Git".into()))?;
    let (name, rev, path) = parse(spec)?;
    let store = stores.get(name);
    let dir = store.and_then(|s| crate::store::resolve(Some(session.ws), s).git_dir);
    let rev = rev.to_ascii_lowercase();
    let full = crate::stale::full_rev(&rev);
    let (commit, blob) = match (&dir, full) {
        (Some(dir), true) => facts(git, dir, &rev, path)?,
        _ => (Commit::Absent, None),
    };
    let admitted = kernel::artifact(
        store.is_some(),
        dir.is_some(),
        full,
        &commit,
        blob.is_some(),
    );
    let (Artifact::Admit, Some(store), Some(blob)) = (&admitted, store, &blob) else {
        let why = match admitted {
            Artifact::Refuse(why) => why,
            Artifact::Admit => Refusal::NoBlob,
        };
        return Err(refusal(&why, spec, name, store, &rev));
    };
    code_token(&Fields {
        repo: name,
        rev: &rev,
        spanned: &whole(path, blob),
        sym: None,
        seen: &seen(session.now),
        ttl: None,
        url: &store.url,
        dirty: None,
    })
}

/// The artifacts file of one checkpoint: its name under the run's
/// directory and its bytes, or `None` without `--artifact`.
pub fn file(
    session: &Session<'_>,
    run: &str,
    stamp: &str,
    specs: &[String],
) -> Result<Option<(String, Vec<u8>)>> {
    if specs.is_empty() {
        return Ok(None);
    }
    let stores = crate::store::read(&session.ws.root)?;
    let mut tokens = Vec::with_capacity(specs.len());
    for spec in specs {
        tokens.push(token(session, &stores, spec)?);
    }
    let name = format!(
        "{DIR}/{}.json",
        super::unique(session.now, &specs.join("\n"))
    );
    let bytes = object(&[
        ("schema", Json::Text(SCHEMA.into())),
        ("run", Json::Text(run.to_owned())),
        ("checkpoint", Json::Text(stamp.to_owned())),
        ("tokens", Json::List(tokens)),
    ]);
    Ok(Some((name, bytes.into_bytes())))
}

/// Every artifact of a run, oldest first, each with its verdict now.
pub fn rows(session: &Session<'_>, rec: &Rec) -> Result<Vec<Row>> {
    let dir = format!("{}/{DIR}", runs::dir(rec));
    let Ok(entries) = std::fs::read_dir(session.ws.root.join(&dir)) else {
        return Ok(Vec::new());
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();
    let mut cites = Vec::new();
    let mut at = Vec::new();
    for name in names {
        let path = format!("{dir}/{name}");
        let text = session.text(&path).unwrap_or_default();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        let checkpoint = value["checkpoint"].as_str().unwrap_or_default().to_owned();
        let listed = value["tokens"].as_array().cloned().unwrap_or_default();
        for token in listed.iter().filter_map(serde_json::Value::as_str) {
            for c in crate::parse::cite::tokens(token) {
                cites.push(Cite::from_citation(&path, cites.len(), c));
                at.push(checkpoint.clone());
            }
        }
    }
    let checked = stale::check(session, &cites, false)?;
    Ok(cites
        .iter()
        .zip(checked)
        .zip(at)
        .map(|((cite, checked), checkpoint)| row(cite, &checked.verdict, checkpoint))
        .collect())
}

fn row(cite: &Cite, verdict: &stale::Verdict, checkpoint: String) -> Row {
    let repo = cite.repo.clone().unwrap_or_default();
    let rev = cite.rev.clone().unwrap_or_default();
    let cpath = cite.cpath.clone().unwrap_or_default();
    let mut flags = Vec::new();
    if let stale::Verdict::Unverifiable(why) = verdict {
        flags.push(why.clone());
    }
    Row {
        kind: "artifact",
        id: format!("{repo}@{}:{cpath}", rev.get(..12).unwrap_or(&rev)),
        status: Some(verdict.name().into()),
        text: cite.raw.clone(),
        path: Some(cite.path.clone()),
        flags,
        more: vec![More {
            label: "checkpoint",
            text: checkpoint,
            ..More::default()
        }],
        ..Row::default()
    }
}
