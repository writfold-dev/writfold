//! Refusal text for a pinned replay: one fact per line, the first naming the
//! typed kind and the recipe span. A selector refusal carries Splicery's own
//! bounded detail (how many sites matched, or the near misses it found, with
//! at most four sites), shown here as line numbers, byte ranges and the
//! start of each site's line in the file the recipe names. Splicery drops
//! the count found when a `line_matches … expect N` does not hold, so that
//! one refusal is recounted here with Splicery's own matcher.

use crate::splicery::ReplayError;
use splicery_compile::detail::{DetailReason, SelectorDetail};
use splicery_compile::regex::{RegexError, match_lines};
use splicery_compile::{CompileDiagnostic, CompileError};
use splicery_domain::{Budget, OriginalImage, Profile, RecipeSpan};
use splicery_syntax::ast::{Collection, Statement};

/// Sites listed for a recount.
const SITES: usize = 4;

/// The most bytes of one site's line shown.
const CONTEXT: usize = 120;

/// The refusal for `error` while replaying `recipe` against `source`, the
/// current bytes of `path`.
pub fn describe(error: &ReplayError, path: &str, source: &[u8], recipe: &[u8]) -> String {
    match error {
        ReplayError::Parse(diagnostic) => format!(
            "{:?} at recipe [{},{}): {}",
            diagnostic.kind, diagnostic.origin.start, diagnostic.origin.end, diagnostic.message
        ),
        ReplayError::Lower(diagnostic) => {
            let mut text = compile(diagnostic, path, source);
            if diagnostic.kind == CompileError::UnexpectedMatchCount
                && let Some(found) = recount(recipe, diagnostic.origin, source)
            {
                let at = text.find("\n  hint:").unwrap_or(text.len());
                text.insert_str(at, &found);
            }
            text
        }
        ReplayError::Kernel(error) => format!("kernel refused {error:?} in {path}"),
        other => format!("{other} ({path})"),
    }
}

fn compile(diagnostic: &CompileDiagnostic, path: &str, source: &[u8]) -> String {
    let origin = diagnostic.origin;
    let mut out = format!(
        "{:?} at recipe [{},{}) in {path}",
        diagnostic.kind, origin.start, origin.end
    );
    if let Some(detail) = &diagnostic.detail {
        let sites: Vec<String> = detail
            .recorded()
            .map(|span| {
                let line = line_number(source, span.lo());
                format!("line {line} bytes [{},{})", span.lo(), span.hi())
            })
            .collect();
        out.push_str(&format!("\n  {}: {}", heading(detail), sites.join("; ")));
        for span in detail.recorded() {
            let line = line_number(source, span.lo());
            out.push_str(&format!(
                "\n  line {line}: {}",
                line_text(source, span.lo())
            ));
        }
    }
    if let Some(hint) = hint(diagnostic) {
        out.push_str(&format!("\n  hint: {hint}"));
    }
    out
}

fn heading(detail: &SelectorDetail) -> String {
    let count = detail.count;
    let mut text = match detail.reason {
        DetailReason::Matches => format!("matched {count} sites"),
        DetailReason::OutsideWithin => format!("{count} match(es) outside the within window"),
        DetailReason::Whitespace => {
            format!("{count} line(s) match once spaces and tabs are trimmed")
        }
        DetailReason::LineEnding => format!("{count} match(es) with LF and CRLF swapped"),
        DetailReason::Anchors => "the anchors".to_owned(),
    };
    let shown = detail.recorded().count();
    if shown < count {
        text.push_str(&format!(" (first {shown} shown)"));
    }
    text
}

fn hint(diagnostic: &CompileDiagnostic) -> Option<&'static str> {
    let reason = diagnostic.detail.as_ref().map(|detail| detail.reason);
    Some(match (diagnostic.kind, reason) {
        (CompileError::AmbiguousSelector, _) => {
            "add context to the needle, or narrow it with within a window"
        }
        (CompileError::UnexpectedMatchCount, _) => {
            "the expect count is part of the recipe; check the sites above before changing it"
        }
        (CompileError::MissingSelector, Some(DetailReason::OutsideWithin)) => {
            "the needle occurs outside the within window; widen or drop it"
        }
        (CompileError::MissingSelector, Some(DetailReason::Whitespace)) => {
            "copy the exact indentation and trailing spaces from the file"
        }
        (CompileError::MissingSelector, Some(DetailReason::LineEnding)) => {
            "the file uses the other line ending"
        }
        (CompileError::MissingSelector, _) => {
            "nothing matched; selectors compare exact bytes of the worktree file"
        }
        _ => return None,
    })
}

