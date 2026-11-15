use super::*;
use crate::{graph, source::Reply};
use serde_json::json;

#[test]
fn nested_headers_close_before_their_parent_without_repeating_root() {
    let answer = Reply {
        rows: vec![
            json!({"kind":"step","id":"first","status":"skipped","more":[{"label":"splice","text":"root group"}]}),
            json!({"kind":"step","id":"child","status":"current","more":[{"label":"splice","text":"nested group"},{"label":"under","text":"parent"}]}),
            json!({"kind":"step","id":"parent","status":"done","flags":["out-of-order"],"more":[{"label":"splice","text":"root group"}]}),
            json!({"kind":"step","id":"end","status":"later","more":[{"label":"splice","text":"ending group"}]}),
        ],
        ..Reply::default()
    };
    let nodes = graph::nodes(&answer);
    let show = Show {
        glyphs: Glyphs::Unicode,
        look: Look::default(),
        full: true,
        selected: None,
    };
    let rail = build(&nodes, 50, show);
    let text: Vec<_> = rail.lines.iter().map(ToString::to_string).collect();
    assert_eq!(
        text.iter()
            .filter(|line| line.contains("root group"))
            .count(),
        1
    );
    assert!(text[rail.node_lines[0]].contains('–'));
    assert!(text[rail.node_lines[1]].contains("│ ▶ child"));
    assert!(text[rail.node_lines[2] - 1].contains("├─╯"));
    assert!(text[rail.node_lines[2]].contains("✓ parent"));
    assert!(text[rail.node_lines[2]].ends_with("!out-of-order"));
    assert!(text.iter().any(|line| line.contains("┼ ending group")));
}

#[test]
fn visible_window_keeps_context_beyond_terminal_offset_range() {
    let rail = Rail {
        lines: vec![Line::raw("step"); 70_010],
        node_lines: vec![70_000],
    };
    let range = window(&rail, 0, 9, 0);
    assert!(range.start > usize::from(u16::MAX));
    assert!(range.contains(&70_000));
    assert!(70_000 - range.start >= 2);
    assert!(range.end - 70_000 > 2);
    assert_eq!(window(&rail, 0, 9, range.start), range);
}
