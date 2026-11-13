//! `status <investigation> --set concluded`: refused until the record says
//! what it was checked against (`## Checked against`), where its limits are
//! (`## Scope and limits`), and every hypothesis under `## Hypotheses` has
//! an outcome. A hypothesis is a bullet `- H<n>: <outcome> …` with outcome
//! `open`, `supported`, `refuted` or `unk` (any case); a bullet that starts
//! `- H<n>` without one is refused as unreadable rather than skipped. The
//! decision is [`writfold_core::investigation::conclude`].
use crate::error::{KbError, Result};
use writfold_core::investigation::{Hypothesis, conclude};
use writfold_sources::documents::MarkdownDocument;

/// `H` followed by digits, and the rest of the text.
fn label(text: &str) -> Option<&str> {
    let rest = text.strip_prefix(['H', 'h'])?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let after = &rest[digits..];
    let boundary = after.chars().next().is_none_or(|c| !c.is_alphanumeric());
    (digits > 0 && boundary).then_some(after)
}

fn outcome(after: &str) -> Option<Hypothesis> {
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    let word: String = after
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    match word.to_ascii_lowercase().as_str() {
        "open" => Some(Hypothesis::Open),
        "supported" => Some(Hypothesis::Supported),
        "refuted" => Some(Hypothesis::Refuted),
        "unk" => Some(Hypothesis::Unknown),
        _ => None,
    }
}

/// The outcomes of the hypothesis bullets in `section`.
pub fn hypotheses(section: &str) -> Result<Vec<Hypothesis>> {
    let mut found = Vec::new();
    for line in section.lines() {
        let line = line.trim();
        let Some(bullet) = line.strip_prefix(['-', '*']) else {
            continue;
        };
        let Some(after) = label(bullet.trim_start()) else {
            continue;
        };
        match outcome(after) {
            Some(h) => found.push(h),
            None => {
                return Err(KbError::Refused(format!(
                    "unreadable hypothesis {line:?}: write - H<n>: open|supported|refuted|unk"
                )));
            }
        }
    }
    Ok(found)
}

/// Refuse unless the investigation may be concluded.
pub fn check(id: &str, doc: &MarkdownDocument) -> Result<()> {
    let has = |name: &str| doc.section(name).is_some_and(|s| !s.trim().is_empty());
    let hyps = hypotheses(doc.section("Hypotheses").unwrap_or_default())?;
    let missing = conclude(has("Checked against"), has("Scope and limits"), &hyps);
    if missing.none() {
        return Ok(());
    }
    let mut owed = Vec::new();
    if missing.checked_against {
        owed.push("a ## Checked against section".to_owned());
    }
    if missing.scope_and_limits {
        owed.push("a ## Scope and limits section".to_owned());
    }
    if missing.open > 0 {
        owed.push(format!("an outcome for {} open hypotheses", missing.open));
    }
    Err(KbError::Refused(format!(
        "cannot conclude {id}: it needs {}",
        owed.join(", ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_hypothesis_bullets() -> Result<()> {
        let text = "- H1: supported by the trace\n* h2 : REFUTED\n- H3: unk\nprose\n- Hx not one\n";
        assert_eq!(
            hypotheses(text)?,
            [
                Hypothesis::Supported,
                Hypothesis::Refuted,
                Hypothesis::Unknown
            ]
        );
        assert_eq!(hypotheses("- H4: open, still looking")?, [Hypothesis::Open]);
        assert!(hypotheses("- H5 no outcome").is_err());
        assert!(hypotheses("- H6: maybe").is_err());
        assert!(hypotheses("- H12abc: open")?.is_empty());
        Ok(())
    }
}
