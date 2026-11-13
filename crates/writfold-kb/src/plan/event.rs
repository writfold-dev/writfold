//! The run graph's event log: one JSON file per event under
//! `runs/<run>/plan/`, written create-new through the write protocol and
//! never changed. A `splice-` file adds a template's steps under a parent
//! (the run's root for `plan set`); a `done-` file records that a step's
//! gates were all met, naming what met each; a `skip-` file closes a step
//! without calling it done. Each file name is derived from what the event
//! is about, so a second event about the same thing is a write conflict,
//! and each file ends with `digest`, the SHA-256 of its canonical form
//! (as receipts do): a hand-edited event reads as invalid.
//!
//! Every event also records the run's records at that moment
//! ([`Baseline`]): the window of a step's run-record checks runs from the
//! event that began the step to the one that closed it.
use crate::gate::receipt::canonical;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const SCHEMA: &str = "writfold-plan-event/1";
/// The directory of a run that holds its plan events.
pub const DIR: &str = "plan";
/// Template snapshots, by digest, under [`DIR`].
pub const TEMPLATES: &str = "templates";

/// The run's own records at one moment, as a step window's edge.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Baseline {
    /// Checkpoint entries in `checkpoint.md`.
    pub entries: u64,
    /// Retrieval receipt file names, sorted.
    pub retrievals: Vec<String>,
    /// `sha256:<hex>` over the KB's files outside `runs/`.
    pub kb: String,
    /// Proposals recorded for this run.
    pub proposals: u64,
}

/// One template a splice's template includes (`use=`), directly or not.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Include {
    pub name: String,
    pub digest: String,
    pub source: String,
}

/// A splice: `template`'s steps added under `parent` (`""` is the root).
/// `ordinal` is the splice's place among the splices under `parent`.
#[derive(Clone, Debug)]
pub struct Splice {
    pub parent: String,
    pub template: String,
    pub digest: String,
    pub ordinal: u64,
    /// An intended repeat (`plan use --again`): written as `"again": true`
    /// only when set, so other splices keep their bytes; a merge-made
    /// repeat lacks it and is flagged `!repeated` ([`super::disorder`]).
    pub again: bool,
    /// Where the template was read from (`.writfold/plans/<name>.toml`,
    /// `<repo>:.writfold/plans/…`, or `--from`).
    pub source: String,
    /// The closure of `use=` includes, each snapshotted; empty when the
    /// template includes none (and then not written, so events written
    /// before includes existed keep their bytes).
    pub includes: Vec<Include>,
    pub records: Baseline,
    pub recorded: String,
}

