use super::*;
use serde_json::{Value, json};

pub(super) fn record(v: &Record, limits: Limits) -> Result<Vec<u8>> {
    let id = v.identity(limits)?;
    let payload = match v {
        Record::Recipe(v) => {
            json!({"language_profile": v.language_profile(), "source": hex(v.source())})
        }
        Record::Capture(v) => capture(v),
        Record::Candidate(v) => {
            json!({"capture": v.capture().to_string(), "content": hex(v.content()), "content_sha256": hex(&raw_sha256(v.content())), "length": v.content().len()})
        }
        Record::PreparedEdit(v) => prepared(v),
        Record::RequestKey(v) => {
            json!({"principal": v.principal().as_str(), "realm": v.realm().as_str(), "run": v.run().as_str(), "opaque_key": hex(v.opaque_key())})
        }
        Record::Observation(v) => observation(v),
    };
    let raw = crate::RawJson {
        value: json!({"profile": PROFILE, "algorithm": ALGORITHM, "kind": v.kind(), "id": id, "payload": payload}),
    };
    let bytes = crate::canonical(&raw, limits.bytes())?;
    // Apply the same depth/node/container bounds as decoding, not only byte bounds.
    crate::RawJson::parse(&bytes, limits)?;
    Ok(bytes)
}
fn capture(v: &Capture) -> Value {
    let s = v.scope();
    let m = v.metadata();
    json!({"scope": {"realm": s.realm().as_str(), "run": s.run().as_str(), "repo": s.repo().as_str(), "worktree": s.worktree().as_str(), "binding": s.binding().as_str()},
        "path": v.path(), "content": hex(v.content()), "content_sha256": hex(&raw_sha256(v.content())), "length": v.content().len(),
        "metadata_policy": PosixMetadata::POLICY,
        "metadata": {"device": m.device(), "inode": m.inode(), "mode": m.mode(), "uid": m.uid(), "gid": m.gid(), "links": 1,
            "mtime": {"seconds": m.mtime().seconds(), "nanoseconds": m.mtime().nanoseconds()},
            "ctime": {"seconds": m.ctime().seconds(), "nanoseconds": m.ctime().nanoseconds()}}})
}
fn artifact(v: &Artifact) -> Value {
    json!({"profile": v.profile(), "sha256": hex(v.sha256())})
}
fn prepared(v: &PreparedEdit) -> Value {
    let splices: Vec<_> = v
        .splices()
        .iter()
        .map(|s| {
            json!({"start": s.start(), "end": s.end(), "replacement": hex(s.replacement()),
        "origin": {"start": s.origin().start(), "end": s.origin().end()}})
        })
        .collect();
    let reads: Vec<_> = v
        .read_set()
        .iter()
        .map(|r| json!({"start": r.start(), "bytes": hex(r.bytes())}))
        .collect();
    json!({"operation": "replace-content", "recipe": v.recipe().to_string(), "capture": v.capture().to_string(), "candidate": v.candidate().to_string(),
        "compiler": artifact(v.compiler()), "kernel": artifact(v.kernel()), "splices": splices, "read_set": reads})
}
fn observation(v: &EffectObservation) -> Value {
    json!({"intent": v.intent().to_string(), "stamp": v.stamp().to_string(), "prepared": v.prepared().to_string(),
        "outcome": encode::outcome(v.outcome()), "content": encode::content(v.content()), "durability": encode::durability(v.durability()),
        "metadata": encode::metadata(v.metadata()), "observed_sha256": v.observed_sha256().map(|h| hex(h))})
}
