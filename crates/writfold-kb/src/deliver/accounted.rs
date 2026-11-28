//! `claims.accounted` (#189, RFC 2026-11-27-claims-in-text): every prose
//! block of a candidate is cited (and its citations fresh), tagged with
//! one of the mode's `status_tags`, or excused; the outcome is the proved
//! [`writfold_core::claims::accounted`]. A bare block is drawn at its line
//! with the ways to account for it. A near-miss tag, two tags, or a tag on
//! a cited block is a finding and leaves the block bare. The check never
//! says the text is grounded: a tag accounts for a block, nothing more.
use super::checks::{Evaluated, Finding, cites};
use super::markdown::{Block as Text, blocks};
use crate::Result;
use crate::session::Session;
use crate::stale::Verdict;
use writfold_core::claims::{Block, Standing, accounted};
use writfold_core::diag::suggest::suggest;

/// A block's tag, read from its end.
enum Tag {
    None,
    One,
    /// A bracketed word the mode does not take, with a close one it does.
    Near(String, String),
    Many,
}

fn tag(text: &str, tags: &[String]) -> Tag {
    let mut rest = text.trim_end();
    let mut found = Vec::new();
    while let Some(open) = rest
        .strip_suffix(']')
        .and_then(|r| r.rfind('[').map(|i| (i, r)))
    {
        let (at, r) = open;
        let word = &r[at + 1..];
        if word.is_empty() || !word.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
            break;
        }
        found.push(word.to_owned());
        rest = r[..at].trim_end();
    }
    let known: Vec<&str> = tags.iter().map(String::as_str).collect();
    match found.as_slice() {
        [] => Tag::None,
        [one] if known.contains(&one.as_str()) => Tag::One,
        [one] => match suggest(one, &known).first() {
            Some(near) => Tag::Near(one.clone(), (*near).to_owned()),
            None => Tag::None,
        },
        _ => Tag::Many,
    }
}

/// How the citations within a block's lines stand together.
fn standing(verdicts: &[(u32, &Verdict)], b: &Text) -> Option<Standing> {
    let mine: Vec<&Verdict> = (verdicts.iter())
        .filter(|(line, _)| (b.line..=b.end).contains(line))
        .map(|(_, v)| *v)
        .collect();
    if mine.is_empty() {
        return None;
    }
    if mine
        .iter()
        .any(|v| !matches!(v, Verdict::Fresh | Verdict::Unverifiable(_)))
    {
        return Some(Standing::Stale);
    }
    match mine.iter().any(|v| matches!(v, Verdict::Unverifiable(_))) {
        true => Some(Standing::Unverifiable),
        false => Some(Standing::Fresh),
    }
}

fn ways(tags: &[String]) -> String {
    let tagged = match tags.is_empty() {
        true => "add status_tags to the mode and end it with one".to_owned(),
        false => format!("end it with one of [{}]", tags.join("], [")),
    };
    format!(
        "no citation, status tag or excuse: cite it (writfold kb cite make), {tagged}, or rewrite it"
    )
}

/// Class every block and decide.
pub fn evaluate(
    session: &Session<'_>,
    text: &str,
    path: &str,
    tags: &[String],
) -> Result<Evaluated> {
    let cites = cites(text, path);
    let checked = crate::stale::check(session, &cites, false)?;
    let verdicts: Vec<(u32, &Verdict)> = cites
        .iter()
        .zip(&checked)
        .map(|(c, k)| (c.line, &k.verdict))
        .collect();
    let (mut classes, mut findings) = (Vec::new(), Vec::new());
    let (mut cited, mut tagged) = (0, 0);
    for b in blocks(text) {
        let said = tag(&b.text, tags);
        let class = match (standing(&verdicts, &b), said) {
            (Some(_), Tag::One | Tag::Many | Tag::Near(..)) => {
                findings.push(Finding {
                    line: b.line,
                    rule: "tag-on-cited".into(),
                    detail: "a cited block takes no status tag: the citation accounts for it"
                        .into(),
                });
                Block::Bare
            }
            (Some(s), Tag::None) => {
                cited += 1;
                Block::Cited(s)
            }
            (None, Tag::One) => {
                tagged += 1;
                Block::Tagged
            }
            (None, Tag::Near(written, near)) => {
                findings.push(Finding {
                    line: b.line,
                    rule: "tag-unknown".into(),
                    detail: format!(
                        "[{written}] is not one of the mode's status_tags; did you mean [{near}]?"
                    ),
                });
                Block::Bare
            }
            (None, Tag::Many) => {
                findings.push(Finding {
                    line: b.line,
                    rule: "tag-twice".into(),
                    detail: "a block takes one status tag".into(),
                });
                Block::Bare
            }
            (None, Tag::None) => {
                findings.push(Finding {
                    line: b.line,
                    rule: "bare".into(),
                    detail: ways(tags),
                });
                Block::Bare
            }
        };
        classes.push(class);
    }
    let bare = classes.iter().filter(|c| **c == Block::Bare).count();
    let note = format!(
        "{} blocks: {cited} cited, {tagged} tagged, 0 excused, {bare} bare",
        classes.len()
    );
    let outcome = accounted(&classes);
    if outcome != writfold_core::gate::Outcome::Fail {
        findings.retain(|f| f.rule != "bare");
    }
    Ok(Evaluated {
        outcome,
        findings,
        advice: Vec::new(),
        note: Some(note),
    })
}