/// For an unmet `expect N` at `origin`: how many lines match, and where.
/// Only a top-level `line_matches` without `within` is recounted.
fn recount(recipe: &[u8], origin: RecipeSpan, source: &[u8]) -> Option<String> {
    let parsed = splicery_syntax::parse_recipe(recipe, None).ok()?;
    let (binder, pattern, wanted) = parsed.statements().iter().find_map(|s| match s {
        Statement::Iteration(it) => match &it.collection {
            Collection::LineMatches {
                pattern,
                within: None,
                expect: wanted,
                origin: at,
            } if *at == origin => Some((it.binder.clone(), pattern.clone(), *wanted)),
            _ => None,
        },
        _ => None,
    })?;
    let image = OriginalImage::new(source.to_vec(), Profile::Text)?;
    let count = |expected: usize| {
        let mut budget = Budget::new();
        let capture = image.capture_sha256();
        match_lines(
            &image,
            &binder,
            &pattern,
            None,
            expected,
            capture,
            &mut budget,
        )
    };
    let actual = match count(wanted) {
        Err(RegexError::UnexpectedMatchCount { actual, .. }) => actual,
        _ => return None,
    };
    let mut out = format!("\n  expected {wanted} matching lines, found {actual}");
    if let Ok(set) = count(actual) {
        let sites: Vec<String> = set
            .members
            .iter()
            .take(SITES)
            .map(|m| {
                let line = line_number(source, m.span.lo());
                format!("line {line} bytes [{},{})", m.span.lo(), m.span.hi())
            })
            .collect();
        if !sites.is_empty() {
            out.push_str(&format!(": {}", sites.join("; ")));
        }
        for m in set.members.iter().take(SITES) {
            let line = line_number(source, m.span.lo());
            out.push_str(&format!(
                "\n  line {line}: {}",
                line_text(source, m.span.lo())
            ));
        }
    }
    Some(out)
}

fn line_number(bytes: &[u8], at: usize) -> usize {
    let at = at.min(bytes.len());
    bytes[..at].iter().filter(|byte| **byte == b'\n').count() + 1
}

fn line_text(bytes: &[u8], at: usize) -> String {
    let at = at.min(bytes.len());
    let start = bytes[..at]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |index| index + 1);
    let end = bytes[start..]
        .iter()
        .position(|byte| *byte == b'\n' || *byte == b'\r')
        .map_or(bytes.len(), |offset| start + offset);
    let cut = end.min(start + CONTEXT);
    let mut text: String = String::from_utf8_lossy(&bytes[start..cut])
        .chars()
        .flat_map(char::escape_debug)
        .collect();
    if cut < end {
        text.push_str("...");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::describe;
    use crate::splicery::replay;

    #[test]
    fn ambiguous_selector_names_every_site() {
        let source = b"a = 1\nb = 1\nc = 1\n".to_vec();
        let recipe = b"edit \"x.txt\" {\n  replace one text \"= 1\" with \"= 2\"\n}\n";
        let error = replay(source.clone(), "x.txt", recipe).err();
        let text = error.map(|e| describe(&e, "x.txt", &source, recipe));
        let text = text.unwrap_or_default();
        assert!(text.starts_with("AmbiguousSelector at recipe ["), "{text}");
        assert!(
            text.contains(
                "matched 3 sites: line 1 bytes [2,5); line 2 bytes [8,11); line 3 bytes [14,17)"
            ),
            "{text}"
        );
        assert!(text.contains("\n  line 2: b = 1"), "{text}");
        let recipe = b"edit \"x.txt\" {\n  each @l in line_matches rx\"= 1$\" expect 2 {\n    delete @l\n  }\n}\n";
        let error = replay(source.clone(), "x.txt", recipe).err();
        let text = error.map(|e| describe(&e, "x.txt", &source, recipe));
        let text = text.unwrap_or_default();
        assert!(
            text.starts_with("UnexpectedMatchCount at recipe ["),
            "{text}"
        );
        assert!(
            text.contains("expected 2 matching lines, found 3: line 1 bytes [2,5)"),
            "{text}"
        );
    }
}
