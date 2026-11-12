//! ID-shaped tokens in prose and JSON strings, for `mention` edges. Phase A
//! cannot know which IDs exist, so it keeps every token that could be one;
//! Phase B keeps those that resolve.
//!
//! The ID-boundary rule: a token is a maximal run of `[A-Za-z0-9_./-]`
//! with trailing dots removed (a sentence end), so one ID is never found
//! inside a longer one (`harbor-api` is not in `harbor-api-v2`, in
//! `x/harbor-api` or in `harbor-api.json`). A run preceded by `ev:`, `sig:`,
//! `inv:` or another typed prefix is kept with that prefix. A run must hold
//! a letter and a digit or separator: plain words (and so purely alphabetic
//! aliases) and plain numbers or dates are not kept.

/// Typed prefixes a mention may carry, written `<prefix>:<id>`.
const TYPED: [&str; 8] = [
    "ev",
    "sig",
    "inv",
    "project",
    "dossier",
    "runbook",
    "correction",
    "rel",
];

fn id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-')
}

fn shaped(token: &str) -> bool {
    token.len() >= 3
        && token.bytes().any(|b| b.is_ascii_alphabetic())
        && token
            .bytes()
            .any(|b| b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'/' | b'-'))
}

/// The typed prefix written immediately before byte `start` of `line`.
fn prefix_before(line: &str, start: usize) -> Option<&'static str> {
    let before = line.get(..start)?.strip_suffix(':')?;
    TYPED.into_iter().find(|p| {
        before.ends_with(p)
            && !before[..before.len() - p.len()]
                .chars()
                .next_back()
                .is_some_and(id_char)
    })
}

/// Distinct candidate tokens of `text` with the one-based line of their
/// first occurrence, in order of first occurrence. Lines before `from`
/// (one-based) are skipped, so front matter is not scanned twice.
pub fn scan(text: &str, from: u32) -> Vec<(String, u32)> {
    let mut seen = std::collections::BTreeSet::new();
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let number = u32::try_from(index + 1).unwrap_or(u32::MAX);
        if number < from {
            continue;
        }
        let mut chars = line.char_indices().peekable();
        while let Some((start, c)) = chars.next() {
            if !id_char(c) {
                continue;
            }
            let mut end = start + c.len_utf8();
            while let Some(&(at, next)) = chars.peek() {
                if !id_char(next) {
                    break;
                }
                end = at + next.len_utf8();
                chars.next();
            }
            let run = line[start..end].trim_end_matches('.');
            let token = match prefix_before(line, start) {
                Some(prefix) if !run.is_empty() => format!("{prefix}:{run}"),
                _ if shaped(run) => run.to_owned(),
                _ => continue,
            };
            if seen.insert(token.clone()) {
                found.push((token, number));
            }
        }
    }
    found
}

/// The first line after a leading front-matter block, or 1.
pub fn body_start(text: &str) -> u32 {
    let mut lines = text.lines().enumerate();
    if lines.next().is_none_or(|(_, first)| first.trim() != "---") {
        return 1;
    }
    lines
        .find(|(_, line)| line.trim() == "---")
        .and_then(|(index, _)| u32::try_from(index + 2).ok())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::{body_start, scan};

    #[test]
    fn boundaries_and_prefixes() {
        let text = "---\nassets: [skip-me-1]\n---\nSee harbor-api, not harbor-api-v2 or x/harbor-api.\n\
                    HBR-101 closed; sig:fsync-stall and ev:harbor/flush-latency. The ledger, 2026-01-02, \
                    harbor-api.json and decision/2026-03-02-batch-ledger-writes.\n";
        let tokens: Vec<_> = scan(text, body_start(text))
            .into_iter()
            .map(|(t, line)| format!("{t}@{line}"))
            .collect();
        assert_eq!(
            tokens,
            [
                "harbor-api@4",
                "harbor-api-v2@4",
                "x/harbor-api@4",
                "HBR-101@5",
                "sig:fsync-stall@5",
                "ev:harbor/flush-latency@5",
                "harbor-api.json@5",
                "decision/2026-03-02-batch-ledger-writes@5",
            ]
        );
        assert_eq!(body_start("no front matter"), 1);
    }
}
