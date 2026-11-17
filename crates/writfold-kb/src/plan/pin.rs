//! Policy pins (policy spec 2.5): `runs/<run>/policy/pin-<seq>-<hex>.json`
//! (`<hex>`, the first eight hex digits of the file's own SHA-256, keeps
//! two clones' pins of the same `seq` from colliding when they merge),
//! written with every splice (`plan set`, `plan use`), in the same write
//! plan and ordered before the splice files. A pin holds each gate the
//! spliced closure names that no earlier pin names, with its definition
//! digest; the first pin also holds the required file's digest and the
//! templates it locks (`unskippable`). Each file is create-new and ends
//! with a digest over the rest, like a plan event; an edited pin reads
//! invalid and the run is then judged moved. A `plan repin` writes a pin
//! of kind `repin` ([`super::repin`]): every gate of the plan, the
//! required file and lock list as they are now, a `reason` and `by`, and
//! under `steps` the digest of each done step that differs from the
//! gate's. Whether the policy moved is decided by
//! [`writfold_core::policy::pin_one`] and `pin_all` ([`super::drift`]),
//! and which entry is in force by [`writfold_core::policy::repin::in_force`]
//! ([`gate_in_force`], [`required_in_force`]); this module reads and
//! writes the files and gathers the entries.
use crate::error::{KbError, Result};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

pub const SCHEMA: &str = "writfold-policy-pin/1";
/// The directory of a run that holds pins.
pub const DIR: &str = "policy";

/// What the first pin records about `required.toml`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequiredPin {
    pub source: String,
    /// `sha256:<hex>` of the file's bytes, or `absent`.
    pub digest: String,
    pub unskippable: Vec<String>,
}

/// One pin file, read back.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pin {
    /// KB-relative path.
    pub path: String,
    pub seq: u64,
    /// `set`, `use` or `repin`.
    pub kind: String,
    /// The splice files (names under `plan/`) this pin was written with.
    pub covers: Vec<String>,
    pub required: Option<RequiredPin>,
    /// `(gate, definition digest)`.
    pub gates: Vec<(String, String)>,
    pub recorded: String,
    /// A repin only: `(step path, gate, definition digest)` for each done
    /// step whose record names another digest than the gate's entry.
    pub steps: Vec<(String, String, String)>,
    /// A repin only: why the run adopted the policy, and who says so.
    pub reason: Option<String>,
    pub by: Option<String>,
}

fn digest_of(value: &Value) -> String {
    let canonical = crate::gate::receipt::canonical(value);
    format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(canonical.as_bytes()))
    )
}

/// The file's bytes: two-space JSON with its digest last.
pub fn bytes(run: &str, pin: &Pin) -> Vec<u8> {
    let gates: Vec<Value> = (pin.gates.iter())
        .map(|(g, d)| json!({"gate": g, "definition": d}))
        .collect();
    let mut value = json!({
        "schema": SCHEMA, "run": run, "seq": pin.seq, "kind": pin.kind,
        "covers": pin.covers, "gates": gates, "recorded": pin.recorded,
    });
    if let Some(r) = &pin.required {
        value["required"] =
            json!({"source": r.source, "digest": r.digest, "unskippable": r.unskippable});
    }
    // The keys of a repin; absent otherwise, so a `set` or `use` pin keeps
    // its bytes.
    if !pin.steps.is_empty() {
        let steps: Vec<Value> = (pin.steps.iter())
            .map(|(s, g, d)| json!({"step": s, "gate": g, "definition": d}))
            .collect();
        value["steps"] = json!(steps);
    }
    if let Some(reason) = &pin.reason {
        value["reason"] = json!(reason);
    }
    if let Some(by) = &pin.by {
        value["by"] = json!(by);
    }
    let digest = digest_of(&value);
    value["digest"] = json!(digest);
    let mut out = serde_json::to_vec_pretty(&value).unwrap_or_default();
    out.push(b'\n');
    out
}

fn strings(v: &Value, key: &str) -> std::result::Result<Vec<String>, String> {
    let list = v
        .get(key)
        .and_then(Value::as_array)
        .ok_or(format!("{key} is missing"))?;
    (list.iter())
        .map(|s| {
            s.as_str()
                .map(str::to_owned)
                .ok_or(format!("{key} holds a non-string"))
        })
        .collect()
}

fn text(v: &Value, key: &str) -> std::result::Result<String, String> {
    (v.get(key).and_then(Value::as_str).map(str::to_owned)).ok_or(format!("{key} is missing"))
}

