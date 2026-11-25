//! The built-in delivery checks, evaluated against a candidate's exact
//! bytes. Each returns an outcome and findings (line, rule, detail); none
//! judges tone, truth or audience fit, and none detects a claim made in
//! prose without a citation token.
//!
//! - `paragraphs.bounded`: no prose paragraph ([`super::markdown`]) has
//!   more words than the mode's bound.
//! - `references.denied`: no line matches a rule of the audience's deny
//!   list. With no deny list the outcome is `unknown`, never `pass`.
//! - `length.bounded`: the whole payload has at most `max_chars`
//!   characters and `max_lines` lines (only when a mode lists it).
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

/// The column a finding's detail starts with (`column 14` or
/// `column 14: warning: …`).
pub fn column(detail: &str) -> Option<usize> {
    let rest = detail.strip_prefix("column ")?;
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

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
    /// Advice: findings that never decide the outcome (`prose.rules`
    /// advice, a script gate's output on a pass).
    pub advice: Vec<Finding>,
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
        advice: Vec::new(),
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

/// `length.bounded`: the whole payload within the mode's `max_chars`
/// (Unicode scalar values) and `max_lines`; a finding names the line
/// where the bound is crossed.
pub fn length(text: &str, mode: &Mode) -> Evaluated {
    let mut findings = Vec::new();
    let mut said = Vec::new();
    if let Some(max) = mode.max_chars {
        said.push(format!("at most {max} characters"));
        let n = text.chars().count();
        if n > max {
            let mut seen = 0;
            let crossed = text.split_inclusive('\n').position(|l| {
                seen += l.chars().count();
                seen > max
            });
            findings.push(Finding {
                line: u32::try_from(crossed.map_or(1, |i| i + 1)).unwrap_or(u32::MAX),
                rule: format!("max-{max}-chars"),
                detail: format!("{n} characters; this mode takes at most {max} (max_chars)"),
            });
        }
    }
    if let Some(max) = mode.max_lines {
        said.push(format!("at most {max} lines"));
        let n = text.lines().count();
        if n > max {
            findings.push(Finding {
                line: u32::try_from(max + 1).unwrap_or(u32::MAX),
                rule: format!("max-{max}-lines"),
                detail: format!("{n} lines; this mode takes at most {max} (max_lines)"),
            });
        }
    }
    judged(findings, Some(said.join(", ")))
}

/// `references.denied`: the first match of each rule on each line.
pub fn references(text: &str, deny: &Deny) -> Evaluated {
    if deny.path.is_none() {
        return Evaluated {
            outcome: Outcome::Unknown,
            findings: Vec::new(),
            advice: Vec::new(),
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

/// The citation tokens of a candidate's text filed at `path`, as
/// `claims.cited-fresh` and its re-check on read see them.
pub fn cites(text: &str, path: &str) -> Vec<Cite> {
    tokens(text)
        .into_iter()
        .filter(|c| !c.illustrative)
        .enumerate()
        .map(|(ord, c)| Cite::from_citation(path, ord, c))
        .collect()
}

/// `claims.cited-fresh`: every citation token in the candidate, checked by
/// `stale` (local repositories only; nothing is fetched).
pub fn claims(session: &Session<'_>, text: &str, path: &str) -> Result<Evaluated> {
    let cites = self::cites(text, path);
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
    let recorded = checked
        .iter()
        .filter(|c| c.recorded.is_some() && c.verdict == Verdict::Fresh)
        .count();
    let note = match crate::gate::ground::recorded_note(recorded) {
        Some(said) => Some(format!("{} citation tokens; {said}", cites.len())),
        None => Some(format!("{} citation tokens", cites.len())),
    };
    if findings.is_empty() && !unverifiable.is_empty() {
        return Ok(Evaluated {
            outcome: Outcome::Unknown,
            findings: unverifiable,
            advice: Vec::new(),
            note,
        });
    }
    findings.extend(unverifiable);
    Ok(judged(findings, note))
}