/// What met one gate when a step was advanced.
#[derive(Clone, Debug)]
pub struct Met {
    pub gate: String,
    /// `satisfied`, `attested` or `waived`.
    pub standing: String,
    /// The receipt or statement file (KB-relative).
    pub evidence: Option<String>,
    pub definition: String,
    /// The scope or run-record digest the evidence is about.
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct Done {
    pub node: String,
    pub met: Vec<Met>,
    pub records: Baseline,
    pub recorded: String,
}

#[derive(Clone, Debug)]
pub struct Skip {
    pub node: String,
    pub reason: String,
    pub by: String,
    pub records: Baseline,
    pub recorded: String,
}

#[derive(Clone, Debug)]
pub enum Event {
    Splice(Splice),
    Done(Done),
    Skip(Skip),
}

/// The node prefix a splice creates: `<parent>/<template>#<n>`.
pub fn prefix(parent: &str, template: &str, ordinal: u64) -> String {
    match parent {
        "" => format!("{template}#{ordinal}"),
        _ => format!("{parent}/{template}#{ordinal}"),
    }
}

/// A node path as a file-name part: `/` as `~` and `#` as `.` (step IDs
/// and template names are slugs, so this is reversible).
pub fn encode(path: &str) -> String {
    path.replace('/', "~").replace('#', ".")
}

/// The file name an event must have.
pub fn name(event: &Event) -> String {
    match event {
        Event::Splice(s) => format!(
            "splice-{}.json",
            encode(&prefix(&s.parent, &s.template, s.ordinal))
        ),
        Event::Done(d) => format!("done-{}.json", encode(&d.node)),
        Event::Skip(s) => format!("skip-{}.json", encode(&s.node)),
    }
}

fn baseline_json(b: &Baseline) -> Value {
    json!({"entries": b.entries, "retrievals": b.retrievals, "kb": b.kb, "proposals": b.proposals})
}

fn body(event: &Event) -> Value {
    match event {
        Event::Splice(s) => {
            let mut value = json!({
                "event": "splice", "parent": s.parent, "ordinal": s.ordinal,
                "template": {"name": s.template, "digest": s.digest, "source": s.source},
                "records": baseline_json(&s.records), "recorded": s.recorded,
            });
            if let (false, Value::Object(map)) = (s.includes.is_empty(), &mut value) {
                let includes = s
                    .includes
                    .iter()
                    .map(|i| json!({"name": i.name, "digest": i.digest, "source": i.source}));
                map.insert("includes".into(), Value::Array(includes.collect()));
            }
            if let (true, Value::Object(map)) = (s.again, &mut value) {
                map.insert("again".into(), Value::Bool(true));
            }
            value
        }
        Event::Done(d) => {
            let met: Vec<Value> = d
                .met
                .iter()
                .map(|m| {
                    json!({
                        "gate": m.gate, "standing": m.standing, "evidence": m.evidence,
                        "definition": m.definition, "content": m.content,
                    })
                })
                .collect();
            json!({"event": "done", "node": d.node, "met": met,
                   "records": baseline_json(&d.records), "recorded": d.recorded})
        }
        Event::Skip(s) => json!({
            "event": "skip", "node": s.node, "reason": s.reason, "by": s.by,
            "records": baseline_json(&s.records), "recorded": s.recorded,
        }),
    }
}

/// The event's bytes: two-space JSON ending in a line break.
pub fn bytes(run: &str, event: &Event) -> Vec<u8> {
    let mut value = body(event);
    if let Value::Object(map) = &mut value {
        map.insert("schema".into(), Value::String(SCHEMA.into()));
        map.insert("run".into(), Value::String(run.into()));
    }
    let digest = format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(canonical(&value).as_bytes()))
    );
    if let Value::Object(map) = &mut value {
        map.insert("digest".into(), Value::String(digest));
    }
    let mut out = serde_json::to_vec_pretty(&value).unwrap_or_default();
    out.push(b'\n');
    out
}

type Parsed<T> = std::result::Result<T, String>;

fn text(v: &Value, key: &str) -> Parsed<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|t| !t.contains(['\n', '\r']))
        .map(str::to_owned)
        .ok_or_else(|| format!("{key} is missing or not a one-line string"))
}

fn count(v: &Value, key: &str) -> Parsed<u64> {
    v.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} is missing or not a count"))
}

fn names(v: &Value, key: &str) -> Parsed<Vec<String>> {
    let wrong = || format!("{key} is not a list of strings");
    match v.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|i| i.as_str().map(str::to_owned).ok_or_else(wrong))
            .collect(),
        _ => Err(wrong()),
    }
}

fn baseline(v: &Value) -> Parsed<Baseline> {
    let r = v.get("records").ok_or("records is missing")?;
    Ok(Baseline {
        entries: count(r, "entries")?,
        retrievals: names(r, "retrievals")?,
        kb: text(r, "kb")?,
        proposals: count(r, "proposals")?,
    })
}

fn met(v: &Value) -> Parsed<Met> {
    Ok(Met {
        gate: text(v, "gate")?,
        standing: text(v, "standing")?,
        evidence: match v.get("evidence") {
            Some(Value::Null) | None => None,
            Some(_) => Some(text(v, "evidence")?),
        },
        definition: text(v, "definition")?,
        content: text(v, "content")?,
    })
}

fn includes(v: &Value) -> Parsed<Vec<Include>> {
    let include = |i: &Value| {
        Ok(Include {
            name: text(i, "name")?,
            digest: text(i, "digest")?,
            source: text(i, "source")?,
        })
    };
    match v.get("includes") {
        None => Ok(Vec::new()),
        Some(Value::Array(items)) if !items.is_empty() => items.iter().map(include).collect(),
        Some(_) => Err("includes is not a non-empty list".into()),
    }
}

