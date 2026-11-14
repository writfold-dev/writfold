//! Observations: what an agent saw when it looked at an outside source
//! during a run, and how the look went, recorded by `writfold kb observe`
//! ([`crate::write::observe`]) as one new JSON file,
//! `runs/<run>/observed/<UTC compact time>-<6 hex>.json`, schema
//! `writfold-observation/1`, written create-new and never changed.
//!
//! An `ok` or `partial` observation embeds exactly one ordinary citation
//! token (`cite`), so its freshness is the token's: the kind declarations
//! in `.writfold/sources/` (lifetime, immutable, checked) apply as to any
//! other citation. The other statuses (`no-data`, `not-found`, `timeout`,
//! `denied`, `error`) embed nothing: they record that someone looked and
//! could not tell. An observation may name the checklist item it was made
//! for (`for`, `<gate>.<item>`); `gate status` shows the looks under that
//! item. Only an `ok` observation whose citation is fresh grounds evidence
//! ([`writfold_core::observe::grounds`], read by [`crate::gate::ground`]).
//!
//! The directory is `observed/`, not `observations/`: a run's
//! `observations/` belongs to another reader and is never written.
use crate::emit::More;
use serde_json::{Value, json};
use std::path::Path;
use writfold_core::observe::{Status, embeds};

pub const SCHEMA: &str = "writfold-observation/1";
/// The directory of a run that holds observations.
pub const DIR: &str = "observed";

/// One observation, written or read back.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observation {
    pub run: String,
    /// The source's kind, as citation tokens name kinds.
    pub source: String,
    pub locator: String,
    pub status: Status,
    pub revision: Option<String>,
    /// `(gate, item)` the look was made for.
    pub item: Option<(String, String)>,
    pub note: Option<String>,
    /// The embedded citation token (`ok` and `partial` only).
    pub cite: Option<String>,
    pub observed: String,
}

/// The file's bytes: two-space JSON ending in a line break.
pub fn bytes(o: &Observation) -> Vec<u8> {
    let value = json!({
        "schema": SCHEMA,
        "run": o.run,
        "source": o.source,
        "locator": o.locator,
        "status": o.status.name(),
        "revision": o.revision,
        "for": o.item.as_ref().map(|(g, i)| format!("{g}.{i}")),
        "note": o.note,
        "cite": o.cite,
        "observed": o.observed,
    });
    let mut out = serde_json::to_vec_pretty(&value).unwrap_or_default();
    out.push(b'\n');
    out
}

/// `<gate>.<item>`, both slugs.
pub fn item(text: &str) -> Option<(String, String)> {
    let (gate, item) = text.split_once('.')?;
    (crate::write::is_slug(gate) && crate::write::is_slug(item))
        .then(|| (gate.to_owned(), item.to_owned()))
}

fn s(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty() && !t.contains(['\n', '\r']))
        .map(str::to_owned)
        .ok_or_else(|| format!("{key} is missing or not a one-line string"))
}

fn maybe(value: &Value, key: &str) -> Result<Option<String>, String> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        _ => s(value, key).map(Some),
    }
}

/// Read an observation back, refusing anything `bytes` would not write.
pub fn parse(text: &str) -> Result<Observation, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| format!("not JSON ({e})"))?;
    if value.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return Err(format!("schema is not {SCHEMA}"));
    }
    let status = s(&value, "status")?;
    let status = Status::parse(&status).ok_or_else(|| format!("unknown status {status}"))?;
    let item = match maybe(&value, "for")? {
        None => None,
        Some(text) => Some(item(&text).ok_or_else(|| format!("for {text:?} is not gate.item"))?),
    };
    let cite = maybe(&value, "cite")?;
    if cite.is_some() != embeds(status) {
        return Err(format!(
            "a {} observation {} a citation",
            status.name(),
            if embeds(status) {
                "needs"
            } else {
                "carries no"
            }
        ));
    }
    Ok(Observation {
        run: s(&value, "run")?,
        source: s(&value, "source")?,
        locator: s(&value, "locator")?,
        status,
        revision: maybe(&value, "revision")?,
        item,
        note: maybe(&value, "note")?,
        cite,
        observed: s(&value, "observed")?,
    })
}

