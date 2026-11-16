//! Phrases as terminal rows: the lead column, then the rest broken at
//! spaces, continuation rows hanging under the rest. A mark keeps its own
//! tone; names are plain; the rest takes the phrase's tone.
use crate::lines::Tone;
use crate::rail_text::width as cells;
use crate::say::{Phrase, Piece, Told};
use crate::style::{self, Look};
use ratatui::text::{Line, Span};

/// The rows of every phrase at `width` cells, then the withheld line.
pub fn told(told: &Told, width: usize, look: Look) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = told
        .lines
        .iter()
        .flat_map(|p| phrase(p, width, look))
        .collect();
    if let Some(closing) = told.closing() {
        if out.last().is_some_and(|l| l.width() > 0) {
            out.push(Line::default());
        }
        out.extend(phrase(&closing, width, look));
    }
    out
}

/// One phrase's rows.
pub fn phrase(p: &Phrase, width: usize, look: Look) -> Vec<Line<'static>> {
    let indent = usize::from(p.indent);
    let mut first: Vec<Span<'static>> = vec![Span::raw(" ".repeat(indent))];
    let lead = &p.pieces[..p.lead];
    for (at, piece) in lead.iter().enumerate() {
        if at > 0 {
            first.push(Span::raw(" "));
        }
        let tone = match piece {
            Piece::Mark(cell) => cell.tone,
            _ if p.indent == 0 && matches!(p.tone, Tone::Head | Tone::Current) => Tone::Head,
            _ => Tone::Plain,
        };
        first.push(Span::styled(
            piece.text().to_owned(),
            style::style(look, tone),
        ));
    }
    let rest = p.rest_text();
    let used: usize = first.iter().map(|s| cells(&s.content)).sum();
    let hang = if p.lead > 0 && !rest.is_empty() {
        let column = usize::from(p.column).max(used + 3);
        first.push(Span::raw(" ".repeat(column - used)));
        column
    } else {
        used
    };
    let hang = if p.lead > 0 && hang + 12 > width {
        indent + 2
    } else {
        hang
    };
    let rest_style = style::style(look, p.tone);
    let start: usize = first.iter().map(|s| cells(&s.content)).sum();
    let rows = wrap(&rest, width, start, hang);
    let mut out = Vec::new();
    for (n, row) in rows.into_iter().enumerate() {
        let mut spans = if n == 0 {
            std::mem::take(&mut first)
        } else {
            vec![Span::raw(" ".repeat(hang))]
        };
        spans.push(Span::styled(row, rest_style));
        out.push(Line::from(spans));
    }
    if out.is_empty() {
        out.push(Line::from(first));
    }
    out
}

/// `text` broken at spaces: the first row starts at `start` cells, the
/// others at `hang`; a word longer than a row is cut across rows.
fn wrap(text: &str, width: usize, start: usize, hang: usize) -> Vec<String> {
    let mut rows = Vec::new();
    let mut row = String::new();
    let mut used = start;
    for word in text.split(' ') {
        let gap = usize::from(!row.is_empty());
        if !row.is_empty() && used + gap + cells(word) > width {
            rows.push(std::mem::take(&mut row));
            used = hang;
        } else if !row.is_empty() {
            row.push(' ');
            used += 1;
        }
        // A first word that fits a continuation row starts there whole.
        if row.is_empty() && rows.is_empty() && used > hang {
            let size = cells(word);
            if used + size > width && hang + size <= width {
                rows.push(String::new());
                used = hang;
            }
        }
        let mut rest = word;
        while used + cells(rest) > width && width > hang + 1 {
            let room = width.saturating_sub(used);
            let cut = rest
                .char_indices()
                .scan(0, |taken, (at, ch)| {
                    *taken += cells(ch.encode_utf8(&mut [0; 4]));
                    Some((at, *taken))
                })
                .find(|(_, taken)| *taken > room)
                .map_or(rest.len(), |(at, _)| at);
            if cut == 0 {
                if row.is_empty() {
                    break;
                }
                rows.push(std::mem::take(&mut row));
                used = hang;
                continue;
            }
            row.push_str(&rest[..cut]);
            rows.push(std::mem::take(&mut row));
            used = hang;
            rest = &rest[cut..];
        }
        row.push_str(rest);
        used += cells(rest);
    }
    if !row.is_empty() || rows.is_empty() {
        rows.push(row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::say::said;

    fn text(lines: &[Line<'_>]) -> Vec<String> {
        lines.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn rest_hangs_under_its_column() {
        let p = Phrase::led(
            2,
            Tone::Plain,
            vec![
                said("tests"),
                said("out of date"),
                said("the evidence it cites has changed"),
            ],
            1,
        );
        let rows = text(&phrase(&p, 40, Look::default()));
        assert_eq!(
            rows,
            vec![
                "  tests   out of date · the evidence it",
                "          cites has changed",
            ]
        );
        let long = Phrase::new(0, Tone::Plain, vec![said("abcdefghijklmnop")]);
        assert_eq!(
            text(&phrase(&long, 6, Look::default())),
            vec!["abcdef", "ghijkl", "mnop"]
        );
    }
}
