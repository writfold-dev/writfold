//! Asset, relationship and run-manifest JSON, and other JSON documents.
use super::{cite, reference, typed};
use crate::layout::{Kind, stem};
use crate::model::{Facts, Record, Reference};
use serde_json::{Map, Value};

fn object(text: &str) -> Result<Map<String, Value>, String> {
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err("JSON is not an object".into()),
        Err(error) => Err(format!("invalid JSON: {error}")),
    }
}

fn text(map: &Map<String, Value>, key: &str) -> Option<String> {
    match map.get(key)? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_owned()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn strings(map: &Map<String, Value>, key: &str) -> Vec<String> {
    match map.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_owned()),
                _ => None,
            })
            .collect(),
        Some(Value::String(s)) if !s.trim().is_empty() => vec![s.trim().to_owned()],
        _ => Vec::new(),
    }
}

/// The 1-based line of the first `"key"` in the raw text, or 1.
pub(crate) fn line_of(raw: &str, key: &str) -> u32 {
    let needle = format!("\"{key}\"");
    raw.lines()
        .position(|line| line.contains(&needle))
        .and_then(|index| u32::try_from(index + 1).ok())
        .unwrap_or(1)
}

pub fn parse(path: &str, kind: Kind, raw: &str) -> Result<Facts, String> {
    let map = object(raw)?;
    match kind {
        Kind::Asset => asset(raw, map),
        Kind::Relationship => relationship(path, raw, map),
        _ => run(path, raw, map),
    }
}

/// Any other JSON file: scanned as a document; it need not be an object.
pub fn document(_path: &str, raw: &str) -> Result<Facts, String> {
    serde_json::from_str::<Value>(raw).map_err(|error| format!("invalid JSON: {error}"))?;
    Ok(Facts::default())
}

fn asset(raw: &str, map: Map<String, Value>) -> Result<Facts, String> {
    let id = text(&map, "id").ok_or("asset has no id")?;
    let mut facts = Facts::default();
    let line = line_of(raw, "lifecycle");
    for project in strings(&map, "projects") {
        let token = typed("project:", &project);
        facts
            .references
            .push(reference(&id, "project", token, line_of(raw, "projects")));
    }
    for alias in strings(&map, "aliases") {
        facts.aliases.push((alias, "alias"));
    }
    if let Some(locator) = text(&map, "locator") {
        facts.aliases.push((locator, "locator"));
    }
    facts.record = Some(Record {
        kind: Some(Kind::Asset),
        line,
        title: text(&map, "name"),
        status: text(&map, "lifecycle"),
        project: strings(&map, "projects").into_iter().next(),
        created: text(&map, "created"),
        updated: text(&map, "verified_at"),
        summary: text(&map, "notes"),
        extra: Value::Object(map).to_string(),
        id,
    });
    Ok(facts)
}

fn relationship(path: &str, raw: &str, map: Map<String, Value>) -> Result<Facts, String> {
    let (Some(rel), Some(from), Some(to)) =
        (text(&map, "type"), text(&map, "from"), text(&map, "to"))
    else {
        return Err("relationship needs type, from and to".into());
    };
    let id = format!("rel:{}", stem(path));
    let status = text(&map, "status").unwrap_or_else(|| "explicit".into());
    let mut facts = Facts::default();
    let evidence_line = line_of(raw, "evidence");
    for item in strings(&map, "evidence") {
        if let Some(citation) = cite::locator(&item, evidence_line) {
            facts.citations.push(citation);
        }
    }
    facts.references.push(Reference {
        line: line_of(raw, "to"),
        src: from.clone(),
        rel: rel.clone(),
        token: to.clone(),
        origin: "authored",
        status: status.clone(),
        cite: None,
    });
    facts.record = Some(Record {
        id,
        kind: Some(Kind::Relationship),
        line: line_of(raw, "type"),
        title: Some(format!("{from} {rel} {to}")),
        status: Some(status),
        extra: Value::Object(map).to_string(),
        ..Record::default()
    });
    Ok(facts)
}

fn run(path: &str, raw: &str, map: Map<String, Value>) -> Result<Facts, String> {
    let directory = path.split('/').nth(1).unwrap_or_default();
    let id = text(&map, "run_id").unwrap_or_else(|| directory.to_owned());
    let mut facts = Facts::default();
    let project = text(&map, "project");
    if let Some(project) = &project {
        let token = typed("project:", project);
        facts
            .references
            .push(reference(&id, "project", token, line_of(raw, "project")));
    }
    facts.record = Some(Record {
        kind: Some(Kind::Run),
        line: line_of(raw, "state"),
        title: text(&map, "task"),
        status: text(&map, "state"),
        project,
        created: text(&map, "created"),
        extra: Value::Object(map).to_string(),
        id,
        ..Record::default()
    });
    Ok(facts)
}
