//! Step-by-step agreement between Lean `append` and the Rust ledger kernel on
//! the exported reservation-calibration scenarios. Every step's outcome,
//! resulting head and inventory epoch must match exactly. Finite executable
//! agreement, not a refinement proof.
use serde_json::Value;
use writfold_domain::engineering::ledger::ReducerProfile;
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};
use writfold_domain::{LedgerSequence, StreamId};
use writfold_kernel::ledger::{Head, Record, Store, Validator};

type Fallible<T = ()> = Result<T, Box<dyn std::error::Error>>;
const CASES: &str = include_str!("fixtures/ledger-lean-cases.json");

#[derive(Clone, Debug, Eq, PartialEq)]
enum Payload {
    Reserve(u64),
    Release(u64),
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct Historical {
    actor: u64,
    admitted_actor: u64,
    capacity: u64,
    authority_object: BlobDigest,
}
type R = Record<Payload, Historical>;

/// Rust copy of `Ledger.Calibration.validate`. Lean's natural subtraction
/// truncates at zero, so release uses saturating subtraction here too.
struct Reservation;
fn occupied(prefix: &[R]) -> u64 {
    prefix.iter().fold(0, |total, r| match r.payload {
        Payload::Reserve(n) => total.saturating_add(n),
        Payload::Release(n) => total.saturating_sub(n),
    })
}
impl Validator<Payload, Historical> for Reservation {
    fn validate(&self, prefix: &[R], record: &R) -> bool {
        let h = &record.historical;
        let used = occupied(prefix);
        h.actor == h.admitted_actor
            && record.profile == ReducerProfile::EngineeringV1
            && record.closure.contains(&h.authority_object)
            && match record.payload {
                Payload::Reserve(n) => {
                    0 < n && used.checked_add(n).is_some_and(|t| t <= h.capacity)
                }
                Payload::Release(n) => 0 < n && n <= used,
            }
    }
}

fn blob(n: u64) -> BlobDigest {
    let mut bytes = [0u8; 32];
    bytes[24..].copy_from_slice(&n.to_be_bytes());
    BlobDigest::from_digest(bytes)
}
fn num(v: &Value, key: &str) -> Fallible<u64> {
    v.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing number {key}").into())
}
fn stream(n: u64) -> Fallible<StreamId> {
    Ok(format!("stream-{n}").parse()?)
}
fn blobs(v: &Value, key: &str) -> Fallible<Vec<BlobDigest>> {
    v.get(key)
        .and_then(Value::as_array)
        .ok_or(key)?
        .iter()
        .map(|x| x.as_u64().map(blob).ok_or_else(|| "blob".into()))
        .collect()
}

/// A Lean value with no Rust domain representation (sequence 0, an unknown
/// reducer profile) is a refusal at the domain boundary.
fn record(v: &Value) -> Fallible<Option<R>> {
    let Ok(sequence) = LedgerSequence::new(num(v, "sequence")?) else {
        return Ok(None);
    };
    if num(v, "profile")? != 1 {
        return Ok(None);
    }
    let payload = v.get("payload").ok_or("payload")?;
    let units = num(payload, "units")?;
    Ok(Some(Record {
        id: format!("record-{}", num(v, "id")?).parse()?,
        stream: stream(num(v, "stream")?)?,
        sequence,
        predecessor: match v.get("predecessor") {
            Some(Value::Null) | None => None,
            Some(p) => Some(format!("record-{}", p.as_u64().ok_or("predecessor")?).parse()?),
        },
        profile: ReducerProfile::EngineeringV1,
        payload: match payload.get("kind").and_then(Value::as_str) {
            Some("reserve") => Payload::Reserve(units),
            Some("release") => Payload::Release(units),
            _ => return Err("payload kind".into()),
        },
        historical: Historical {
            actor: num(v, "actor")?,
            admitted_actor: num(v, "admitted_actor")?,
            capacity: num(v, "capacity")?,
            authority_object: blob(num(v, "authority_object")?),
        },
        closure: blobs(v, "closure")?,
    }))
}
fn head(v: &Value) -> Fallible<Option<Head>> {
    if v.is_null() {
        return Ok(None);
    }
    Ok(Some(Head {
        stream: stream(num(v, "stream")?)?,
        sequence: LedgerSequence::new(num(v, "sequence")?)?,
        record: format!("record-{}", num(v, "record")?).parse()?,
        profile: ReducerProfile::EngineeringV1,
    }))
}

#[test]
fn rust_ledger_matches_lean_append_step_by_step() -> Fallible {
    let doc: Value = serde_json::from_str(CASES)?;
    if doc.get("schema").and_then(Value::as_str) != Some("writfold-ledger-lean-cases/1") {
        return Err("unexpected ledger case schema".into());
    }
    let target = stream(num(&doc, "stream")?)?;
    let (mut steps, mut accepted, mut refused) = (0, 0, 0);
    for scenario in doc
        .get("scenarios")
        .and_then(Value::as_array)
        .ok_or("scenarios")?
    {
        let name = scenario.get("name").and_then(Value::as_str).ok_or("name")?;
        let mut store: Store<Payload, Historical> = Store::default();
        for (index, step) in scenario
            .get("steps")
            .and_then(Value::as_array)
            .ok_or("steps")?
            .iter()
            .enumerate()
        {
            let lean_ok = step
                .get("lean_ok")
                .and_then(Value::as_bool)
                .ok_or("lean_ok")?;
            let expected = head(step.get("expected").ok_or("expected")?)?;
            let available = blobs(step, "available")?;
            let rust_ok = match record(step.get("record").ok_or("record")?)? {
                Some(r) => store
                    .append(&Reservation, &target, expected.as_ref(), r, &available)
                    .is_ok(),
                None => false,
            };
            let lean_head = head(step.get("head_after").ok_or("head_after")?)?;
            let lean_epoch = num(step, "inventory_epoch_after")?;
            if rust_ok != lean_ok
                || store.current_head(&target) != lean_head
                || store.inventory_epoch() != lean_epoch
            {
                return Err(format!("{name} step {index}: lean ok={lean_ok} head={lean_head:?} epoch={lean_epoch}; rust ok={rust_ok} head={:?} epoch={}", store.current_head(&target), store.inventory_epoch()).into());
            }
            steps += 1;
            if lean_ok { accepted += 1 } else { refused += 1 }
        }
    }
    if steps < 30 || accepted < 10 || refused < 14 {
        return Err(format!(
            "corpus too small: {steps} steps, {accepted} accepted, {refused} refused"
        )
        .into());
    }
    Ok(())
}
