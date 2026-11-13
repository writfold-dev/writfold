//! Cover rows: one JSON file per judgement about one checklist item,
//! `runs/<run>/covers/<gate>.<item>-<UTC compact time>-<6 hex>.json`,
//! written create-new through the write protocol by `gate cover` and by
//! nothing else. A row binds the gate's definition digest and the scope
//! content digest (as a statement does), says `supported` (with the
//! references it cites and what each stood on when it was written),
//! `unsupported` or `not-applicable` (with a reason), and carries `seq`:
//! one more than the highest of this gate and item in the run, so the
//! newest row is decided without a clock. It ends with `digest`, the
//! SHA-256 of the rest in canonical form; a row that fails it, or sits
//! under a name that is not its own gate and item, reads as invalid.
use super::super::receipt::canonical;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use writfold_core::assess::Grounding;
use writfold_core::checklist::Outcome;

pub const SCHEMA: &str = "writfold-cover/1";
/// The directory of a run that holds cover rows.
pub const DIR: &str = "covers";

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{}", crate::index::hex(&Sha256::digest(bytes)))
}

/// One cited reference, as recorded when the row was written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cited {
    pub reference: String,
    pub grounding: Grounding,
    pub standing: String,
    /// The KB file the evidence is in, when there is one.
    pub path: Option<String>,
}

/// A cover row's content, written or read back.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    pub gate: String,
    pub item: String,
    pub outcome: Outcome,
    /// Required for `unsupported` and `not-applicable`; absent otherwise.
    pub reason: Option<String>,
    pub note: Option<String>,
    pub by: String,
    pub run: String,
    pub repo: String,
    pub commit: String,
    /// The scope content digest the row is about.
    pub content: String,
    pub definition: String,
    pub evidence: Vec<Cited>,
    pub seq: u64,
    pub recorded: String,
}

/// The order rows are judged in, oldest first: by `seq`, then by file
/// name. `recorded` never decides it.
pub fn order(a: (&str, &Cover), b: (&str, &Cover)) -> std::cmp::Ordering {
    (a.1.seq, a.0).cmp(&(b.1.seq, b.0))
}

/// Whether `name` is a row file of `gate` and `item`:
/// `<gate>.<item>-<8 digits>T<6 digits>Z-<6 hex>.json`.
pub fn filed_as(name: &str, gate: &str, item: &str) -> bool {
    let Some(rest) = name
        .strip_prefix(&format!("{gate}.{item}-"))
        .and_then(|r| r.strip_suffix(".json"))
    else {
        return false;
    };
    let b = rest.as_bytes();
    let digits =
        |r: std::ops::Range<usize>| b.get(r).is_some_and(|s| s.iter().all(u8::is_ascii_digit));
    b.len() == 23
        && digits(0..8)
        && b[8] == b'T'
        && digits(9..15)
        && &b[15..17] == b"Z-"
        && b[17..]
            .iter()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
}

fn cited(c: &Cited) -> Value {
    json!({"reference": c.reference, "grounding": c.grounding.name(),
           "standing": c.standing, "path": c.path})
}

/// The row's bytes: two-space JSON ending in a line break.
pub fn bytes(c: &Cover) -> Vec<u8> {
    let mut value = json!({
        "schema": SCHEMA,
        "gate": c.gate,
        "item": c.item,
        "outcome": c.outcome.name(),
        "reason": c.reason,
        "note": c.note,
        "by": c.by,
        "run": c.run,
        "subject": {"repo": c.repo, "commit": c.commit, "content": c.content},
        "definition": c.definition,
        "evidence": c.evidence.iter().map(cited).collect::<Vec<_>>(),
        "seq": c.seq,
        "recorded": c.recorded,
    });
    let digest = sha(canonical(&value).as_bytes());
    if let Value::Object(map) = &mut value {
        map.insert("digest".into(), Value::String(digest));
    }
    let mut out = serde_json::to_vec_pretty(&value).unwrap_or_default();
    out.push(b'\n');
    out
}

