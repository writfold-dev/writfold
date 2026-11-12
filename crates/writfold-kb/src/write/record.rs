//! `record ticket|decision|link`: one new file (or decision directory) per
//! record, create-new. An existing ID is a conflict (exit 3). The shapes
//! are the ones the KB already holds, so the prototype's validator keeps
//! accepting them: every asset a ticket or link names must exist, and a
//! decision starts as a `draft` of workflow 3.
use super::json::{Json, object};
use super::{Op, Plan, is_slug, list, one_line, slug};
use crate::error::{KbError, Result};
use crate::index::query::{self, Rec};
use crate::parse::ticket as normalise;
use crate::session::Session;
use crate::time;
use writfold_sources::documents::render_document;

pub const TICKET_STATES: [&str; 6] = [
    "todo",
    "in-progress",
    "investigating",
    "blocked",
    "done",
    "n/a",
];
pub const LINK_TYPES: [&str; 10] = [
    "alerts-on",
    "consumes",
    "defines-contract-for",
    "depends-on",
    "deploys",
    "documents",
    "implements",
    "monitors",
    "owned-by",
    "produces",
];
pub const LINK_STATES: [&str; 5] = ["confirmed", "disputed", "explicit", "inferred", "retired"];
pub const ORIGINS: [&str; 7] = [
    "consolidation",
    "other",
    "review-finding",
    "steward",
    "suggest-skills",
    "unk",
    "user-request",
];

/// A decision's front-matter keys, in the order the KB's decisions use;
/// the ones Writfold does not set are written empty.
const DECISION_KEYS: [&str; 36] = [
    "id",
    "kind",
    "type",
    "title",
    "status",
    "created",
    "updated",
    "origin",
    "proposed_by",
    "requested_by",
    "source_session",
    "project",
    "swept_at",
    "target",
    "references",
    "landed",
    "decided_by",
    "superseded_by",
    "profile",
    "target_revision",
    "acceptor",
    "executor",
    "verifier",
    "accepted_revision",
    "environment",
    "scope",
    "last_actor",
    "regression_note",
    "workflow_version",
    "target_source",
    "schema_version",
    "binding_design_revision",
    "binding_source",
    "binding_target_revision",
    "binding_actor",
    "binding_observed_at",
];

