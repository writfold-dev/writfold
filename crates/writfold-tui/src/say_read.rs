//! Whole or nothing: a verb's text is quoted only when no token of it is
//! notation (brackets, flags, `key=value`, positions, hashes, stamps,
//! typed ids, commands). A hand-written token scan; no regex at run time.
use crate::say::Piece;

/// True when no whitespace-separated token of `text` is notation.
pub fn readable(text: &str) -> bool {
    if text.contains("writfold kb") || text.contains('`') || text.contains('→') {
        return false;
    }
    text.split_whitespace().all(plain_token)
}

/// `Quoted` when the text reads as words, else nothing: whole or nothing.
pub fn quote(text: &str) -> Option<Piece> {
    (!text.is_empty() && readable(text)).then(|| Piece::Quoted(text.to_owned()))
}

/// Label prefixes the verbs print before a continuation.
const LABELS: [&str; 8] = [
    "owes:", "next:", "read:", "confirm:", "accept:", "splice:", "under:", "field:",
];

/// Chip glyphs: two in a row are a strip, not words.
const CHIPS: &str = "●◆○!×?·";

fn plain_token(token: &str) -> bool {
    let chars: Vec<char> = token.chars().collect();
    let marked = |words: &[&str]| words.contains(&token);
    let strip = chars
        .windows(2)
        .any(|w| CHIPS.contains(w[0]) && CHIPS.contains(w[1]));
    let flagged = |lead: &str| {
        token
            .strip_prefix(lead)
            .and_then(|rest| rest.chars().next())
            .is_some_and(|c| c.is_ascii_alphabetic())
    };
    let bracket = token
        .find('[')
        .is_some_and(|open| token[open..].contains(']'));
    !(bracket
        || strip
        || marked(&LABELS)
        || marked(&["WHO", "#"])
        || token.starts_with("would-")
        || token.contains("REF…")
        || token.starts_with('@')
        || flagged("--")
        || flagged("!")
        || between(&chars, '=', char::is_alphanumeric)
        || between(&chars, '_', char::is_alphabetic)
        || run(&chars, |c| c.is_ascii_digit()) >= 8
        || run(&chars, |c| c.is_ascii_hexdigit()) >= 12
        || iso(token)
        || typed(&chars)
        || token.split('/').filter(|s| !s.is_empty()).count() >= 3)
}

/// `mark` with a matching character on both sides.
fn between(chars: &[char], mark: char, kind: fn(char) -> bool) -> bool {
    chars
        .windows(3)
        .any(|w| w[1] == mark && kind(w[0]) && kind(w[2]))
}

fn run(chars: &[char], kind: impl Fn(char) -> bool) -> usize {
    let mut best = 0;
    let mut now = 0;
    for c in chars {
        now = if kind(*c) { now + 1 } else { 0 };
        best = best.max(now);
    }
    best
}

/// `DDDD-DD-DDTDD:DD` or `DD:DD:DDZ` anywhere in the token.
fn iso(token: &str) -> bool {
    let bytes = token.as_bytes();
    ["dddd-dd-ddTdd:dd", "dd:dd:ddZ"].iter().any(|shape| {
        bytes.windows(shape.len()).any(|w| {
            w.iter().zip(shape.bytes()).all(|(b, s)| match s {
                b'd' => b.is_ascii_digit(),
                other => *b == other,
            })
        })
    })
}

/// A lowercase word directly followed by `:` and more text (`gate:lint`).
fn typed(chars: &[char]) -> bool {
    chars.iter().enumerate().any(|(at, c)| {
        *c == ':'
            && at > 0
            && chars[at - 1].is_ascii_lowercase()
            && chars.get(at + 1).is_some_and(|n| !n.is_whitespace())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_refuses_notation() {
        for text in [
            "1 reference by agent; cited evidence moved: gate:lint was satisfied, now stale",
            "mechanical pass exit=0 at kb@0ea3a743bfef scope=a61da1b21345 paths=1",
            "created=2026-11-10T23:59:00Z session=s-progress",
            "project:harbor",
            "writfold kb gate run lint",
            "unit [satisfied] mechanical",
            "@runs/x.json:1",
            "see --by WHO",
            "flag !stale here",
            "commit 0ea3a743bfef1",
            "run 202611010900",
            "at 2026-11-10T23:59",
            "decided_by ops",
            "proposal/decision/2026-03-02-batch",
            "a `tick`",
            "a → b",
            "at 09:00:00Z",
            "owes: x",
            "would-deny",
            "REF… by WHO",
            "strip ●!",
        ] {
            assert!(!readable(text), "{text}");
        }
        for text in [
            "changed in scope: harbor/lint.txt",
            "1 reference by agent",
            "entry 3 of 3: Review started.",
            "Review started; lint went stale after the rename.",
            "ticket HBR-210: Lint gate scope misses the command module",
            "at 14:10 on 2026-11-10",
        ] {
            assert!(readable(text), "{text}");
        }
        assert_eq!(quote(""), None);
        assert_eq!(quote("a=b"), None);
    }
}