fn s(value: &Value, path: &[&str]) -> Result<String, String> {
    path.iter()
        .try_fold(value, |v, k| v.get(k))
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty() && !t.contains(['\n', '\r']))
        .map(str::to_owned)
        .ok_or_else(|| format!("{} is missing or not a one-line string", path.join(".")))
}

fn maybe(value: &Value, key: &str) -> Result<Option<String>, String> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        _ => s(value, &[key]).map(Some),
    }
}

fn evidence(value: &Value) -> Result<Vec<Cited>, String> {
    let items = value
        .get("evidence")
        .and_then(Value::as_array)
        .ok_or("evidence is not a list")?;
    items
        .iter()
        .map(|c| {
            Ok(Cited {
                reference: s(c, &["reference"])?,
                grounding: Grounding::parse(&s(c, &["grounding"])?).ok_or("unknown grounding")?,
                standing: s(c, &["standing"])?,
                path: maybe(c, "path")?,
            })
        })
        .collect()
}

/// What each outcome must carry: `supported` cites at least one grounded
/// reference and gives no reason; the others give a reason and cite
/// nothing.
fn shaped(c: &Cover) -> Result<(), String> {
    match c.outcome {
        Outcome::Supported if c.evidence.is_empty() => Err("supported, but cites nothing".into()),
        Outcome::Supported if c.reason.is_some() => Err("supported, with a reason".into()),
        Outcome::Supported
            if c.evidence
                .iter()
                .any(|e| e.grounding == Grounding::Ungrounded) =>
        {
            Err("supported by a reference that was not grounded".into())
        }
        Outcome::Supported => Ok(()),
        _ if c.reason.is_none() => Err(format!("{} without a reason", c.outcome.name())),
        _ if !c.evidence.is_empty() => Err(format!("{} cites evidence", c.outcome.name())),
        _ => Ok(()),
    }
}

