//! Delivery candidates: `runs/<run>/deliveries/<time>-<hex>/` holding
//! `candidate.md`, the exact payload that would be sent, and
//! `delivery.json`, its manifest (mode, audience, destination, and the
//! digests of the source artifact and of the candidate as prepared). The
//! directory is written whole through the write protocol (staged, then
//! renamed into place), so a run holds no candidate or a complete one.
//!
//! The candidate may be edited in place to repair it; the manifest keeps
//! the digest it was prepared with, and checks bind the bytes they read.
//! Nothing here sends anything: there is no network and no adapter.
use super::config::{AUDIENCES, mode};
use crate::error::{KbError, Result};
use crate::write::{Op, one_line, unique};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

pub const SCHEMA: &str = "writfold-delivery/1";
/// The directory of a run that holds its candidates.
pub const DIR: &str = "deliveries";
pub const PAYLOAD: &str = "candidate.md";
pub const MANIFEST: &str = "delivery.json";
/// The largest payload accepted.
const MAX_BYTES: usize = 1 << 20;

pub fn sha(bytes: &[u8]) -> String {
    format!("sha256:{}", crate::index::hex(&Sha256::digest(bytes)))
}

/// What `deliver prepare` was asked for.
#[derive(Clone, Debug)]
pub struct Prepare {
    pub mode: String,
    pub from: std::path::PathBuf,
    pub audience: Option<String>,
    pub destination: Option<String>,
    pub run: Option<String>,
}

/// A manifest, as written and read back.
#[derive(Clone, Debug)]
pub struct Manifest {
    pub id: String,
    pub run: String,
    pub mode: String,
    pub audience: String,
    pub destination: Option<String>,
    pub source_name: String,
    pub source_digest: String,
    /// The candidate's digest when it was prepared.
    pub prepared: String,
    pub recorded: String,
}

fn manifest_bytes(m: &Manifest, size: usize) -> Vec<u8> {
    let value = json!({
        "schema": SCHEMA,
        "id": m.id,
        "run": m.run,
        "mode": m.mode,
        "audience": m.audience,
        "destination": m.destination,
        "source": {"name": m.source_name, "digest": m.source_digest},
        "candidate": {"file": PAYLOAD, "digest": m.prepared, "bytes": size},
        "rendering": "verbatim",
        "recorded": m.recorded,
    });
    let mut out = serde_json::to_vec_pretty(&value).unwrap_or_default();
    out.push(b'\n');
    out
}

fn field(value: &Value, pointer: &str) -> std::result::Result<String, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty() && !t.contains(['\n', '\r']))
        .map(str::to_owned)
        .ok_or_else(|| format!("{pointer} is missing or not a one-line string"))
}

/// Read a manifest filed under `run` as candidate `id`.
pub fn parse(text: &str, run: &str, id: &str) -> std::result::Result<Manifest, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| format!("not JSON ({e})"))?;
    if field(&value, "/schema")? != SCHEMA {
        return Err(format!("schema is not {SCHEMA}"));
    }
    let m = Manifest {
        id: field(&value, "/id")?,
        run: field(&value, "/run")?,
        mode: field(&value, "/mode")?,
        audience: field(&value, "/audience")?,
        destination: field(&value, "/destination").ok(),
        source_name: field(&value, "/source/name")?,
        source_digest: field(&value, "/source/digest")?,
        prepared: field(&value, "/candidate/digest")?,
        recorded: field(&value, "/recorded")?,
    };
    if m.id != id || m.run != run {
        return Err("its id or run does not match where it is filed".into());
    }
    Ok(m)
}

/// One candidate directory of a run, as found.
pub struct Candidate {
    pub id: String,
    /// KB-relative directory.
    pub dir: String,
    pub manifest: std::result::Result<Manifest, String>,
    /// The payload's current bytes, if readable.
    pub bytes: Option<Vec<u8>>,
}

impl Candidate {
    pub fn payload_path(&self) -> String {
        format!("{}/{PAYLOAD}", self.dir)
    }

