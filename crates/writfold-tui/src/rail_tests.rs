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

/// A loop step `fix#1/converge` in round 3, after two earlier rounds of
/// two steps each, then a step after the loop.
fn looped() -> Reply {
    let step = |id: &str, status: &str, round: Option<u64>| {
        let mut more = vec![json!({"label":"splice","text":"fix#1"})];
        let mut row = json!({"kind":"step","id":id,"status":status});
        if let Some(r) = round {
            more = vec![
                json!({"label":"splice","text":format!("attempt#{r}")}),
                json!({"label":"under","text":"fix#1/converge"}),
            ];
            row["round"] = json!(r);
            row["superseded"] = json!(r < 3);
        }
        row["more"] = json!(more);
        row
    };
    Reply {
        rows: vec![
            step("fix#1/prep", "done", None),
            step("fix#1/converge/attempt#1/change", "done", Some(1)),
            step("fix#1/converge/attempt#1/review", "done", Some(1)),
            step("fix#1/converge/attempt#2/change", "done", Some(2)),
            step("fix#1/converge/attempt#2/review", "done", Some(2)),
            step("fix#1/converge/attempt#3/change", "current", Some(3)),
            step("fix#1/converge", "later", None),
            step("fix#1/ship", "later", None),
        ],
        ..Reply::default()
    }
}

#[test]
fn earlier_rounds_fold_into_one_line_that_opens_on_selection() {
    let answer = looped();
    let nodes = graph::nodes(&answer);
    let show = |selected| Show {
        glyphs: Glyphs::Ascii,
        look: Look::default(),
        full: false,
        selected: Some(selected),
    };
    let rail = build(&nodes, 60, show(5));
    let text: Vec<_> = rail.lines.iter().map(ToString::to_string).collect();
    let fold = rail.node_lines[1];
    assert!(text[fold].contains(".. rounds 1-2: done"), "{text:#?}");
    assert!(
        (1..5).all(|i| rail.node_lines[i] == fold),
        "every folded step points at the fold"
    );
    assert!(
        !text
            .iter()
            .any(|l| l.contains("attempt#1") || l.contains("attempt#2"))
    );
    assert_eq!(
        text.iter().filter(|l| l.contains("+-/")).count(),
        1,
        "{text:#?}"
    );
    assert!(text[rail.node_lines[5]].contains("change"));
    // Selecting a folded step opens the whole run.
    let open = build(&nodes, 60, show(3));
    let lines: std::collections::BTreeSet<_> = open.node_lines.iter().collect();
    assert_eq!(lines.len(), nodes.len());
    let text: Vec<_> = open.lines.iter().map(ToString::to_string).collect();
    assert!(!text.iter().any(|l| l.contains("rounds 1-2")));
    assert_eq!(
        text.iter().filter(|l| l.contains("+-/")).count(),
        3,
        "{text:#?}"
    );
}

#[test]
fn a_fold_says_how_its_rounds_ended() {
    let mut answer = looped();
    answer.rows[2]["status"] = json!("skipped");
    let nodes = graph::nodes(&answer);
    assert_eq!(crate::rail_fold::label(&nodes[1..5]), "rounds 1-2: history");
    assert_eq!(crate::rail_fold::label(&nodes[1..2]), "round 1: done");
    assert_eq!(crate::rail_fold::label(&nodes[2..3]), "round 1: skipped");
}