/// Read one row of `run` filed as `name`, checking its schema, digest,
/// filing and shape. The error says why it is invalid.
pub fn parse(text: &str, run: &str, name: &str) -> Result<Cover, String> {
    let mut value: Value = serde_json::from_str(text).map_err(|e| format!("not JSON ({e})"))?;
    let claimed = value
        .as_object_mut()
        .ok_or("not a JSON object")?
        .remove("digest");
    if s(&value, &["schema"])? != SCHEMA {
        return Err(format!("schema is not {SCHEMA}"));
    }
    if claimed.as_ref().and_then(Value::as_str) != Some(sha(canonical(&value).as_bytes()).as_str())
    {
        return Err(
            "not written by writfold gate cover (its digest does not match its content)".into(),
        );
    }
    let (gate, item) = (s(&value, &["gate"])?, s(&value, &["item"])?);
    if s(&value, &["run"])? != run || !filed_as(name, &gate, &item) {
        return Err("its run, gate or item does not match where it is filed".into());
    }
    let cover = Cover {
        outcome: Outcome::parse(&s(&value, &["outcome"])?).ok_or("unknown outcome")?,
        reason: maybe(&value, "reason")?,
        note: maybe(&value, "note")?,
        by: s(&value, &["by"])?,
        run: run.to_owned(),
        repo: s(&value, &["subject", "repo"])?,
        commit: s(&value, &["subject", "commit"])?,
        content: s(&value, &["subject", "content"])?,
        definition: s(&value, &["definition"])?,
        evidence: evidence(&value)?,
        seq: value
            .get("seq")
            .and_then(Value::as_u64)
            .filter(|n| *n > 0)
            .ok_or("seq is not a count")?,
        recorded: s(&value, &["recorded"])?,
        gate,
        item,
    };
    shaped(&cover)?;
    Ok(cover)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn sample() -> Cover {
        Cover {
            gate: "review".into(),
            item: "errors".into(),
            outcome: Outcome::Supported,
            reason: None,
            note: Some("read every call site".into()),
            by: "agent".into(),
            run: "run-1".into(),
            repo: "demo".into(),
            commit: "abc".into(),
            content: "sha256:00".into(),
            definition: "sha256:11".into(),
            evidence: vec![Cited {
                reference: "gate:unit".into(),
                grounding: Grounding::Grounded,
                standing: "satisfied".into(),
                path: None,
            }],
            seq: 1,
            recorded: "2026-11-11T00:00:00Z".into(),
        }
    }

    const NAME: &str = "review.errors-20261111T000000Z-a1b2c3.json";

    #[test]
    fn rows_round_trip_and_edits_are_invalid() -> Result<(), String> {
        let text = String::from_utf8(bytes(&sample())).map_err(|e| e.to_string())?;
        assert_eq!(parse(&text, "run-1", NAME)?, sample());
        let edited = text.replace("\"supported\"", "\"not-applicable\"");
        assert!(parse(&edited, "run-1", NAME).is_err());
        assert!(parse(&text, "run-2", NAME).is_err());
        for name in [
            "review.tests-20261111T000000Z-a1b2c3.json",
            "review.errors-x-20261111T000000Z-a1b2c3.json",
            "review.errors-20261111T000000Z-a1b2c.json",
            "review-errors-20261111T000000Z-a1b2c3.json",
            "review.errors-20261111T000000Z-A1B2C3.json",
        ] {
            assert!(parse(&text, "run-1", name).is_err(), "{name}");
        }
        Ok(())
    }

    /// A row that does not have its outcome's shape is invalid even with
    /// a correct digest.
    #[test]
    fn each_outcome_has_its_shape() -> Result<(), String> {
        let check = |c: &Cover| {
            let text = String::from_utf8(bytes(c)).map_err(|e| e.to_string())?;
            Ok::<bool, String>(parse(&text, "run-1", NAME).is_ok())
        };
        let mut c = sample();
        c.evidence.clear();
        assert!(!check(&c)?, "supported without evidence");
        let mut c = sample();
        c.evidence[0].grounding = Grounding::Ungrounded;
        assert!(!check(&c)?, "supported by an ungrounded reference");
        let mut c = sample();
        c.outcome = Outcome::NotApplicable;
        assert!(!check(&c)?, "not-applicable with evidence and no reason");
        c.evidence.clear();
        assert!(!check(&c)?, "not-applicable without a reason");
        c.reason = Some("no hot path touched".into());
        assert!(check(&c)?);
        c.outcome = Outcome::Unsupported;
        assert!(check(&c)?);
        Ok(())
    }

    /// Over random sequences of rows, the newest by [`order`] is the one
    /// with the highest `seq` (the file name on ties), whatever each row
    /// says it was recorded at.
    #[test]
    fn the_newest_row_is_decided_by_seq_never_the_clock() {
        let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = move |n: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % n
        };
        for _ in 0..500 {
            let len = 1 + next(6);
            let mut rows: Vec<(String, Cover)> = (0..len)
                .map(|i| {
                    let mut c = sample();
                    c.seq = 1 + next(4);
                    c.recorded = format!("2026-11-{:02}T00:00:00Z", 1 + next(28));
                    (format!("review.errors-2026111{i}T000000Z-a1b2c3.json"), c)
                })
                .collect();
            let max = rows.iter().map(|(_, c)| c.seq).max().unwrap_or(0);
            let want = rows
                .iter()
                .filter(|(_, c)| c.seq == max)
                .map(|(n, _)| n.clone())
                .max();
            rows.sort_by(|a, b| order((&a.0, &a.1), (&b.0, &b.1)));
            assert_eq!(rows.last().map(|(n, _)| n.clone()), want);
            // Reversing the clocks changes nothing.
            for (_, c) in &mut rows {
                c.recorded = c.recorded.replace("2026", "1999");
            }
            rows.reverse();
            rows.sort_by(|a, b| order((&a.0, &a.1), (&b.0, &b.1)));
            assert_eq!(rows.last().map(|(n, _)| n.clone()), want);
        }
    }
}
