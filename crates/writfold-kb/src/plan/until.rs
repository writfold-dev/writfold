//! What a loop round began on (#76, #81): `plan again` records, in its
//! splice event's optional `until`, each of the loop step's gates with its
//! standing and evidence at that moment, so a wasted round is visible.
//! Events without it (first rounds, older binaries) keep their bytes;
//! older binaries ignore it. Parsing and rendering only; tested.
use serde_json::{Map, Value, json};

/// One gate as it stood when a round began.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Began {
    pub gate: String,
    pub standing: String,
    pub evidence: Option<String>,
}

/// Add `until` to an event body, only when there is something to record.
pub fn write(map: &mut Map<String, Value>, until: &[Began]) {
    if until.is_empty() {
        return;
    }
    let items = until.iter().map(|b| {
        let mut item = Map::new();
        item.insert("gate".into(), json!(b.gate));
        item.insert("standing".into(), json!(b.standing));
        if let Some(e) = &b.evidence {
            item.insert("evidence".into(), json!(e));
        }
        Value::Object(item)
    });
    map.insert("until".into(), Value::Array(items.collect()));
}

fn one(v: &Value) -> Result<Began, String> {
    let text = |key: &str| match v.get(key) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        _ => Err(format!("until.{key} is not a non-empty string")),
    };
    Ok(Began {
        gate: text("gate")?,
        standing: text("standing")?,
        evidence: match v.get("evidence") {
            None => None,
            Some(_) => Some(text("evidence")?),
        },
    })
}

/// The `until` of an event body: empty when absent; malformed is an error.
pub fn parse(v: &Value) -> Result<Vec<Began>, String> {
    match v.get("until") {
        None => Ok(Vec::new()),
        Some(Value::Array(items)) if !items.is_empty() => items.iter().map(one).collect(),
        Some(_) => Err("until is not a non-empty list".into()),
    }
}

/// `fixed failed (runs/…/receipt.json), lint open`.
pub fn said(until: &[Began]) -> String {
    let each = until.iter().map(|b| match &b.evidence {
        Some(e) => format!("{} {} ({e})", b.gate, b.standing),
        None => format!("{} {}", b.gate, b.standing),
    });
    each.collect::<Vec<_>>().join(", ")
}

/// The `until` key of a `plan status --json` row.
pub fn json(until: &[Began]) -> Value {
    let mut map = Map::new();
    write(&mut map, until);
    map.remove("until").unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn until_round_trips_and_is_absent_when_empty() {
        let until = vec![
            Began {
                gate: "fixed".into(),
                standing: "failed".into(),
                evidence: Some("runs/r/gates/fixed.json".into()),
            },
            Began {
                gate: "lint".into(),
                standing: "open".into(),
                evidence: None,
            },
        ];
        let mut map = Map::new();
        write(&mut map, &until);
        assert_eq!(parse(&Value::Object(map.clone())), Ok(until.clone()));
        assert_eq!(
            said(&until),
            "fixed failed (runs/r/gates/fixed.json), lint open"
        );
        let mut empty = Map::new();
        write(&mut empty, &[]);
        assert!(empty.is_empty());
        assert_eq!(parse(&json!({})), Ok(Vec::new()));
        assert!(parse(&json!({"until": []})).is_err());
        assert!(parse(&json!({"until": [{"gate": "x"}]})).is_err());
        assert!(
            parse(&json!({"until": [{"gate": "x", "standing": "open", "evidence": 3}]})).is_err()
        );
    }
}
