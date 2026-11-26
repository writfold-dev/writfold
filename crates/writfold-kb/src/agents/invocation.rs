//! Invocation records (#148): `runs/<run>/invocations/<id>.json`, schema
//! `writfold-invocation/1`, one create-new file per `agent prepare`. A
//! record holds paths and digests, never copies: Git holds the bytes.
//!
//! State is derived on every read, never stored ([`state`]): `prepared`
//! (no live candidate names it), `delivered` (live candidates name it, all
//! with the same bytes), `conflicting` (live candidates with different
//! bytes), `retried` (a later invocation names it as `retry_of`).
use crate::deliver::candidate::{self, Candidate};
use serde_json::{Value, json};
use std::path::Path;

pub const SCHEMA: &str = "writfold-invocation/1";
/// The directory of a run that holds its invocations.
pub const DIR: &str = "invocations";

/// One definition file of the closure, as recorded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Used {
    /// `agent`, `skill`, `tool`, or `instructions`.
    pub kind: String,
    pub id: String,
    /// As rows show it: KB-relative, or `<repo>:<path>`.
    pub path: String,
    pub digest: String,
}

/// An invocation record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    pub id: String,
    pub seq: u64,
    pub run: String,
    /// The step path; a loop round is part of it.
    pub step: String,
    pub agent: String,
    pub closure: String,
    pub definitions: Vec<Used>,
    /// The input's file name, size and digest.
    pub input: (String, u64, String),
    /// The packet's digest and whether it was complete, when one was made.
    pub packet: Option<(String, bool)>,
    pub package: String,
    pub retry_of: Option<String>,
    pub by: Option<String>,
    pub model: Option<String>,
    /// `--for GATE.ITEM`: the checklist item this invocation's report
    /// answers (#156); written only when given.
    pub for_item: Option<(String, String)>,
    /// Each gate the profile or a skill `needs`, with the receipt or
    /// statement it stood on at prepare (#167); written only when there
    /// are needs.
    pub needs: Vec<(String, Option<String>)>,
    pub recorded: String,
}

pub fn bytes(r: &Record) -> Vec<u8> {
    let defs: Vec<Value> = (r.definitions.iter())
        .map(|d| json!({"kind": d.kind, "id": d.id, "path": d.path, "digest": d.digest}))
        .collect();
    let value = json!({
        "schema": SCHEMA,
        "id": r.id,
        "seq": r.seq,
        "run": r.run,
        "step": r.step,
        "agent": r.agent,
        "closure": r.closure,
        "definitions": defs,
        "input": {"name": r.input.0, "bytes": r.input.1, "digest": r.input.2},
        "packet": r.packet.as_ref().map(|(d, c)| json!({"digest": d, "complete": c})),
        "package": r.package,
        "retry_of": r.retry_of,
        "by": r.by,
        "model": r.model,
        "recorded": r.recorded,
    });
    let mut value = value;
    if let (Some((gate, item)), Some(map)) = (&r.for_item, value.as_object_mut()) {
        map.insert("for".into(), json!(format!("{gate}.{item}")));
    }
    if let (false, Some(map)) = (r.needs.is_empty(), value.as_object_mut()) {
        let needs: Vec<Value> = (r.needs.iter())
            .map(|(gate, on)| json!({"gate": gate, "on": on}))
            .collect();
        map.insert("needs".into(), json!(needs));
    }
    let mut out = serde_json::to_vec_pretty(&value).unwrap_or_default();
    out.push(b'\n');
    out
}

/// `GATE.ITEM`: the gate is a slug, so it ends at the first `.`.
pub fn for_item(text: &str) -> Option<(String, String)> {
    let (gate, item) = text.split_once('.')?;
    (crate::write::is_slug(gate) && !item.is_empty() && !item.contains(['\n', '\r']))
        .then(|| (gate.to_owned(), item.to_owned()))
}

fn text(v: &Value, pointer: &str) -> Result<String, String> {
    (v.pointer(pointer).and_then(Value::as_str))
        .filter(|t| !t.is_empty() && !t.contains(['\n', '\r']))
        .map(str::to_owned)
        .ok_or_else(|| format!("{pointer} is missing or not a one-line string"))
}

fn optional(v: &Value, pointer: &str) -> Result<Option<String>, String> {
    match v.pointer(pointer) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => text(v, pointer).map(Some),
    }
}

fn number(v: &Value, pointer: &str) -> Result<u64, String> {
    (v.pointer(pointer).and_then(Value::as_u64))
        .ok_or_else(|| format!("{pointer} is not a non-negative integer"))
}

/// `needs`, when present: each gate and what it stood on.
fn needs(v: &Value) -> Result<Vec<(String, Option<String>)>, String> {
    let Some(list) = v.get("needs") else {
        return Ok(Vec::new());
    };
    let list = list.as_array().ok_or("/needs is not a list")?;
    (list.iter())
        .map(|n| Ok((text(n, "/gate")?, optional(n, "/on")?)))
        .collect()
}

fn definitions(v: &Value) -> Result<Vec<Used>, String> {
    let list =
        (v.get("definitions").and_then(Value::as_array)).ok_or("/definitions is not a list")?;
    list.iter()
        .map(|d| {
            Ok(Used {
                kind: text(d, "/kind")?,
                id: text(d, "/id")?,
                path: text(d, "/path")?,
                digest: text(d, "/digest")?,
            })
        })
        .collect()
}