#[derive(Clone, Debug, Default)]
pub struct Ticket {
    pub ticket: String,
    pub title: String,
    pub assets: Vec<String>,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub prs: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Decision {
    pub title: String,
    pub target_source: String,
    pub project: Option<String>,
    pub references: Vec<String>,
    pub context: Option<String>,
    pub evidence: Option<String>,
    pub change: Option<String>,
    pub why: Option<String>,
    pub origin: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Link {
    pub rel: String,
    pub from: String,
    pub to: String,
    pub status: Option<String>,
    pub evidence: Vec<String>,
    pub applicability: Option<String>,
}

fn choose(flag: &str, value: Option<&str>, default: &str, allowed: &[&str]) -> Result<String> {
    let value = value.unwrap_or(default);
    if allowed.contains(&value) {
        Ok(value.to_owned())
    } else {
        Err(KbError::Usage(format!(
            "{flag} must be one of {}",
            allowed.join(", ")
        )))
    }
}

/// Refuse when `id` is already a record.
fn fresh_id(session: &Session<'_>, id: &str) -> Result<()> {
    match query::record(&session.conn, id)? {
        Some(rec) => Err(KbError::Refused(format!(
            "write conflict: {id} already exists at {}",
            rec.path
        ))),
        None => Ok(()),
    }
}

/// Every name must be an asset in the index.
fn assets(session: &Session<'_>, names: &[String]) -> Result<()> {
    for name in names {
        let known: Option<Rec> = query::record(&session.conn, name)?;
        if !is_slug(name) || known.is_none_or(|r| r.kind != "asset") {
            return Err(KbError::Refused(format!(
                "unknown asset {name:?}: record it in inventory/assets/ first"
            )));
        }
    }
    Ok(())
}

fn rendered<S: AsRef<str>>(
    fields: &[(&str, Option<&str>)],
    sections: &[(&str, S)],
    title: &str,
) -> Result<Vec<u8>> {
    let sections: Vec<(&str, &str)> = sections.iter().map(|(n, c)| (*n, c.as_ref())).collect();
    render_document(fields, &sections, title)
        .map(|doc| doc.as_bytes().to_vec())
        .map_err(|error| KbError::Usage(format!("cannot render the record: {error}")))
}

pub fn ticket(session: &Session<'_>, t: &Ticket) -> Result<Plan> {
    let id = normalise(&t.ticket).ok_or_else(|| {
        KbError::Usage(format!(
            "{:?} is not a ticket ID (LETTERS-digits)",
            t.ticket
        ))
    })?;
    let title = one_line("--title", &t.title)?;
    let status = choose("--status", t.status.as_deref(), "todo", &TICKET_STATES)?;
    fresh_id(session, &id)?;
    assets(session, &t.assets)?;
    let date = time::date(session.now);
    let year = &date[..4];
    let lower = id.to_lowercase();
    let path = format!("ktlo/{year}/{lower}-{}.md", slug(&title));
    let (assets, prs) = (list(&t.assets), list(&t.prs));
    let summary = t.summary.as_deref().map(str::trim).unwrap_or_default();
    let fields = [
        ("ticket", Some(id.as_str())),
        ("title", Some(title.as_str())),
        ("assets", Some(assets.as_str())),
        ("status", Some(status.as_str())),
        ("prs", Some(prs.as_str())),
        ("summary", Some(summary)),
        ("created", Some(date.as_str())),
        ("updated", Some(date.as_str())),
        ("promoted_to", Some("")),
    ];
    let sections = [
        ("Symptom", ""),
        ("Evidence", ""),
        ("Change", ""),
        ("Outcome", ""),
    ];
    let bytes = rendered(&fields, &sections, &format!("{id}: {title}"))?;
    Ok(Plan {
        ops: vec![
            Op::AbsentPrefix {
                dir: format!("ktlo/{year}"),
                prefix: format!("{lower}-"),
                what: id.clone(),
            },
            Op::Create { path, bytes },
        ],
        show: Some(id.clone()),
        notes: Vec::new(),
        current: None,
        verb: "record-ticket",
        args: vec![id, title],
    })
}

pub fn decision(session: &Session<'_>, d: &Decision) -> Result<Plan> {
    let title = one_line("--title", &d.title)?;
    let target = one_line("--target-source", &d.target_source)?;
    if matches!(target.as_str(), "unknown" | "unk") {
        return Err(KbError::Usage(
            "--target-source must name the source the decision changes".into(),
        ));
    }
    let origin = choose("--origin", d.origin.as_deref(), "user-request", &ORIGINS)?;
    let date = time::date(session.now);
    let home = format!("{date}-{}", slug(&title));
    let id = format!("decision/{home}");
    fresh_id(session, &format!("proposal/{id}"))?;
    let project = d.project.as_deref().unwrap_or_default();
    let references = list(&d.references);
    let bullets: Vec<String> = d.references.iter().map(|r| format!("- {r}")).collect();
    let given = [
        ("id", id.as_str()),
        ("kind", "decision"),
        ("title", title.as_str()),
        ("status", "draft"),
        ("created", date.as_str()),
        ("updated", date.as_str()),
        ("origin", origin.as_str()),
        ("project", project),
        ("target", "[]"),
        ("references", references.as_str()),
        ("landed", "[]"),
        ("profile", "modern"),
        ("workflow_version", "3"),
        ("target_source", target.as_str()),
        ("schema_version", "1"),
    ];
    let fields: Vec<(&str, Option<&str>)> = DECISION_KEYS
        .iter()
        .map(|key| {
            let value = given.iter().find(|(k, _)| k == key).map_or("", |(_, v)| *v);
            (*key, Some(value))
        })
        .collect();
    let bytes = rendered(&fields, &body(d, &date, &bullets), &title)?;
    let files = vec![
        ("proposal.md".to_owned(), bytes),
        ("gates.json".to_owned(), b"[]\n".to_vec()),
        ("research/".to_owned(), Vec::new()),
        ("evidence/".to_owned(), Vec::new()),
    ];
    Ok(Plan {
        ops: vec![Op::CreateDir {
            path: format!("proposals/decision/{home}"),
            files,
        }],
        show: Some(format!("proposal/{id}")),
        notes: Vec::new(),
        current: None,
        verb: "record-decision",
        args: vec![id, title],
    })
}

/// A new decision's sections, in the order the KB's decisions use.
fn body(d: &Decision, date: &str, bullets: &[String]) -> Vec<(&'static str, String)> {
    let text = |v: &Option<String>| v.as_deref().map(str::trim).unwrap_or_default().to_owned();
    vec![
        ("Context", text(&d.context)),
        ("Evidence", text(&d.evidence)),
        ("Proposed change", text(&d.change)),
        ("Research", String::new()),
        ("Alternatives considered", String::new()),
        ("Acceptance gates", String::new()),
        ("Impact", String::new()),
        ("Landing plan", String::new()),
        ("References", bullets.join("\n")),
        ("Why general / justification", text(&d.why)),
        (
            "Decision log",
            format!("- {date}: draft (recorded with writfold kb record decision)"),
        ),
    ]
}

/// `ev:<home>/<slug>`, or a code locator `<repo>@<7-40 hex>:<path>[:a[-b]]`.
fn evidence_ok(item: &str) -> bool {
    if let Some(rest) = item.strip_prefix("ev:") {
        return rest
            .split_once('/')
            .is_some_and(|(h, s)| !h.is_empty() && is_slug(s));
    }
    let Some((repo, rest)) = item.split_once('@') else {
        return false;
    };
    let Some((rev, path)) = rest.split_once(':') else {
        return false;
    };
    let hex = (7..=40).contains(&rev.len()) && rev.bytes().all(|b| b.is_ascii_hexdigit());
    let path = match path.rsplit_once(':') {
        Some((p, range)) if range.bytes().next().is_some_and(|b| b.is_ascii_digit()) => {
            let (a, b) = range.split_once('-').unwrap_or((range, range));
            match (a.parse::<u32>(), b.parse::<u32>()) {
                (Ok(a), Ok(b)) if a >= 1 && a <= b => p,
                _ => return false,
            }
        }
        _ => path,
    };
    !repo.is_empty() && hex && !path.is_empty()
}

pub fn link(session: &Session<'_>, l: &Link) -> Result<Plan> {
    let rel = choose("type", Some(l.rel.as_str()), "", &LINK_TYPES)?;
    let status = choose("--status", l.status.as_deref(), "explicit", &LINK_STATES)?;
    assets(session, &[l.from.clone(), l.to.clone()])?;
    if let Some(bad) = l.evidence.iter().find(|e| !evidence_ok(e)) {
        return Err(KbError::Usage(format!(
            "evidence {bad:?} is neither ev:<home>/<slug> nor <repo>@<commit>:<path>[:a-b]"
        )));
    }
    let stem = format!("{rel}__{}__{}", l.from, l.to);
    fresh_id(session, &format!("rel:{stem}"))?;
    let applicability = l
        .applicability
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    let bytes = object(&[
        ("schema_version", Json::Int(1)),
        ("id", Json::Text(stem.clone())),
        ("type", Json::Text(rel.clone())),
        ("from", Json::Text(l.from.clone())),
        ("to", Json::Text(l.to.clone())),
        ("status", Json::Text(status)),
        ("evidence", Json::List(l.evidence.clone())),
        ("applicability", Json::Text(applicability.to_owned())),
    ]);
    Ok(Plan {
        ops: vec![Op::Create {
            path: format!("inventory/relationships/{stem}.json"),
            bytes: bytes.into_bytes(),
        }],
        show: Some(format!("rel:{stem}")),
        notes: Vec::new(),
        current: None,
        verb: "record-link",
        args: vec![stem],
    })
}

#[cfg(test)]
mod tests {
    use super::evidence_ok;

    #[test]
    fn evidence_forms() {
        assert!(evidence_ok("ev:harbor/fsync-latency"));
        assert!(evidence_ok("demo@abcdef1:src/a.rs:3-5"));
        assert!(evidence_ok("demo@abcdef1:src/a.rs"));
        assert!(!evidence_ok("demo@main:src/a.rs"));
        assert!(!evidence_ok("demo@abcdef1:src/a.rs:5-3"));
        assert!(!evidence_ok("ev:harbor/Bad"));
    }
}