    /// The digest of the current payload (`missing` when unreadable).
    pub fn digest(&self) -> String {
        self.bytes.as_deref().map_or_else(|| "missing".into(), sha)
    }
}

/// Every candidate of the run whose directory is `run_dir`, oldest first.
pub fn list(root: &Path, run_dir: &str, run: &str) -> Vec<Candidate> {
    let base = format!("{run_dir}/{DIR}");
    let Ok(entries) = std::fs::read_dir(root.join(&base)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|id| {
            let dir = format!("{base}/{id}");
            let manifest = std::fs::read_to_string(root.join(&dir).join(MANIFEST))
                .map_err(|e| format!("{MANIFEST} unreadable ({e})"))
                .and_then(|t| parse(&t, run, &id));
            let bytes = std::fs::read(root.join(&dir).join(PAYLOAD)).ok();
            Candidate {
                id,
                dir,
                manifest,
                bytes,
            }
        })
        .collect()
}

/// Check the request and plan the candidate directory. Returns the
/// operation and the candidate's ID.
pub fn plan(
    root: &Path,
    cwd: &Path,
    p: &Prepare,
    run: (&str, &str),
    now: i64,
) -> Result<(Op, String)> {
    let (run_id, run_dir) = run;
    mode(root, &p.mode)?;
    let audience = p.audience.clone().unwrap_or_else(|| "internal".into());
    if !AUDIENCES.contains(&audience.as_str()) {
        return Err(KbError::Usage(format!(
            "--audience must be one of {}",
            AUDIENCES.join(", ")
        )));
    }
    let destination = match &p.destination {
        Some(d) => Some(one_line("--destination", d)?),
        None => None,
    };
    let from = cwd.join(&p.from);
    let bytes = std::fs::read(&from)
        .map_err(|e| KbError::Usage(format!("--from {}: {e}", p.from.display())))?;
    if bytes.is_empty() || bytes.len() > MAX_BYTES || std::str::from_utf8(&bytes).is_err() {
        return Err(KbError::Usage(format!(
            "--from must be a non-empty UTF-8 text file of at most {MAX_BYTES} bytes"
        )));
    }
    let digest = sha(&bytes);
    let id = unique(now, &digest);
    let manifest = Manifest {
        id: id.clone(),
        run: run_id.to_owned(),
        mode: p.mode.clone(),
        audience,
        destination,
        source_name: from
            .file_name()
            .map_or_else(|| "source".into(), |n| n.to_string_lossy().into_owned()),
        source_digest: digest.clone(),
        prepared: digest,
        recorded: crate::time::stamp(now),
    };
    let op = Op::CreateDir {
        path: format!("{run_dir}/{DIR}/{id}"),
        files: vec![
            (PAYLOAD.into(), bytes.clone()),
            (MANIFEST.into(), manifest_bytes(&manifest, bytes.len())),
        ],
    };
    Ok((op, id))
}

/// The candidate `named` (its ID, or the six hex digits that end it), or
/// the run's only candidate when none is named.
pub fn pick<'a>(cands: &'a [Candidate], named: Option<&str>, run: &str) -> Result<&'a Candidate> {
    let found: Vec<&Candidate> = match named {
        Some(n) => cands
            .iter()
            .filter(|c| c.id == n || c.id.ends_with(&format!("-{n}")))
            .collect(),
        None => cands.iter().collect(),
    };
    match (found.as_slice(), named) {
        ([one], _) => Ok(one),
        ([], Some(n)) => Err(KbError::Refused(format!("no candidate {n} in run {run}"))),
        ([], None) => Err(KbError::Refused(format!(
            "run {run} has no delivery candidate: writfold kb deliver prepare --mode M --from FILE"
        ))),
        (many, _) => {
            let ids: Vec<&str> = many.iter().map(|c| c.id.as_str()).collect();
            Err(KbError::Refused(format!(
                "{} candidates; name one: {}",
                many.len(),
                ids.join(", ")
            )))
        }
    }
}