/// Read one pin of `run` filed at `path`, checking its schema and digest.
pub fn parse(raw: &str, run: &str, path: &str) -> std::result::Result<Pin, String> {
    let mut value: Value = serde_json::from_str(raw).map_err(|e| format!("not JSON ({e})"))?;
    let claimed = value.as_object_mut().and_then(|m| m.remove("digest"));
    if claimed.as_ref().and_then(Value::as_str) != Some(digest_of(&value).as_str()) {
        return Err(
            "not written by writfold kb plan (its digest does not match its content)".into(),
        );
    }
    if text(&value, "schema")? != SCHEMA || text(&value, "run")? != run {
        return Err("another schema or another run".into());
    }
    let required = match value.get("required") {
        None => None,
        Some(r) => Some(RequiredPin {
            source: text(r, "source")?,
            digest: text(r, "digest")?,
            unskippable: strings(r, "unskippable")?,
        }),
    };
    let gates = (value
        .get("gates")
        .and_then(Value::as_array)
        .ok_or("gates is missing")?)
    .iter()
    .map(|g| Ok((text(g, "gate")?, text(g, "definition")?)))
    .collect::<std::result::Result<Vec<_>, String>>()?;
    let steps = (value.get("steps").and_then(Value::as_array))
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .map(|s| Ok((text(s, "step")?, text(s, "gate")?, text(s, "definition")?)))
        .collect::<std::result::Result<Vec<_>, String>>()?;
    let optional = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    Ok(Pin {
        path: path.to_owned(),
        seq: value
            .get("seq")
            .and_then(Value::as_u64)
            .ok_or("seq is missing")?,
        kind: text(&value, "kind")?,
        covers: strings(&value, "covers")?,
        required,
        gates,
        recorded: text(&value, "recorded")?,
        steps,
        reason: optional("reason"),
        by: optional("by"),
    })
}

/// Every pin of the run, by `seq`, and the files that do not read.
pub fn read(root: &Path, run_dir: &str, run: &str) -> (Vec<Pin>, Vec<(String, String)>) {
    let dir = root.join(run_dir).join(DIR);
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("pin-") && n.ends_with(".json"))
        .collect();
    names.sort();
    let (mut pins, mut invalid) = (Vec::new(), Vec::new());
    for name in names {
        let path = format!("{run_dir}/{DIR}/{name}");
        let raw = std::fs::read_to_string(root.join(&path)).unwrap_or_default();
        match parse(&raw, run, &path) {
            Ok(pin) => pins.push(pin),
            Err(why) => invalid.push((path, why)),
        }
    }
    pins.sort_by(|a, b| (a.seq, &a.path).cmp(&(b.seq, &b.path)));
    (pins, invalid)
}

/// The path of a pin with these bytes.
pub fn path(run_dir: &str, seq: u64, bytes: &[u8]) -> String {
    let hex = crate::index::hex(&Sha256::digest(bytes));
    format!(
        "{run_dir}/{DIR}/pin-{seq}-{}.json",
        hex.get(..8).unwrap_or(&hex)
    )
}

/// Whether the newest pin is a leftover: it covers splice files none of
/// which was written (an interrupted plan write).
pub fn leftover(root: &Path, existing: &[Pin]) -> bool {
    let Some(last) = existing.last() else {
        return false;
    };
    let dir = Path::new(&last.path).parent().and_then(Path::parent);
    let written =
        |c: &String| dir.is_some_and(|d| root.join(d).join(super::event::DIR).join(c).is_file());
    !last.covers.is_empty() && !last.covers.iter().any(written)
}

/// The pin to write with these splices, or `None` to reuse a leftover pin
/// that holds what would be written now. A leftover that holds something
/// else is refused, naming the file to delete. `prior` are the pins before
/// any leftover.
pub fn next(root: &Path, existing: &[Pin], mut new: Pin, taken: u64) -> Result<Option<Pin>> {
    if leftover(root, existing)
        && let Some(last) = existing.last()
    {
        let same =
            last.gates == new.gates && last.required == new.required && last.kind == new.kind;
        return match same {
            true => Ok(None),
            false => Err(KbError::Refused(format!(
                "{} covers splices that were never written (an interrupted plan write) and holds another policy; delete it and run the command again",
                last.path
            ))),
        };
    }
    new.seq = existing.iter().map(|p| p.seq).max().unwrap_or(0).max(taken) + 1;
    Ok(Some(new))
}

/// The highest `seq` any pin file name takes, valid or not.
pub fn max_seq(root: &Path, run_dir: &str) -> u64 {
    std::fs::read_dir(root.join(run_dir).join(DIR))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let rest = name.strip_prefix("pin-")?.strip_suffix(".json")?;
            rest.split('-').next()?.parse::<u64>().ok()
        })
        .max()
        .unwrap_or(0)
}

/// The pinned digest of `gate` for the step at `step`, and the pin that
/// holds it: the newest entry that applies
/// ([`writfold_core::policy::repin::in_force`]). The entries are every
/// pin's, by `seq`: its gates, then a repin's step entries; one applies
/// when it names the gate and, if it names a step, that step. `step` is
/// `None` to ask for the gate's own entry.
pub fn gate_in_force<'a>(
    pins: &'a [Pin],
    gate: &str,
    step: Option<&str>,
) -> Option<(&'a str, &'a Pin)> {
    let mut entries: Vec<(&str, &Pin)> = Vec::new();
    let mut applies: Vec<bool> = Vec::new();
    for pin in pins {
        for (g, digest) in &pin.gates {
            entries.push((digest, pin));
            applies.push(g == gate);
        }
        for (s, g, digest) in &pin.steps {
            entries.push((digest, pin));
            applies.push(g == gate && step == Some(s.as_str()));
        }
    }
    let at = writfold_core::policy::repin::in_force(&applies)?;
    entries.get(at).copied()
}

/// The required file's pin in force: the newest pin that carries one (the
/// run's first pin, or its newest repin).
pub fn required_in_force(pins: &[Pin]) -> Option<(&Pin, &RequiredPin)> {
    let applies: Vec<bool> = pins.iter().map(|p| p.required.is_some()).collect();
    let pin = pins.get(writfold_core::policy::repin::in_force(&applies)?)?;
    pin.required.as_ref().map(|r| (pin, r))
}
