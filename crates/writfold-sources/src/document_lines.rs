use crate::unicode::{self, space, trim};

/// Original byte offsets and borrowed lines outside Markdown backtick/tilde fences.
pub fn unfenced_lines(text: &str) -> UnfencedLines<'_> {
    UnfencedLines {
        lines: unicode::lines(text),
        fence: None,
    }
}

pub struct UnfencedLines<'a> {
    lines: unicode::Lines<'a>,
    fence: Option<(char, usize)>,
}

fn marker(line: &str) -> Option<(char, usize, &str)> {
    let line = line.trim_end_matches(['\r', '\n']);
    let indent = line.bytes().take_while(|&b| b == b' ').count();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let kind = rest.chars().next()?;
    if kind != '`' && kind != '~' {
        return None;
    }
    let width = rest.chars().take_while(|&c| c == kind).count();
    (width >= 3).then_some((kind, width, &rest[width..]))
}

impl<'a> Iterator for UnfencedLines<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        for (offset, line) in self.lines.by_ref() {
            let marker = marker(line);
            if let Some((kind, width)) = self.fence {
                if let Some((next_kind, next_width, rest)) = marker
                    && kind == next_kind
                    && next_width >= width
                    && trim(rest).is_empty()
                {
                    self.fence = None;
                }
            } else if let Some((kind, width, rest)) = marker {
                if kind != '`' || !rest.contains('`') {
                    self.fence = Some((kind, width));
                } else {
                    return Some((offset, line));
                }
            } else {
                return Some((offset, line));
            }
        }
        None
    }
}

/// Equivalent to Python ^##\s+(.+?)\s*#*\s*$ over a split line.
pub(crate) fn heading(line: &str) -> Option<&str> {
    let text = line.trim_end_matches(['\r', '\n']).strip_prefix("##")?;
    let whitespace = text.len() - text.trim_start_matches(space).len();
    if whitespace == 0 {
        return None;
    }
    let mut start = whitespace;
    if start == text.len() {
        start = text.char_indices().next_back()?.0;
        if start == 0 {
            return None;
        }
    }
    let rest = &text[start..];
    let suffix = rest
        .trim_end_matches(space)
        .trim_end_matches('#')
        .trim_end_matches(space);
    let first_end = rest.chars().next()?.len_utf8();
    Some(trim(&rest[..suffix.len().max(first_end)]))
}
