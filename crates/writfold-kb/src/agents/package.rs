//! The package `agent prepare` writes for an external harness (#148):
//! `invocation.json` (the record), `instructions.md` (the profile's
//! instructions, then each skill's, in listed order), `input` (the input's
//! exact bytes) and, when the profile requires one, `packet.md` (the run's
//! context packet). Written into an empty or new directory outside the KB,
//! after the record is published. Tested, not proved.
use super::invocation::{self, Found, Record};
use super::prepare::{Gathered, Prepare};
use super::refs::Table;
use crate::emit::Output;
use crate::error::{KbError, Result};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const RECORD: &str = "invocation.json";
pub const INSTRUCTIONS: &str = "instructions.md";
pub const INPUT: &str = "input";
pub const PACKET: &str = "packet.md";

/// The package's files: name and bytes, in order.
pub(super) type Files = Vec<(&'static str, Vec<u8>)>;

/// `--out DIR`: a new directory, or an empty one.
pub fn fresh(cwd: &Path, out: &Path) -> Result<PathBuf> {
    let dir = cwd.join(out);
    let empty = match std::fs::read_dir(&dir) {
        Ok(mut entries) => entries.next().is_none(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => return Err(e.into()),
    };
    match empty {
        true => Ok(dir),
        false => Err(KbError::Usage(format!(
            "--out {}: not empty; name a new or empty directory",
            out.display()
        ))),
    }
}

/// The package's digest: each file's name, length and bytes, in order.
fn digest(files: &[(&'static str, Vec<u8>)]) -> String {
    let mut h = Sha256::new();
    for (name, bytes) in files {
        h.update(name.as_bytes());
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes);
    }
    format!("sha256:{}", crate::index::hex(&h.finalize()))
}

fn retry(found: &[Found], run: &str, retry_of: Option<&str>) -> Result<Option<String>> {
    let Some(id) = retry_of else {
        return Ok(None);
    };
    match found.iter().any(|f| f.id == id) {
        true => Ok(Some(id.to_owned())),
        false => Err(KbError::Usage(format!(
            "--retry-of {id}: run {run} has no such invocation"
        ))),
    }
}

/// The record and the package files of an admitted prepare.
pub(super) fn build(
    t: &Table,
    g: &Gathered<'_>,
    a: &Prepare,
    (run, found): (&str, &[Found]),
    now: i64,
) -> Result<(Record, Files)> {
    let refused = |why: String| KbError::Refused(format!("{why}; nothing written"));
    let (Some(p), Ok(closure), Some(step), Ok(input)) = (g.profile, &g.closure, &g.step, &g.input)
    else {
        return Err(KbError::Refused("prepare was not admitted".into()));
    };
    let mut files = vec![
        (
            INSTRUCTIONS,
            super::inspect::instructions(t, p).map_err(refused)?,
        ),
        (INPUT, input.clone()),
    ];
    if let Some((text, _)) = &g.packet {
        files.push((PACKET, text.clone()));
    }
    let by = match &a.by {
        Some(by) => Some(crate::write::one_line("--by", by)?),
        None => None,
    };
    let name =
        (a.input.file_name()).map_or_else(|| "input".into(), |n| n.to_string_lossy().into_owned());
    let seq = (found.iter())
        .filter_map(|f| f.record.as_ref().ok().map(|r| r.seq))
        .max()
        .unwrap_or(0)
        + 1;
    let record = Record {
        id: crate::write::unique(now, closure),
        seq,
        run: run.to_owned(),
        step: step.clone(),
        agent: p.id.clone(),
        closure: closure.clone(),
        definitions: super::inspect::used(t, p).map_err(refused)?,
        input: (name, input.len() as u64, invocation::sha(input)),
        packet: (g.packet.as_ref()).map(|(text, whole)| (invocation::sha(text), *whole)),
        package: digest(&files),
        retry_of: retry(found, run, a.retry_of.as_deref())?,
        by,
        model: p.model.clone(),
        for_item: match &a.for_item {
            None => None,
            Some(text) => Some(invocation::for_item(text).ok_or_else(|| {
                KbError::Usage(format!(
                    "--for {text}: write GATE.ITEM, a checklist gate and one of its items"
                ))
            })?),
        },
        needs: (g.needs.iter())
            .map(|n| (n.gate.clone(), n.basis.clone()))
            .collect(),
        recorded: crate::time::stamp(now),
    };
    Ok((record, files))
}

/// Write the package into `dir`.
pub(super) fn write(dir: &Path, record: &Record, files: &[(&'static str, Vec<u8>)]) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    for (name, bytes) in files {
        std::fs::write(dir.join(name), bytes)?;
    }
    std::fs::write(dir.join(RECORD), invocation::bytes(record))?;
    Ok(())
}

/// What `agent prepare` prints.
pub(super) fn shown(record: &Record, path: &str, out: &Path, json: bool) -> Output {
    let next = format!(
        "give the package to a harness; record its report with writfold kb deliver prepare --mode MODE --from REPORT --invocation {}",
        record.id
    );
    let text = match json {
        true => format!(
            "{}\n",
            json!({"writfold_kb": 1, "verb": "agent prepare", "invocation": record.id,
                "record": path, "step": record.step, "agent": record.agent,
                "closure": record.closure, "package": out.display().to_string(),
                "trailer": {"coverage": "complete", "answer": "prepared", "next": next}})
        ),
        false => format!(
            "invocation {} {} at {}\n# record={path}\n# package={}\n# closure={}\n# note: Writfold starts no model; {next}\n# coverage=complete answer=prepared\n",
            record.id,
            record.agent,
            record.step,
            out.display(),
            record.closure
        ),
    };
    Output {
        text,
        exit: 0,
        report: None,
        shown: None,
        drawn: None,
    }
}
