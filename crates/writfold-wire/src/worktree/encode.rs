use super::*;
use cbor::Value::{self, Array, Bytes, Null, Signed, Text, Unsigned};

fn envelope(kind: &str, payload: &[Value<'_>], limits: Limits) -> Result<Vec<u8>> {
    Ok(cbor::encode(
        &Array(&[Text(PROFILE), Text(ALGORITHM), Text(kind), Array(payload)]),
        limits,
    )?
    .into_bytes())
}
pub(super) fn record(record: &Record, limits: Limits) -> Result<Vec<u8>> {
    match record {
        Record::Recipe(v) => recipe_digest(v, limits),
        Record::Capture(v) => capture_digest(v, limits),
        Record::Candidate(v) => candidate_digest(v, limits),
        Record::PreparedEdit(v) => prepared_edit_digest(v, limits),
        Record::RequestKey(v) => request_key_digest(v, limits),
        Record::Observation(v) => observation_digest(v, limits),
    }
}
pub(super) fn recipe_digest(v: &Recipe, limits: Limits) -> Result<Vec<u8>> {
    envelope(
        "recipe",
        &[Text(v.language_profile()), Bytes(v.source())],
        limits,
    )
}
pub(super) fn capture_digest(v: &Capture, limits: Limits) -> Result<Vec<u8>> {
    let s = v.scope();
    let m = v.metadata();
    let hash = raw_sha256(v.content());
    envelope(
        "capture",
        &[
            Array(&[
                Text(s.realm().as_str()),
                Text(s.run().as_str()),
                Text(s.repo().as_str()),
                Text(s.worktree().as_str()),
                Text(s.binding().as_str()),
            ]),
            Text(v.path()),
            Bytes(v.content()),
            Bytes(&hash),
            Unsigned(v.content().len() as u64),
            Text(PosixMetadata::POLICY),
            Array(&[
                Unsigned(m.device()),
                Unsigned(m.inode()),
                Unsigned(m.mode() as u64),
                Unsigned(m.uid() as u64),
                Unsigned(m.gid() as u64),
                Unsigned(1),
                Array(&[
                    Signed(m.mtime().seconds()),
                    Unsigned(m.mtime().nanoseconds() as u64),
                ]),
                Array(&[
                    Signed(m.ctime().seconds()),
                    Unsigned(m.ctime().nanoseconds() as u64),
                ]),
            ]),
        ],
        limits,
    )
}
pub(super) fn candidate_digest(v: &Candidate, limits: Limits) -> Result<Vec<u8>> {
    let capture = v.capture().to_string();
    let hash = raw_sha256(v.content());
    envelope(
        "candidate",
        &[
            Text(&capture),
            Bytes(v.content()),
            Bytes(&hash),
            Unsigned(v.content().len() as u64),
        ],
        limits,
    )
}
pub(super) fn request_key_digest(v: &RequestKey, limits: Limits) -> Result<Vec<u8>> {
    envelope(
        "request-key",
        &[
            Text(v.principal().as_str()),
            Text(v.realm().as_str()),
            Text(v.run().as_str()),
            Bytes(v.opaque_key()),
        ],
        limits,
    )
}
pub(super) fn prepared_edit_digest(v: &PreparedEdit, limits: Limits) -> Result<Vec<u8>> {
    let recipe = v.recipe().to_string();
    let capture = v.capture().to_string();
    let candidate = v.candidate().to_string();
    let splices: Vec<_> = v
        .splices()
        .iter()
        .map(|s| {
            [
                Unsigned(s.start()),
                Unsigned(s.end()),
                Bytes(s.replacement()),
                Unsigned(s.origin().start()),
                Unsigned(s.origin().end()),
            ]
        })
        .collect();
    let splice_values: Vec<_> = splices.iter().map(|s| Array(s)).collect();
    let reads: Vec<_> = v
        .read_set()
        .iter()
        .map(|r| [Unsigned(r.start()), Bytes(r.bytes())])
        .collect();
    let read_values: Vec<_> = reads.iter().map(|r| Array(r)).collect();
    envelope(
        "prepared-edit",
        &[
            Text("replace-content"),
            Text(&recipe),
            Text(&capture),
            Text(&candidate),
            Array(&[Text(v.compiler().profile()), Bytes(v.compiler().sha256())]),
            Array(&[Text(v.kernel().profile()), Bytes(v.kernel().sha256())]),
            Array(&splice_values),
            Array(&read_values),
        ],
        limits,
    )
}
pub(super) fn observation_digest(v: &EffectObservation, limits: Limits) -> Result<Vec<u8>> {
    let intent = v.intent().to_string();
    let stamp = v.stamp().to_string();
    let prepared = v.prepared().to_string();
    envelope(
        "observation",
        &[
            Text(&intent),
            Text(&stamp),
            Text(&prepared),
            Text(outcome(v.outcome())),
            Text(content(v.content())),
            Text(durability(v.durability())),
            Text(metadata(v.metadata())),
            v.observed_sha256().map_or(Null, |h| Bytes(h)),
        ],
        limits,
    )
}
pub(super) fn outcome(v: Outcome) -> &'static str {
    match v {
        Outcome::Confirmed => "confirmed",
        Outcome::NoEffect => "no-effect",
        Outcome::Unknown => "unknown",
        Outcome::Partial => "partial",
    }
}
pub(super) fn content(v: &ContentObservation) -> &'static str {
    match v {
        ContentObservation::Original(_) => "original",
        ContentObservation::Candidate(_) => "candidate",
        ContentObservation::Other(_) => "other",
        ContentObservation::Unobserved => "unobserved",
    }
}
pub(super) fn durability(v: DurabilityObservation) -> &'static str {
    match v {
        DurabilityObservation::Durable => "durable",
        DurabilityObservation::NotDurable => "not-durable",
        DurabilityObservation::Unknown => "unknown",
    }
}
pub(super) fn metadata(v: MetadataObservation) -> &'static str {
    match v {
        MetadataObservation::Preserved => "preserved",
        MetadataObservation::Changed => "changed",
        MetadataObservation::Unobserved => "unobserved",
    }
}
