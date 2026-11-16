//! `status <investigation> --set concluded`: refused until the record says
//! what it was checked against (`## Checked against`), where its limits are
//! (`## Scope and limits`), and every hypothesis under `## Hypotheses` has
//! an outcome. A hypothesis is a bullet `- H<n>: <outcome> …` with outcome
//! `open`, `supported`, `refuted` or `unk` (any case); a bullet that starts
//! `- H<n>` without one is refused as unreadable rather than skipped. The
//! decision is [`writfold_core::investigation::conclude`].
use crate::defs::Sink;
use crate::error::{KbError, Result};
use writfold_core::diag::{Span, suggest};
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

/// Each `## ` heading of `text`: its name and the byte span of its line.
fn headings(text: &str) -> Vec<(&str, Span)> {
    let mut out = Vec::new();
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\r', '\n']);
        if let Some(name) = bare.strip_prefix("## ") {
            out.push((name.trim(), Span::new(at, at + bare.len())));
        }
        at += line.len();
    }
    out
}

/// A required section that is absent or empty, at the heading that may be
/// meant (a close name), at its own heading when empty, else the file.
fn section(sink: &mut Sink<'_>, doc: &MarkdownDocument, name: &str) {
    if doc.section(name).is_some_and(|s| !s.trim().is_empty()) {
        return;
    }
    let all = headings(doc.text());
    let own = all.iter().find(|(h, _)| *h == name);
    let close = all.iter().find(|(h, _)| !suggest(h, &[name]).is_empty());
    let d = match (own, close) {
        (Some((_, span)), _) => {
            let d = sink.error(
                "section.missing",
                Some(*span),
                format!("the ## {name} section is empty"),
            );
            d.label("nothing under it");
            d.help(format!("write under ## {name} what concluding it rests on"));
            d
        }
        (None, Some((written, span))) => {
            let d = sink.error(
                "section.missing",
                Some(*span),
                format!("concluding needs a ## {name} section"),
            );
            d.label("not the heading it needs");
            d.help(format!(
                "did you mean \"## {name}\"? (written \"## {written}\")"
            ));
            d
        }
        (None, None) => {
            let d = sink.error(
                "section.missing",
                None,
                format!("concluding needs a ## {name} section"),
            );
            d.help(format!("add a section: ## {name}"));
            d
        }
    };
    d.note("an investigation is concluded with ## Hypotheses, ## Checked against and ## Scope and limits".into());
}

/// Every hypothesis bullet without an outcome, or still open.
fn bullets(sink: &mut Sink<'_>, doc: &MarkdownDocument) {
    let Some(range) = doc.section_span("Hypotheses") else {
        return;
    };
    let mut at = range.start;
    for line in doc.text()[range].split_inclusive('\n') {
        let bare = line.trim_end_matches(['\r', '\n']);
        let span = Span::new(at, at + bare.len());
        at += line.len();
        let trimmed = bare.trim();
        let Some(after) = trimmed
            .strip_prefix(['-', '*'])
            .and_then(|b| label(b.trim_start()))
        else {
            continue;
        };
        match outcome(after) {
            None => {
                let d = sink.error(
                    "section.form",
                    Some(span),
                    "a hypothesis without an outcome".into(),
                );
                d.label("no outcome");
                d.help("write - H<n>: open|supported|refuted|unk".into());
            }
            Some(Hypothesis::Open) => {
                let d = sink.error(
                    "section.form",
                    Some(span),
                    "this hypothesis is still open".into(),
                );
                d.label("open");
                d.help("write its outcome: supported, refuted or unk".into());
            }
            Some(_) => {}
        }
    }
}

/// Refuse unless the investigation may be concluded; the refusal names
/// every missing section and open hypothesis at its line in `path`.
pub fn check(id: &str, path: &str, doc: &MarkdownDocument) -> Result<()> {
    let has = |name: &str| doc.section(name).is_some_and(|s| !s.trim().is_empty());
    let hyps = hypotheses(doc.section("Hypotheses").unwrap_or_default());
    if let Ok(hyps) = &hyps
        && conclude(has("Checked against"), has("Scope and limits"), hyps).none()
    {
        return Ok(());
    }
    let mut sink = Sink::new(path, doc.text());
    bullets(&mut sink, doc);
    for name in ["Checked against", "Scope and limits"] {
        section(&mut sink, doc, name);
    }
    if sink.found.is_empty() {
        return Err(KbError::Refused(format!("cannot conclude {id}")));
    }
    Err(KbError::Definition(Box::new(sink.report())))
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
