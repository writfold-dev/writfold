//! Paragraphs of a Markdown candidate, for `paragraphs.bounded`. The rules
//! are deliberately simple and line-based, and are the documented
//! contract (kb-v1-design.md §19.3):
//!
//! - A blank line ends a paragraph.
//! - Fenced code (a line opening with three or more backticks or tildes,
//!   up to the matching closing fence or the end) is not prose: skipped.
//! - An indented line (four spaces or a tab) that does not continue an
//!   open paragraph is indented code: skipped.
//! - A table row (a line starting with `|`), a heading (`#`) and a
//!   thematic break (`---`, `***`, `___`) end a paragraph and are skipped.
//! - Each list item (`-`, `*`, `+`, or `1.`/`1)` followed by a space)
//!   starts a paragraph of its own; its continuation lines belong to it.
//! - A block quote's `>` markers are removed and its text is measured
//!   like any other (quoted text is part of the payload); a line holding
//!   only `>` ends a paragraph.
//! - A word is a whitespace-separated token holding at least one letter
//!   or digit; Markdown punctuation alone (`-`, `|`, `**`) is not a word.

/// One paragraph: its first line (one-based) and its word count.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Paragraph {
    pub line: u32,
    pub words: usize,
}

/// The number of words in `text`.
pub fn words(text: &str) -> usize {
    text.split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count()
}

/// Up to three leading spaces removed, as Markdown allows before a block.
fn unindent(line: &str) -> &str {
    let spaces = line.bytes().take_while(|b| *b == b' ').count();
    if spaces <= 3 { &line[spaces..] } else { line }
}

/// A fence opener's character and length.
fn fence(line: &str) -> Option<(char, usize)> {
    let text = unindent(line);
    let first = text.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let count = text.chars().take_while(|c| *c == first).count();
    (count >= 3).then_some((first, count))
}

fn closes(line: &str, (ch, count): (char, usize)) -> bool {
    let text = unindent(line);
    let run = text.chars().take_while(|c| *c == ch).count();
    run >= count && text[run * ch.len_utf8()..].trim().is_empty()
}

/// The text after a list marker, if the line is a list item.
fn item(text: &str) -> Option<&str> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = text.strip_prefix(marker) {
            return Some(rest);
        }
    }
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    let rest = text.get(digits..)?;
    if (1..=9).contains(&digits) {
        return rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") "));
    }
    None
}

fn thematic(text: &str) -> bool {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    compact.len() >= 3
        && ['-', '*', '_']
            .iter()
            .any(|m| compact.chars().all(|c| c == *m))
}

fn heading(text: &str) -> bool {
    let hashes = text.chars().take_while(|c| *c == '#').count();
    (1..=6).contains(&hashes)
        && text[hashes..]
            .chars()
            .next()
            .is_none_or(char::is_whitespace)
}

/// Remove block-quote markers (`>` with an optional space), any depth.
fn unquote(mut text: &str) -> (&str, bool) {
    let mut quoted = false;
    while let Some(rest) = unindent(text).strip_prefix('>') {
        quoted = true;
        text = rest.strip_prefix(' ').unwrap_or(rest);
    }
    (text, quoted)
}

/// Every prose paragraph of `text`, in order.
pub fn paragraphs(text: &str) -> Vec<Paragraph> {
    let mut out = Vec::new();
    let mut open: Option<Paragraph> = None;
    let mut fenced: Option<(char, usize)> = None;
    let mut flush = |open: &mut Option<Paragraph>| {
        if let Some(p) = open.take().filter(|p| p.words > 0) {
            out.push(p);
        }
    };
    for (index, raw) in text.lines().enumerate() {
        let number = u32::try_from(index + 1).unwrap_or(u32::MAX);
        if let Some(f) = fenced {
            if closes(raw, f) {
                fenced = None;
            }
            continue;
        }
        let (line, _) = unquote(raw);
        if let Some(f) = fence(line) {
            flush(&mut open);
            fenced = Some(f);
            continue;
        }
        if line.trim().is_empty() {
            flush(&mut open);
            continue;
        }
        let indented = line.starts_with("    ") || line.starts_with('\t');
        if indented && open.is_none() {
            continue;
        }
        let text = unindent(line);
        if text.starts_with('|') || heading(text) || thematic(text) {
            flush(&mut open);
            continue;
        }
        let body = match item(text) {
            Some(rest) => {
                flush(&mut open);
                rest
            }
            None => text,
        };
        let p = open.get_or_insert(Paragraph {
            line: number,
            words: 0,
        });
        p.words += words(body);
    }
    flush(&mut open);
    out
}

#[cfg(test)]
mod tests {
    use super::{Paragraph, paragraphs, words};

    fn counts(text: &str) -> Vec<(u32, usize)> {
        paragraphs(text)
            .into_iter()
            .map(|Paragraph { line, words }| (line, words))
            .collect()
    }

    #[test]
    fn words_need_a_letter_or_digit() {
        assert_eq!(words("a - b ** | c3 `x`"), 4);
    }

    #[test]
    fn blocks_follow_the_documented_rules() {
        let text = "# Title\n\none two three\nfour five\n\n```\ncode code code\n\nmore code\n```\n\n- item one\n- item two two\n  continued here\n\n| a | b |\n|---|---|\n| c | d |\n\n> quoted words here\n>\n> second quote\n\n    indented code block\n\n1. first\n2) second\n---\nafter break\n~~~~\nnever closed ~~~\n";
        assert_eq!(
            counts(text),
            vec![
                (3, 5),
                (12, 2),
                (13, 5),
                (20, 3),
                (22, 2),
                (26, 1),
                (27, 1),
                (29, 2)
            ]
        );
    }

    #[test]
    fn an_unclosed_fence_hides_the_rest() {
        assert_eq!(counts("a b\n```\nc d e\n\nf g\n"), vec![(1, 2)]);
    }
}
