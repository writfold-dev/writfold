//! Handwritten algorithms over pinned, generated Python Unicode tables.
use crate::unicode_data::{DECIMAL, LINE_BREAK, SPACE, WORD};

fn contains(ranges: &[(u32, u32)], character: char) -> bool {
    let value = u32::from(character);
    ranges
        .binary_search_by(|&(first, last)| {
            if value < first {
                std::cmp::Ordering::Greater
            } else if value > last {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

pub(crate) fn space(character: char) -> bool {
    contains(SPACE, character)
}
pub(crate) fn word(character: char) -> bool {
    contains(WORD, character)
}
pub(crate) fn trim(text: &str) -> &str {
    text.trim_matches(space)
}
pub(crate) fn trim_start(text: &str) -> &str {
    text.trim_start_matches(space)
}
pub(crate) fn trim_end(text: &str) -> &str {
    text.trim_end_matches(space)
}

pub(crate) fn has_line_break(text: &str) -> bool {
    text.chars()
        .any(|character| contains(LINE_BREAK, character))
}

pub(crate) fn decimal(character: char) -> Option<u8> {
    let value = u32::from(character);
    DECIMAL
        .iter()
        .find(|&&(first, last)| first <= value && value <= last)
        .map(|&(first, _)| ((value - first) % 10) as u8)
}

/// Python str.splitlines(keepends=True), with byte offsets and no phantom tail.
pub(crate) fn lines(text: &str) -> Lines<'_> {
    Lines { text, offset: 0 }
}

pub(crate) struct Lines<'a> {
    text: &'a str,
    offset: usize,
}

impl<'a> Iterator for Lines<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == self.text.len() {
            return None;
        }
        let start = self.offset;
        let rest = &self.text[start..];
        let mut end = self.text.len();
        for (index, character) in rest.char_indices() {
            if contains(LINE_BREAK, character) {
                end = start + index + character.len_utf8();
                if character == '\r' && self.text[end..].starts_with('\n') {
                    end += 1;
                }
                break;
            }
        }
        self.offset = end;
        Some((start, &self.text[start..end]))
    }
}

pub(crate) fn without_line_end(line: &str) -> &str {
    line.trim_end_matches(|c| contains(LINE_BREAK, c))
}
