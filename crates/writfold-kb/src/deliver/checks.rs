//! The built-in delivery checks, evaluated against a candidate's exact
//! bytes. Each returns an outcome and findings (line, rule, detail); none
//! judges tone, truth or audience fit, and none detects a claim made in
//! prose without a citation token.
//!
//! - `paragraphs.bounded`: no prose paragraph ([`super::markdown`]) has
//!   more words than the mode's bound.
//! - `references.denied`: no line matches a rule of the audience's deny
//!   list. With no deny list the outcome is `unknown`, never `pass`.
//! - `claims.cited-fresh`: every KB citation token is `fresh` under
//!   `stale`'s verdicts. A token `stale` cannot check makes the outcome
//!   `unknown`; any other non-fresh verdict fails it.
use super::config::{Deny, Mode};
use super::markdown;
use crate::error::Result;
use crate::parse::cite::tokens;
use crate::session::Session;
use crate::stale::{self, Cite, Verdict};
use writfold_core::gate::Outcome;

/// The most findings a receipt keeps (the total is kept too).
pub const MAX_FINDINGS: usize = 20;

/// One thing a check found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub line: u32,
    pub rule: String,
    pub detail: String,
}

/// What one evaluation found.
#[derive(Clone, Debug)]
pub struct Evaluated {
    pub outcome: Outcome,
    pub findings: Vec<Finding>,
    /// Why the outcome is `unknown`, or a fact worth showing (`0 tokens`).
    pub note: Option<String>,
}

fn judged(findings: Vec<Finding>, note: Option<String>) -> Evaluated {
    Evaluated {
        outcome: if findings.is_empty() {
            Outcome::Pass
        } else {
            Outcome::Fail
        },
        findings,
        note,
    }
}

/// `paragraphs.bounded`.
pub fn paragraphs(text: &str, mode: &Mode) -> Evaluated {
    let bound = mode.paragraph_words;
    let findings = markdown::paragraphs(text)
        .into_iter()
        .filter(|p| p.words > bound)
        .map(|p| Finding {
            line: p.line,
            rule: format!("max-{bound}-words"),
            detail: format!("{} words", p.words),
        })
        .collect();
    judged(
        findings,
        Some(format!("at most {bound} words per paragraph")),
    )
}

/// `references.denied`: the first match of each rule on each line.
pub fn references(text: &str, deny: &Deny) -> Evaluated {
    if deny.path.is_none() {
        return Evaluated {
            outcome: Outcome::Unknown,
            findings: Vec::new(),
            note: Some(format!(
                "no deny list for audience {}: add {}/deny-{}.toml",
                deny.audience,
                super::config::DIR,
                deny.audience
            )),
        };
    }
    let mut findings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for rule in &deny.rules {
            if let Some(found) = rule.regex.find(line.as_bytes()) {
                findings.push(Finding {
                    line: u32::try_from(index + 1).unwrap_or(u32::MAX),
                    rule: rule.name.clone(),
                    detail: format!("column {}", found.start() + 1),
                });
            }
        }
    }
    let note = format!("{} rules for {}", deny.rules.len(), deny.audience);
    judged(findings, Some(note))
}

/// `claims.cited-fresh`: every citation token in the candidate, checked by
/// `stale` (local repositories only; nothing is fetched).
pub fn claims(session: &Session<'_>, text: &str, path: &str) -> Result<Evaluated> {
    let cites: Vec<Cite> = tokens(text)
        .into_iter()
        .filter(|c| !c.illustrative)
        .enumerate()
        .map(|(ord, c)| Cite::from_citation(path, ord, c))
        .collect();
    let checked = stale::check(session, &cites, false)?;
    let mut findings = Vec::new();
    let mut unverifiable = Vec::new();
    for (cite, checked) in cites.iter().zip(&checked) {
        let finding = Finding {
            line: cite.line,
            rule: checked.verdict.name().to_owned(),
            detail: crate::verbs::stale::target(&cite.raw),
        };
        match &checked.verdict {
            Verdict::Fresh => {}
            Verdict::Unverifiable(why) => unverifiable.push(Finding {
                detail: format!("{} ({why})", finding.detail),
                ..finding
            }),
            _ => findings.push(finding),
        }
    }
    let note = Some(format!("{} citation tokens", cites.len()));
    if findings.is_empty() && !unverifiable.is_empty() {
        return Ok(Evaluated {
            outcome: Outcome::Unknown,
            findings: unverifiable,
            note,
        });
    }
    findings.extend(unverifiable);
    Ok(judged(findings, note))
}
