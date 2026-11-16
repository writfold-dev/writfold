//! One source line as a diagnostic shows it: tabs as four spaces, control
//! characters escaped, wide characters measured by the caller's cell
//! function, and a line too long for the width windowed around the span
//! with `…` at the cut ends.
use super::lines::clean;
use alloc::string::String;
use alloc::vec::Vec;

/// The drawn line and where its caret goes, in cells.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Drawn {
    pub text: String,
    pub caret_at: usize,
    pub carets: usize,
}

struct Piece {
    text: String,
    start: usize,
    width: usize,
}

fn pieces(line: &str, cell: fn(char) -> usize) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut at = 0;
    for c in line.chars() {
        let (text, width) = match c {
            '\t' => (String::from("    "), 4),
            c if c.is_control() => {
                let text = clean(alloc::string::ToString::to_string(&c).as_str());
                let width = text.chars().count();
                (text, width)
            }
            c => (alloc::string::ToString::to_string(&c), cell(c)),
        };
        out.push(Piece {
            text,
            start: at,
            width,
        });
        at += width;
    }
    out
}

/// Draw `line` with a caret under the `len` characters from column `col`
/// (1-based), within `room` cells.
pub fn draw(line: &str, col: u32, len: u32, room: usize, cell: fn(char) -> usize) -> Drawn {
    let pieces = pieces(line, cell);
    let total: usize = pieces.iter().map(|p| p.width).sum();
    let first = usize::try_from(col).unwrap_or(usize::MAX).saturating_sub(1);
    let len = usize::try_from(len).unwrap_or(0);
    let caret_at = pieces.get(first).map_or(total, |p| p.start);
    let span_end = pieces
        .get(first..pieces.len().min(first.saturating_add(len)))
        .map_or(caret_at, |s| {
            s.last().map_or(caret_at, |p| p.start + p.width)
        });
    let carets = span_end.saturating_sub(caret_at).max(1);
    if total.max(caret_at + 1) <= room {
        let text = pieces.iter().map(|p| p.text.as_str()).collect();
        return Drawn {
            text,
            caret_at,
            carets,
        };
    }
    window(&pieces, total, caret_at, carets, room, cell('…'))
}

fn window(
    pieces: &[Piece],
    total: usize,
    caret_at: usize,
    carets: usize,
    room: usize,
    mark: usize,
) -> Drawn {
    let content = room.saturating_sub(2 * mark).max(1);
    let mut from = caret_at.saturating_sub(content / 3);
    if from + content > total + 1 {
        from = (total + 1).saturating_sub(content);
    }
    let to = from + content;
    let mut text = String::new();
    let mut shown_from = None;
    for p in pieces {
        if p.start >= from && p.start + p.width <= to {
            if shown_from.is_none() {
                shown_from = Some(p.start);
                if p.start > 0 {
                    text.push('…');
                }
            }
            text.push_str(&p.text);
        }
    }
    let start = shown_from.unwrap_or(from);
    let left = if start > 0 { mark } else { 0 };
    let shown_to = pieces
        .iter()
        .rfind(|p| p.start >= from && p.start + p.width <= to)
        .map_or(start, |p| p.start + p.width);
    if shown_to < total {
        text.push('…');
    }
    let at = caret_at.saturating_sub(start) + left;
    let room_left = (shown_to.saturating_sub(start) + left + 1).saturating_sub(at);
    Drawn {
        text,
        caret_at: at,
        carets: carets.min(room_left.max(1)),
    }
}
