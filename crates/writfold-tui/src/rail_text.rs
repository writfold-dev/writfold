//! Rail text measured in cells: shown ids, the id column and clipping.
//! Presentation only; ids stay opaque and verb wording is never rewritten.
use crate::cells::Glyphs;
use crate::graph::Node;
use ratatui::text::Span;

pub(crate) fn mark(glyphs: Glyphs, unicode: &'static str, ascii: &'static str) -> &'static str {
    if glyphs == Glyphs::Unicode {
        unicode
    } else {
        ascii
    }
}

pub(crate) fn width(text: &str) -> usize {
    Span::raw(text).width()
}

/// Keep the end of `text`, marking the cut at the start.
pub(crate) fn left(text: &str, room: usize, glyphs: Glyphs) -> String {
    if width(text) <= room {
        return text.into();
    }
    let marker = mark(glyphs, "…", "...");
    let available = room.saturating_sub(width(marker));
    let mut taken = 0;
    let mut start = text.len();
    let mut bytes = [0; 4];
    for (at, ch) in text.char_indices().rev() {
        let size = width(ch.encode_utf8(&mut bytes));
        if taken + size > available {
            break;
        }
        taken += size;
        start = at;
    }
    format!("{marker}{}", &text[start..])
}

/// Keep the start of `text`, marking the cut at the end.
pub(crate) fn clip(text: &str, room: usize, glyphs: Glyphs) -> String {
    if width(text) <= room {
        return text.into();
    }
    let marker = mark(glyphs, "…", "...");
    let available = room.saturating_sub(width(marker));
    if available == 0 {
        return String::new();
    }
    let mut taken = 0;
    let mut end = 0;
    let mut bytes = [0; 4];
    for (at, ch) in text.char_indices() {
        let size = width(ch.encode_utf8(&mut bytes));
        if taken + size > available {
            break;
        }
        taken += size;
        end = at + ch.len_utf8();
    }
    format!("{}{marker}", &text[..end])
}

/// The splice name: the first word of the group's `splice` text.
pub(crate) fn splice_name(group: &str) -> &str {
    group.split(' ').next().unwrap_or_default()
}

/// Inside a group the id is shown from the group's own splice name; the
/// header above already names the ancestors. Calm also drops that name,
/// which the header itself shows, when the id starts with it.
pub(crate) fn shown_id<'a>(id: &'a str, group: &str, full: bool) -> &'a str {
    let name = splice_name(group);
    if name.is_empty() {
        return id;
    }
    let prefix = format!("{name}/");
    let from_name = if id.starts_with(&prefix) {
        id
    } else {
        id.rfind(&format!("/{prefix}"))
            .map_or(id, |at| &id[at + 1..])
    };
    if full {
        return from_name;
    }
    from_name
        .strip_prefix(&prefix)
        .filter(|rest| !rest.is_empty())
        .unwrap_or(from_name)
}

/// The widest id as shown (plus its depth rail), capped so rail text keeps
/// room; ids are aligned to the deepest one.
pub(crate) fn id_column(nodes: &[Node<'_>], full: bool) -> usize {
    nodes
        .iter()
        .map(|n| width(shown_id(n.id, n.group, full)) + 2 * n.depth)
        .max()
        .unwrap_or(0)
        .min(36)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_truncation_preserves_display_width_and_terminal_glyph_set() {
        assert_eq!(left("prefix/last", 5, Glyphs::Unicode), "…last");
        assert_eq!(left("prefix/last", 7, Glyphs::Ascii), "...last");
        assert_eq!(left("prefix/界界", 5, Glyphs::Unicode), "…界界");
    }

    #[test]
    fn clip_counts_cells_and_marks_the_cut() {
        assert_eq!(clip("findings#1 @6ec8", 10, Glyphs::Unicode), "findings#…");
        assert_eq!(clip("findings#1 @6ec8", 10, Glyphs::Ascii), "finding...");
        assert_eq!(clip("界界界", 5, Glyphs::Unicode), "界界…");
        assert_eq!(clip("short", 5, Glyphs::Unicode), "short");
        assert_eq!(clip("long", 0, Glyphs::Unicode), "");
    }

    #[test]
    fn calm_ids_drop_the_group_name_only_as_a_leading_component() {
        let group = "findings#1 @6ec817d3031a from plans/findings.toml";
        let id = "change#1/review/review#1/findings/findings#1/check";
        assert_eq!(shown_id(id, group, true), "findings#1/check");
        assert_eq!(shown_id(id, group, false), "check");
        assert_eq!(shown_id("other/check", group, false), "other/check");
        assert_eq!(shown_id("findings#1/", group, false), "findings#1/");
        assert_eq!(shown_id("plain", "", false), "plain");
    }
}