/// One observation file of a run.
pub struct Filed {
    /// KB-relative path.
    pub path: String,
    pub parsed: Result<Observation, String>,
}

/// Every observation file of the run whose directory is `run_dir`,
/// oldest first (the names start with the UTC time).
pub fn filed(root: &Path, run_dir: &str) -> Vec<Filed> {
    let Ok(entries) = std::fs::read_dir(root.join(run_dir).join(DIR)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json") && !n.starts_with('.'))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let path = format!("{run_dir}/{DIR}/{name}");
            let parsed = std::fs::read_to_string(root.join(&path))
                .map_err(|e| format!("unreadable ({e})"))
                .and_then(|text| parse(&text));
            Filed { path, parsed }
        })
        .collect()
}

/// `n (a, b)`: how many, and each status once in first-seen order.
fn tally(statuses: &[Status]) -> String {
    let mut names: Vec<&str> = Vec::new();
    for s in statuses {
        if !names.contains(&s.name()) {
            names.push(s.name());
        }
    }
    format!("{} ({})", statuses.len(), names.join(", "))
}

/// The `looked:` line under a checklist item: the observations made for
/// `gate`.`item`, and the newest one's path.
pub fn looked(filed: &[Filed], gate: &str, item: &str) -> Option<More> {
    let mine: Vec<(&str, Status)> = filed
        .iter()
        .filter_map(|f| f.parsed.as_ref().ok().map(|o| (f, o)))
        .filter(|(_, o)| o.item.as_ref().is_some_and(|(g, i)| g == gate && i == item))
        .map(|(f, o)| (f.path.as_str(), o.status))
        .collect();
    let (newest, _) = mine.last()?;
    let statuses: Vec<Status> = mine.iter().map(|(_, s)| *s).collect();
    Some(More {
        status: None,
        label: "looked",
        text: tally(&statuses),
        path: Some((*newest).to_owned()),
        line: Some(1),
        flags: Vec::new(),
    })
}

/// `resume`'s note: how many observations the run holds, by status.
pub fn summary(root: &Path, run_dir: &str) -> Option<String> {
    let all = filed(root, run_dir);
    if all.is_empty() {
        return None;
    }
    let statuses: Vec<Status> = all
        .iter()
        .filter_map(|f| f.parsed.as_ref().ok().map(|o| o.status))
        .collect();
    let invalid = all.len() - statuses.len();
    let mut text = format!("observations: {} in {run_dir}/{DIR}/", tally(&statuses));
    if invalid > 0 {
        text.push_str(&format!("; {invalid} unreadable"));
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::{Observation, bytes, item, parse};
    use writfold_core::observe::{ALL, Status, embeds};

    fn one(status: Status) -> Observation {
        Observation {
            run: "run-x".into(),
            source: "dashboard".into(),
            locator: "latency p99".into(),
            status,
            revision: Some("r7".into()),
            item: Some(("review".into(), "perf".into())),
            note: None,
            cite: embeds(status)
                .then(|| "⟦cite kind=dashboard locator=x seen=2026-11-01T00:00Z⟧".into()),
            observed: "2026-11-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn round_trips_and_refusals() -> Result<(), String> {
        for status in ALL {
            let o = one(status);
            let text = String::from_utf8(bytes(&o)).map_err(|e| e.to_string())?;
            assert_eq!(parse(&text)?, o);
            let mut wrong = o.clone();
            wrong.cite = match wrong.cite {
                Some(_) => None,
                None => Some("⟦cite kind=x seen=2026-11-01T00:00Z⟧".into()),
            };
            let text = String::from_utf8(bytes(&wrong)).map_err(|e| e.to_string())?;
            assert!(parse(&text).is_err(), "{status:?}");
        }
        assert!(parse("{}").is_err());
        assert_eq!(item("review.perf"), Some(("review".into(), "perf".into())));
        assert_eq!(item("review"), None);
        assert_eq!(item("Review.perf"), None);
        Ok(())
    }
}
