//! `record investigation`: a new investigation, `open`, in
//! `investigations/<year>/<date>-<slug>/` with its `README.md` and an empty
//! `evidence/`, created as one directory. Its body holds the sections
//! `status --set concluded` later checks, empty, so concluding needs no
//! restructuring. Tickets and signatures it names need not exist yet (an
//! investigation often comes first); assets must.
use super::record::{assets, fresh_id, rendered};
use super::{Op, Plan, is_slug, list, one_line, slug};
use crate::error::{KbError, Result};
use crate::parse::ticket as normalise;
use crate::session::Session;
use crate::time;

#[derive(Clone, Debug, Default)]
pub struct Investigation {
    pub title: String,
    pub trigger: Option<String>,
    pub tickets: Vec<String>,
    pub assets: Vec<String>,
    pub alerts: Vec<String>,
    pub signatures: Vec<String>,
    pub summary: Option<String>,
    pub question: Option<String>,
}

/// The body sections, in the order `concluded` reads them.
pub const SECTIONS: [&str; 5] = [
    "Question",
    "Hypotheses",
    "Checked against",
    "Scope and limits",
    "Conclusion",
];

/// Each `--tickets` entry as a ticket ID, or a usage error.
fn tickets(raw: &[String]) -> Result<Vec<String>> {
    raw.iter()
        .map(|t| {
            normalise(t).ok_or_else(|| {
                KbError::Usage(format!(
                    "--tickets: {t:?} is not a ticket ID (LETTERS-digits)"
                ))
            })
        })
        .collect()
}

pub fn plan(session: &Session<'_>, i: &Investigation) -> Result<Plan> {
    let title = one_line("--title", &i.title)?;
    let trigger = match &i.trigger {
        Some(t) => one_line("--trigger", t)?,
        None => "manual".to_owned(),
    };
    let tickets = tickets(&i.tickets)?;
    if let Some(bad) = i.signatures.iter().find(|s| !is_slug(s)) {
        return Err(KbError::Usage(format!(
            "--signatures: {bad:?} is not a signature slug"
        )));
    }
    let date = time::date(session.now);
    let year = &date[..4];
    let home = format!("{date}-{}", slug(&title));
    let id = format!("{year}/{home}");
    fresh_id(session, &format!("inv:{id}"))?;
    assets(session, &i.assets)?;
    let (tickets, alerts) = (list(&tickets), list(&i.alerts));
    let (assets, signatures) = (list(&i.assets), list(&i.signatures));
    let summary = i.summary.as_deref().map(str::trim).unwrap_or_default();
    let fields = [
        ("id", Some(id.as_str())),
        ("title", Some(title.as_str())),
        ("trigger", Some(trigger.as_str())),
        ("tickets", Some(tickets.as_str())),
        ("alerts", Some(alerts.as_str())),
        ("assets", Some(assets.as_str())),
        ("status", Some("open")),
        ("signatures", Some(signatures.as_str())),
        ("created", Some(date.as_str())),
        ("updated", Some(date.as_str())),
        ("summary", Some(summary)),
    ];
    let question = i.question.as_deref().map(str::trim).unwrap_or_default();
    let sections: Vec<(&str, &str)> = SECTIONS
        .iter()
        .map(|s| (*s, if *s == "Question" { question } else { "" }))
        .collect();
    let bytes = rendered(&fields, &sections, &title)?;
    Ok(Plan {
        ops: vec![Op::CreateDir {
            path: format!("investigations/{id}"),
            files: vec![
                ("README.md".to_owned(), bytes),
                ("evidence/".to_owned(), Vec::new()),
            ],
        }],
        show: Some(format!("inv:{id}")),
        notes: Vec::new(),
        current: None,
        verb: "record-investigation",
        args: vec![id, title],
    })
}
