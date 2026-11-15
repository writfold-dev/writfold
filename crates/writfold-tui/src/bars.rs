//! Header, tab bar and footer as segments. A bar is laid out, not overlaid:
//! when it does not fit, whole units are dropped by priority, never cut.
use crate::cells::Glyphs;
use crate::lines::Tone;
use crate::rail_text::{clip, width};
use crate::style::{self, Chip, Depth, Look};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

pub use crate::bar_content::{footer, tabs, title};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Title,
    Chip(Chip),
    Digit,
    Tab,
    Badge(Tone),
    Muted,
    Key,
    Notice,
    Space,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub role: Role,
}

/// Segments drawn together; `drop` 0 is never dropped, higher drops first.
#[derive(Clone, Debug)]
pub struct Unit {
    pub parts: Vec<Segment>,
    pub drop: u8,
}

impl Unit {
    pub fn new(parts: Vec<Segment>, drop: u8) -> Self {
        Self { parts, drop }
    }

    pub fn one(text: impl Into<String>, role: Role, drop: u8) -> Self {
        Self::new(vec![segment(text, role)], drop)
    }

    fn chip_at(&self, end: bool) -> bool {
        let part = if end {
            self.parts.last()
        } else {
            self.parts.first()
        };
        part.is_some_and(|p| matches!(p.role, Role::Chip(_)))
    }
}

pub fn segment(text: impl Into<String>, role: Role) -> Segment {
    Segment {
        text: text.into(),
        role,
    }
}

/// A chip's text with its padding or brackets, the same width at every depth.
pub fn chip(look: Look, text: &str, kind: Chip) -> Segment {
    segment(style::chip_text(look, text), Role::Chip(kind))
}

/// Words sit `gap` cells apart; a chip's own padding counts as one.
fn joined(units: &[Unit], gap: usize) -> Vec<Segment> {
    let mut out = Vec::new();
    for (at, unit) in units.iter().enumerate() {
        if at > 0 {
            let pads = usize::from(units[at - 1].chip_at(true)) + usize::from(unit.chip_at(false));
            out.push(segment(
                " ".repeat(gap.saturating_sub(pads).max(1)),
                Role::Space,
            ));
        }
        out.extend(unit.parts.iter().cloned());
    }
    out
}

fn span_width(parts: &[Segment]) -> usize {
    parts.iter().map(|p| width(&p.text)).sum()
}

/// The bar's width with every unit kept, margins included.
pub fn total(left: &[Unit], right: &[Unit], gap: usize) -> usize {
    let (l, r) = (
        span_width(&joined(left, gap)),
        span_width(&joined(right, gap)),
    );
    let between = if r > 0 && l > 0 { gap } else { 0 };
    2 + l + r + between
}

/// Drop the highest-priority unit (rightmost on ties) until the bar fits;
/// a unit with priority 0 stays, and only a lone title is ever clipped.
pub fn fit(
    (mut left, mut right): (Vec<Unit>, Vec<Unit>),
    gap: usize,
    room: usize,
    glyphs: Glyphs,
) -> Vec<Segment> {
    while total(&left, &right, gap) > room {
        let candidate = |units: &[Unit]| {
            units
                .iter()
                .enumerate()
                .filter(|(_, u)| u.drop > 0)
                .max_by_key(|(at, u)| (u.drop, *at))
                .map(|(at, u)| (u.drop, at))
        };
        match (candidate(&left), candidate(&right)) {
            (Some((l, at)), r) if r.is_none_or(|(r, _)| l >= r) => drop(left.remove(at)),
            (_, Some((_, at))) => drop(right.remove(at)),
            _ => break,
        }
    }
    let over = total(&left, &right, gap).saturating_sub(room);
    if over > 0
        && let Some(title) = left.first_mut().and_then(|u| u.parts.first_mut())
    {
        title.text = clip(&title.text, width(&title.text).saturating_sub(over), glyphs);
    }
    let left = joined(&left, gap);
    let right = joined(&right, gap);
    let fill = room.saturating_sub(2 + span_width(&left) + span_width(&right));
    let mut out = vec![segment(" ", Role::Space)];
    out.extend(left);
    out.push(segment(" ".repeat(fill), Role::Space));
    out.extend(right);
    out.push(segment(" ", Role::Space));
    out
}

pub fn role_style(look: Look, role: Role) -> Style {
    match role {
        Role::Title => style::style(look, Tone::Head),
        Role::Chip(kind) => style::chip(look, kind),
        Role::Digit if look.depth == Depth::Truecolor => {
            Style::default().fg(style::style(look, Tone::Current).fg.unwrap_or_default())
        }
        Role::Digit => style::style(look, Tone::Head),
        Role::Tab => style::style(look, Tone::Done),
        Role::Badge(tone) => style::style(look, tone),
        Role::Muted => style::style(look, Tone::Dim),
        Role::Key => style::style(look, Tone::Key),
        Role::Notice => style::style(look, Tone::Warn),
        Role::Space => Style::default(),
    }
}

pub fn line(parts: Vec<Segment>, look: Look) -> Line<'static> {
    Line::from(
        parts
            .into_iter()
            .map(|p| Span::styled(p.text, role_style(look, p.role)))
            .collect::<Vec<_>>(),
    )
}
