//! Shapes and tones project verb statuses without judging obligations.
use crate::lines::Tone;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyphs {
    Unicode,
    Ascii,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub glyph: &'static str,
    pub tone: Tone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Done,
    Stale,
    Skipped,
    Current,
    Later,
    Invalid,
}

impl State {
    pub fn cell(self, glyphs: Glyphs) -> Cell {
        let (unicode, ascii, tone) = match self {
            Self::Done => ("✓", "[x]", Tone::Good),
            Self::Stale => ("!", "[!]", Tone::Bad),
            Self::Skipped => ("–", "[-]", Tone::Skipped),
            Self::Current => ("▶", "[>]", Tone::Warn),
            Self::Later => ("○", "[ ]", Tone::Dim),
            Self::Invalid => ("×", "[#]", Tone::Bad),
        };
        cell(glyphs, unicode, ascii, tone)
    }
}

fn cell(glyphs: Glyphs, unicode: &'static str, ascii: &'static str, tone: Tone) -> Cell {
    Cell {
        glyph: match glyphs {
            Glyphs::Unicode => unicode,
            Glyphs::Ascii => ascii,
        },
        tone,
    }
}

pub fn chip(status: &str, glyphs: Glyphs) -> Cell {
    let (unicode, ascii, tone) = match status {
        "satisfied" => ("●", "o", Tone::Good),
        "attested" | "waived" => ("◆", "*", Tone::Vouched),
        "open" => ("○", "o", Tone::Warn),
        "stale" => ("!", "!", Tone::Bad),
        "failed" => ("×", "x", Tone::Bad),
        "later" => ("·", ".", Tone::Dim),
        _ => ("?", "?", Tone::Warn),
    };
    cell(glyphs, unicode, ascii, tone)
}

pub fn segment(status: &str, glyphs: Glyphs) -> Cell {
    let (unicode, ascii, tone) = match status {
        "supported" => ("■", "x", Tone::Good),
        "not-applicable" | "attested" => ("◆", "*", Tone::Vouched),
        "stale" => ("!", "!", Tone::Bad),
        "open" => ("□", ".", Tone::Warn),
        _ => ("?", "?", Tone::Warn),
    };
    cell(glyphs, unicode, ascii, tone)
}