/// Read a record filed under `run` as invocation `id`.
pub fn parse(text_: &str, run: &str, id: &str) -> Result<Record, String> {
    let v: Value = serde_json::from_str(text_).map_err(|e| format!("not JSON ({e})"))?;
    if text(&v, "/schema")? != SCHEMA {
        return Err(format!("schema is not {SCHEMA}"));
    }
    let packet = match v.get("packet") {
        None | Some(Value::Null) => None,
        Some(p) => Some((
            text(p, "/digest")?,
            (p.get("complete").and_then(Value::as_bool))
                .ok_or("/packet/complete is not a boolean")?,
        )),
    };
    let r = Record {
        id: text(&v, "/id")?,
        seq: number(&v, "/seq")?,
        run: text(&v, "/run")?,
        step: text(&v, "/step")?,
        agent: text(&v, "/agent")?,
        closure: text(&v, "/closure")?,
        definitions: definitions(&v)?,
        input: (
            text(&v, "/input/name")?,
            number(&v, "/input/bytes")?,
            text(&v, "/input/digest")?,
        ),
        packet,
        package: text(&v, "/package")?,
        retry_of: optional(&v, "/retry_of")?,
        by: optional(&v, "/by")?,
        model: optional(&v, "/model")?,
        for_item: match optional(&v, "/for")? {
            None => None,
            Some(f) => Some(for_item(&f).ok_or("/for is not GATE.ITEM")?),
        },
        needs: needs(&v)?,
        recorded: text(&v, "/recorded")?,
    };
    if r.id != id || r.run != run {
        return Err("its id or run does not match where it is filed".into());
    }
    Ok(r)
}

/// The `--for` of the invocation record at KB path `path`
/// (`runs/<run>/invocations/<id>.json`), when it reads.
pub fn read_for(root: &Path, path: &str) -> Option<(String, String)> {
    let mut parts = path.strip_prefix("runs/")?.split('/');
    let run_dir = parts.next()?;
    let id = parts.nth(1)?.strip_suffix(".json")?;
    let text = std::fs::read_to_string(root.join(path)).ok()?;
    let run = run_of(root, run_dir)?;
    parse(&text, &run, id).ok()?.for_item
}

/// The run id a run directory holds (its manifest's `run_id`).
fn run_of(root: &Path, run_dir: &str) -> Option<String> {
    let text =
        std::fs::read_to_string(root.join("runs").join(run_dir).join("manifest.json")).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v.get("run_id").and_then(Value::as_str).map(str::to_owned)
}

/// One record file of a run, as found.
pub struct Found {
    pub id: String,
    /// KB-relative path.
    pub path: String,
    pub record: Result<Record, String>,
}

/// The KB path of invocation `id` of the run filed at `run_dir`.
pub fn path(run_dir: &str, id: &str) -> String {
    format!("{run_dir}/{DIR}/{id}.json")
}

/// Every invocation of the run whose directory is `run_dir`, oldest first.
pub fn list(root: &Path, run_dir: &str, run: &str) -> Vec<Found> {
    let Ok(entries) = std::fs::read_dir(root.join(run_dir).join(DIR)) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = (entries.flatten())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.'))
        .filter_map(|n| n.strip_suffix(".json").map(str::to_owned))
        .collect();
    ids.sort();
    ids.into_iter()
        .map(|id| {
            let path = path(run_dir, &id);
            let record = std::fs::read_to_string(root.join(&path))
                .map_err(|e| format!("unreadable ({e})"))
                .and_then(|t| parse(&t, run, &id));
            Found { id, path, record }
        })
        .collect()
}

/// The live (not withdrawn) candidates naming invocation `id`.
pub fn naming<'a>(cands: &'a [Candidate], withdrawn: &[String], id: &str) -> Vec<&'a Candidate> {
    (cands.iter())
        .filter(|c| !withdrawn.contains(&c.id))
        .filter(|c| (c.manifest.as_ref()).is_ok_and(|m| m.invocation.as_deref() == Some(id)))
        .collect()
}

/// Whether every candidate holds the same bytes (true for none or one).
pub fn single(named: &[&Candidate]) -> bool {
    let digests: Vec<String> = named.iter().map(|c| c.digest()).collect();
    digests.windows(2).all(|w| w[0] == w[1]) && !digests.iter().any(|d| d == "missing")
}

/// The derived state of invocation `id`.
pub fn state(found: &[Found], cands: &[Candidate], withdrawn: &[String], id: &str) -> &'static str {
    let retried = (found.iter())
        .any(|f| (f.record.as_ref()).is_ok_and(|r| r.retry_of.as_deref() == Some(id)));
    let named = naming(cands, withdrawn, id);
    match (named.is_empty(), single(&named), retried) {
        (false, false, _) => "conflicting",
        (false, true, _) => "delivered",
        (true, _, true) => "retried",
        (true, _, false) => "prepared",
    }
}

/// The digest of bytes, as records write it.
pub fn sha(bytes: &[u8]) -> String {
    candidate::sha(bytes)
}
