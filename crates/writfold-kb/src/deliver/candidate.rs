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
    /// `--invocation`: the agent invocation this candidate reports (#149).
    pub invocation: Option<String>,
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
    /// Its place among the run's candidates of its mode (0 when the
    /// manifest predates it): what decides which candidate is newest.
    pub seq: u64,
    /// The agent invocation it reports, when prepared with `--invocation`.
    pub invocation: Option<String>,
    /// The run's current step when it was prepared with `--invocation`
    /// (#164): `invocation.bound` judges this step, not whichever is
    /// current when the check runs. Absent on older candidates.
    pub step: Option<String>,
}

fn manifest_bytes(m: &Manifest, size: usize, rendering: &str) -> Vec<u8> {
    let value = json!({
        "schema": SCHEMA,
        "id": m.id,
        "run": m.run,
        "mode": m.mode,
        "audience": m.audience,
        "destination": m.destination,
        "source": {"name": m.source_name, "digest": m.source_digest},
        "candidate": {"file": PAYLOAD, "digest": m.prepared, "bytes": size},
        "rendering": rendering,
        "recorded": m.recorded,
        "seq": m.seq,
    });
    let mut value = value;
    if let (Some(id), Some(map)) = (&m.invocation, value.as_object_mut()) {
        map.insert("invocation".into(), json!(id));
    }
    if let (Some(step), Some(map)) = (&m.step, value.as_object_mut()) {
        map.insert("step".into(), json!(step));
    }
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
        seq: match value.get("seq") {
            None => 0,
            Some(n) => n.as_u64().ok_or("/seq is not a non-negative integer")?,
        },
        invocation: match value.get("invocation") {
            None => None,
            Some(_) => Some(field(&value, "/invocation")?),
        },
        step: match value.get("step") {
            None => None,
            Some(_) => Some(field(&value, "/step")?),
        },
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
/// operation and the candidate's ID. `step` is the run's current step,
/// recorded only with `--invocation` (#164).
pub fn plan(
    root: &Path,
    cwd: &Path,
    p: &Prepare,
    (run, step): ((&str, &str), Option<&str>),
    now: i64,
) -> Result<(Op, String)> {
    let (run_id, run_dir) = run;
    let m = mode(root, &p.mode)?;
    let audience = match (&m.audience, &p.audience) {
        (Some((fixed, line)), Some(asked)) if fixed != asked => {
            return Err(KbError::Refused(format!(
                "{} is for audience {fixed} (@{}:{line}); prepare without --audience",
                p.mode, m.source
            )));
        }
        (Some((fixed, _)), _) => fixed.clone(),
        (None, asked) => asked.clone().unwrap_or_else(|| "internal".into()),
    };
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
    let bytes = super::read::read_from(cwd, &p.from)?;
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
        source_name: match p.from.as_os_str() == "-" {
            true => "-".into(),
            false => from
                .file_name()
                .map_or_else(|| "source".into(), |n| n.to_string_lossy().into_owned()),
        },
        source_digest: digest.clone(),
        prepared: digest,
        recorded: crate::time::stamp(now),
        seq: super::step::next_seq(root, run_dir, run_id, &p.mode),
        invocation: invoked(root, run, p.invocation.as_deref())?,
        step: p.invocation.as_ref().and(step.map(str::to_owned)),
    };
    let op = Op::CreateDir {
        path: format!("{run_dir}/{DIR}/{id}"),
        files: vec![
            (PAYLOAD.into(), bytes.clone()),
            (
                MANIFEST.into(),
                manifest_bytes(&manifest, bytes.len(), &m.rendering),
            ),
        ],
    };
    Ok((op, id))
}

/// `--invocation ID`: an invocation recorded in this run (#149).
fn invoked(
    root: &Path,
    (run_id, run_dir): (&str, &str),
    id: Option<&str>,
) -> Result<Option<String>> {
    let Some(id) = id else {
        return Ok(None);
    };
    let path = crate::agents::invocation::path(run_dir, id);
    match root.join(&path).is_file()
        && (!id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
    {
        true => Ok(Some(id.to_owned())),
        false => Err(KbError::Refused(format!(
            "--invocation {id}: run {run_id} has no such invocation; record one with writfold kb agent prepare"
        ))),
    }
}

pub use super::pick::named as pick;
