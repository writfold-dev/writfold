//! Markdown records: front matter read with the `writfold-sources` codec.
use super::{cite, field_line, first_heading, list, reference, ticket, typed};
use crate::layout::{Kind, stem};
use crate::model::{Facts, Record};
use serde_json::{Map, Value};
use writfold_sources::frontmatter::parse_frontmatter;

/// How a list item becomes a reference token.
#[derive(Clone, Copy)]
enum Target {
    Raw,
    Prefixed(&'static str),
    Ticket,
}

/// Front-matter list fields that are references: field, relation, target.
fn links(kind: Kind) -> &'static [(&'static str, &'static str, Target)] {
    const PROJECT: Target = Target::Prefixed("project:");
    match kind {
        Kind::Proposal => &[
            ("references", "references", Target::Raw),
            ("landed", "landed", Target::Raw),
            ("target", "target", Target::Raw),
            ("project", "project", PROJECT),
            (
                "superseded_by",
                "superseded-by",
                Target::Prefixed("proposal/"),
            ),
        ],
        Kind::Ticket => &[
            ("assets", "concerns", Target::Raw),
            ("prs", "pr", Target::Raw),
            ("promoted_to", "promoted-to", PROJECT),
        ],
        Kind::Investigation => &[
            ("tickets", "ticket", Target::Ticket),
            ("alerts", "alert", Target::Raw),
            ("assets", "concerns", Target::Raw),
            ("signatures", "signature", Target::Prefixed("sig:")),
            ("spawned", "spawned", Target::Raw),
            ("follow_ups", "follow-up", Target::Raw),
            ("promoted_to", "promoted-to", PROJECT),
        ],
        Kind::Signature => &[
            ("assets", "concerns", Target::Raw),
            ("fix", "fix", Target::Raw),
        ],
        Kind::Runbook => &[
            ("assets", "concerns", Target::Raw),
            ("alerts", "alert", Target::Raw),
        ],
        _ => &[],
    }
}

/// The record identity of a document and its aliases, or why it has none.
fn identity(
    path: &str,
    kind: Kind,
    get: &dyn Fn(&str) -> Option<String>,
) -> Result<(String, Vec<String>), String> {
    let parts: Vec<&str> = path.split('/').collect();
    let name = stem(path).to_owned();
    Ok(match kind {
        Kind::Proposal => {
            let id = get("id").ok_or("decision has no id")?;
            (format!("proposal/{id}"), vec![id])
        }
        Kind::Ticket => {
            let raw = get("ticket").ok_or("ticket record has no ticket")?;
            (
                ticket(&raw).ok_or(format!("unrecognised ticket {raw:?}"))?,
                Vec::new(),
            )
        }
        Kind::Investigation => {
            let id = get("id").unwrap_or_else(|| parts[1..parts.len() - 1].join("/"));
            (format!("inv:{id}"), vec![id])
        }
        Kind::Signature => (format!("sig:{name}"), vec![name]),
        Kind::Runbook => (format!("runbook:{}/{name}", parts[1]), Vec::new()),
        Kind::Dossier => (format!("dossier:{name}"), Vec::new()),
        Kind::Correction => (format!("correction:{name}"), Vec::new()),
        Kind::Project => (format!("project:{}", parts[0]), vec![parts[0].to_owned()]),
        Kind::Evidence => (format!("ev:{}/{name}", parts[0]), Vec::new()),
        _ => return Err(String::new()),
    })
}

pub fn parse(path: &str, kind: Kind, text: &str) -> Result<Facts, String> {
    let entries: Vec<(String, String)> = match parse_frontmatter(text) {
        Ok(block) => block
            .entries()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect(),
        Err(error) if kind.needs_identity() => return Err(error.message().to_owned()),
        Err(_) => Vec::new(),
    };
    let get = |key: &str| {
        entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.trim().to_owned())
            .filter(|v| !v.is_empty() && v != "null")
    };
    let mut facts = Facts::default();
    if !kind.needs_identity() {
        return Ok(facts);
    }
    let (id, aliases) = identity(path, kind, &get)?;
    facts.aliases = aliases.into_iter().map(|alias| (alias, "id")).collect();
    for (field, rel, target) in links(kind) {
        let line = field_line(text, field);
        for item in get(field).map(|v| list(&v)).unwrap_or_default() {
            if *field == "fix"
                && let Some(citation) = cite::locator(&item, line)
            {
                facts.citations.push(citation);
                continue;
            }
            let token = match target {
                Target::Raw => Some(item),
                Target::Prefixed(prefix) => Some(typed(prefix, &item)),
                Target::Ticket => ticket(&item).or(Some(item)),
            };
            facts
                .references
                .extend(token.map(|t| reference(&id, rel, t, line)));
        }
    }
    if kind == Kind::Dossier {
        facts
            .references
            .push(reference(&id, "dossier-of", stem(path).to_owned(), 1));
    }
    let status_key = if kind == Kind::Project {
        "phase"
    } else {
        "status"
    };
    let project = match kind {
        Kind::Project | Kind::Evidence => path.split('/').next().map(str::to_owned),
        Kind::Ticket | Kind::Investigation => get("promoted_to"),
        _ => get("project"),
    };
    let mut extra = Map::new();
    for (key, value) in &entries {
        extra.insert(key.clone(), Value::String(value.clone()));
    }
    facts.record = Some(Record {
        id,
        kind: Some(kind),
        line: field_line(text, status_key),
        title: get("title").or_else(|| first_heading(text)),
        status: get(status_key),
        project,
        created: get("created"),
        updated: get("updated"),
        summary: get("summary").or_else(|| get("impact_summary")),
        extra: Value::Object(extra).to_string(),
    });
    Ok(facts)
}