fn event(v: &Value) -> Parsed<Event> {
    let records = baseline(v)?;
    let recorded = text(v, "recorded")?;
    Ok(match text(v, "event")?.as_str() {
        "splice" => {
            let t = v.get("template").ok_or("template is missing")?;
            Event::Splice(Splice {
                parent: text(v, "parent")?,
                template: text(t, "name")?,
                digest: text(t, "digest")?,
                ordinal: count(v, "ordinal")?,
                again: match v.get("again") {
                    None => false,
                    Some(Value::Bool(true)) => true,
                    Some(_) => return Err("again is not true".into()),
                },
                source: text(t, "source")?,
                includes: includes(v)?,
                records,
                recorded,
            })
        }
        "done" => Event::Done(Done {
            node: text(v, "node")?,
            met: match v.get("met") {
                Some(Value::Array(items)) => items.iter().map(met).collect::<Parsed<_>>()?,
                _ => return Err("met is not a list".into()),
            },
            records,
            recorded,
        }),
        "skip" => Event::Skip(Skip {
            node: text(v, "node")?,
            reason: text(v, "reason")?,
            by: text(v, "by")?,
            records,
            recorded,
        }),
        other => return Err(format!("event {other:?} is not splice, done or skip")),
    })
}

/// Read one event of `run` filed as `file`, checking its schema, digest,
/// run and that its file name is the one its content implies.
pub fn parse(text_in: &str, run: &str, file: &str) -> Parsed<Event> {
    let mut value: Value = serde_json::from_str(text_in).map_err(|e| format!("not JSON ({e})"))?;
    let claimed = value.as_object_mut().and_then(|m| m.remove("digest"));
    if value.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return Err(format!("schema is not {SCHEMA}"));
    }
    let digest = format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(canonical(&value).as_bytes()))
    );
    if claimed.as_ref().and_then(Value::as_str) != Some(digest.as_str()) {
        return Err(
            "not written by writfold kb plan (its digest does not match its content)".into(),
        );
    }
    if text(&value, "run")? != run {
        return Err("its run does not match where it is filed".into());
    }
    let parsed = event(&value)?;
    if name(&parsed) != file {
        return Err(format!(
            "its content names {}, not this file",
            name(&parsed)
        ));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::{Baseline, Done, Event, Include, Met, Splice, bytes, name, parse};

    #[test]
    fn events_round_trip_and_edits_are_invalid() -> Result<(), String> {
        let splice = Event::Splice(Splice {
            parent: String::new(),
            template: "change".into(),
            digest: "sha256:aa".into(),
            ordinal: 1,
            again: false,
            source: ".writfold/plans/change.toml".into(),
            includes: Vec::new(),
            records: Baseline::default(),
            recorded: "2026-11-12T00:00:00Z".into(),
        });
        assert_eq!(name(&splice), "splice-change.1.json");
        let text = String::from_utf8(bytes("run-1", &splice)).map_err(|e| e.to_string())?;
        assert!(!text.contains("includes"), "no includes, no key: {text}");
        assert!(!text.contains("again"), "no repeat, no key: {text}");
        let mut nested = match &splice {
            Event::Splice(s) => s.clone(),
            _ => return Err("a splice".into()),
        };
        nested.parent = "change#1/review".into();
        nested.ordinal = 2;
        nested.again = true;
        nested.includes = vec![Include {
            name: "read".into(),
            digest: "sha256:bb".into(),
            source: ".writfold/plans/read.toml".into(),
        }];
        let nested = Event::Splice(nested);
        assert_eq!(name(&nested), "splice-change.1~review~change.2.json");
        let back = String::from_utf8(bytes("run-1", &nested)).map_err(|e| e.to_string())?;
        assert!(
            matches!(parse(&back, "run-1", &name(&nested))?, Event::Splice(s) if s.includes.len() == 1 && s.again && s.parent == "change#1/review")
        );
        assert!(
            matches!(parse(&text, "run-1", "splice-change.1.json")?, Event::Splice(s) if s.template == "change")
        );
        assert!(parse(&text, "run-2", "splice-change.1.json").is_err());
        assert!(parse(&text, "run-1", "splice-other.1.json").is_err());
        assert!(
            parse(
                &text.replace("change", "chance"),
                "run-1",
                "splice-chance.1.json"
            )
            .is_err()
        );
        let done = Event::Done(Done {
            node: "change#1/implement".into(),
            met: vec![Met {
                gate: "unit".into(),
                standing: "satisfied".into(),
                evidence: Some("runs/run-1/gates/unit-x.json".into()),
                definition: "sha256:11".into(),
                content: "sha256:22".into(),
            }],
            records: Baseline::default(),
            recorded: "2026-11-12T00:00:00Z".into(),
        });
        assert_eq!(name(&done), "done-change.1~implement.json");
        let text = String::from_utf8(bytes("run-1", &done)).map_err(|e| e.to_string())?;
        assert!(matches!(parse(&text, "run-1", &name(&done))?, Event::Done(d) if d.met.len() == 1));
        Ok(())
    }
}
