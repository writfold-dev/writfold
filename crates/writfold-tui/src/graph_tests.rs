use super::*;
use crate::cells::{Glyphs, chip, segment};
use crate::lines::Tone;
use serde_json::json;

fn step(id: &str, status: &str, group: &str, parent: Option<&str>) -> Value {
    let mut more = vec![json!({"label": "splice", "text": group})];
    if let Some(parent) = parent {
        more.push(json!({"label": "under", "text": parent}));
    }
    json!({"kind": "step", "id": id, "status": status, "more": more})
}

#[test]
fn opaque_ids_follow_postorder_under_links_and_exact_splice_text() {
    let answer = Reply {
        rows: vec![
            step(
                "leaf without separators",
                "done",
                "shared text",
                Some("middle"),
            ),
            step("middle", "current", "shared text", Some("root")),
            step("looks/nested#99/actually-root", "later", "other", None),
            step("root", "later", "shared text", None),
        ],
        ..Reply::default()
    };
    let nodes = nodes(&answer);
    assert_eq!(
        nodes.iter().map(|n| n.depth).collect::<Vec<_>>(),
        [2, 1, 0, 0]
    );
    assert_eq!(nodes[0].parent, Some("middle"));
    assert_eq!(nodes[1].parent, Some("root"));
    assert_eq!(nodes[0].group, nodes[1].group);
    assert_eq!(nodes[1].group, nodes[3].group);
    assert_ne!(nodes[2].group, nodes[3].group);
    assert!(std::ptr::eq(nodes[0].row, &answer.rows[0]));
}

#[test]
fn own_states_preserve_disorder_and_recorded_current() {
    let mut stale = step("stale", "done", "g", None);
    stale["flags"] = json!(["stale"]);
    let mut done = step("late done", "done", "g", None);
    done["flags"] = json!(["out-of-order", "two-plans", "repeated"]);
    let mut invalid = step("invalid", "done", "g", None);
    invalid["flags"] = json!(["invalid", "stale"]);
    let answer = Reply {
        rows: vec![
            stale,
            step("skip", "skipped", "g", None),
            step("current", "current", "g", None),
            done,
            step("later", "later", "g", None),
            invalid,
        ],
        ..Reply::default()
    };
    let nodes = nodes(&answer);
    assert_eq!(
        nodes.iter().map(|n| n.state).collect::<Vec<_>>(),
        [
            State::Stale,
            State::Skipped,
            State::Current,
            State::Done,
            State::Later,
            State::Invalid,
        ]
    );
    assert_eq!(nodes[2].id, "current");
    assert_eq!(nodes[3].flags, ["out-of-order", "two-plans", "repeated"]);
}

#[test]
fn chips_project_status_not_text_and_preserve_gate_order() {
    let answer = Reply {
        rows: vec![json!({"kind": "step", "id": "opaque", "status": "later",
        "text": "later: gates: example", "more": [
            {"label": "gate", "status": "later", "text": "example [open]"},
            {"label": "gate", "status": "open", "text": "example [satisfied]"},
            {"label": "gate", "text": "example [satisfied]"},
            {"label": "item", "status": "supported"},
            {"label": "gate", "status": "undefined"}
        ]})],
        ..Reply::default()
    };
    let nodes = nodes(&answer);
    assert_eq!(nodes[0].chips, ["later", "open", "", "undefined"]);
    assert_eq!(nodes[0].rail_text, "later: gates: example");
    assert_eq!(chip(nodes[0].chips[0], Glyphs::Unicode).glyph, "·");
    assert_eq!(chip(nodes[0].chips[1], Glyphs::Unicode).glyph, "○");
    assert_eq!(chip(nodes[0].chips[2], Glyphs::Unicode).glyph, "?");
    assert_eq!(chip(nodes[0].chips[3], Glyphs::Unicode).glyph, "?");
}

#[test]
fn malformed_links_are_bounded_and_nonsteps_are_not_nodes() {
    let answer = Reply {
        rows: vec![
            step("a", "later", "g", Some("b")),
            step("b", "later", "g", Some("a")),
            step("missing", "later", "g", Some("absent")),
            step("self", "later", "g", Some("self")),
            json!({"kind": "plan-event", "id": "absent"}),
        ],
        ..Reply::default()
    };
    let nodes = nodes(&answer);
    assert_eq!(nodes.len(), 4);
    assert!(nodes.iter().all(|n| n.depth <= nodes.len()));
    assert_eq!(nodes[2].depth, 0);
    assert_eq!(nodes[2].parent, Some("absent"));
}

#[test]
fn done_continuation_and_skip_text_remain_verbatim() {
    let answer = Reply {
        rows: vec![
            json!({"kind": "step", "status": "done", "text": "summary",
            "more": [{"label": "done", "text": "recorded 2026-11-14T01:02:03Z"}]}),
            json!({"kind": "step", "status": "skipped", "text": "reason (by owner, time)"}),
        ],
        ..Reply::default()
    };
    let nodes = nodes(&answer);
    assert_eq!(nodes[0].rail_text, "recorded 2026-11-14T01:02:03Z");
    assert_eq!(nodes[1].rail_text, "reason (by owner, time)");
}

#[test]
fn ascii_step_shapes_are_six_distinct_states() {
    let states = [
        State::Done,
        State::Stale,
        State::Skipped,
        State::Current,
        State::Later,
        State::Invalid,
    ];
    let shapes = states.map(|state| state.cell(Glyphs::Ascii).glyph);
    assert_eq!(shapes, ["[x]", "[!]", "[-]", "[>]", "[ ]", "[#]"]);
    for (index, shape) in shapes.iter().enumerate() {
        assert!(!shapes[..index].contains(shape));
    }
}

#[test]
fn chip_and_segment_shapes_and_tones_are_status_projections() {
    for (status, unicode, ascii, tone) in [
        ("satisfied", "●", "+", Tone::Good),
        ("attested", "◆", "*", Tone::Vouched),
        ("waived", "◆", "*", Tone::Vouched),
        ("open", "○", "o", Tone::Warn),
        ("stale", "!", "!", Tone::Bad),
        ("failed", "×", "x", Tone::Bad),
        ("later", "·", ".", Tone::Dim),
        ("unknown", "?", "?", Tone::Warn),
        ("undefined", "?", "?", Tone::Warn),
    ] {
        assert_eq!(chip(status, Glyphs::Unicode).glyph, unicode);
        assert_eq!(chip(status, Glyphs::Ascii).glyph, ascii);
        assert_eq!(chip(status, Glyphs::Unicode).tone, tone);
    }
    for (status, shape) in [
        ("supported", "■"),
        ("not-applicable", "◆"),
        ("attested", "◆"),
        ("stale", "!"),
        ("open", "□"),
        ("undefined", "?"),
    ] {
        assert_eq!(segment(status, Glyphs::Unicode).glyph, shape);
    }
    // Shapes alone distinguish every judged standing without colour.
    let shapes: std::collections::HashSet<_> = [
        "satisfied",
        "attested",
        "open",
        "stale",
        "failed",
        "later",
        "unknown",
    ]
    .map(|s| chip(s, Glyphs::Ascii).glyph)
    .into_iter()
    .collect();
    assert_eq!(shapes.len(), 7);
}

#[test]
fn a_loop_shows_its_round_spent_and_earlier_rounds_as_history() {
    let mut spent = step("fix#1/converge", "current", "fix", None);
    spent["flags"] = json!(["spent"]);
    spent["loop"] =
        json!({"template": "attempt", "min": 1, "max": 2, "rounds": 2, "exit": "spent"});
    let mut earlier = step(
        "fix#1/converge/attempt#1/change",
        "done",
        "attempt",
        Some("fix#1/converge"),
    );
    earlier["round"] = json!(1);
    earlier["superseded"] = json!(true);
    let answer = Reply {
        rows: vec![earlier, spent],
        ..Reply::default()
    };
    let nodes = nodes(&answer);
    assert_eq!(crate::say_step::chip(&nodes[0]), Some("earlier round"));
    assert_eq!(crate::say_step::chip(&nodes[1]), Some("spent"));
    assert_eq!(
        crate::say_step::round(&nodes[1]).as_deref(),
        Some("round 2 of at most 2")
    );
    assert_eq!(crate::say_step::round(&nodes[0]), None);
    let mut opened = step(
        "fix#1/converge/attempt#2/change",
        "current",
        "attempt",
        None,
    );
    opened["until"] = json!([{"gate": "fixed", "standing": "failed", "evidence": "runs/r/g.json"}]);
    let answer = Reply {
        rows: vec![opened],
        ..Reply::default()
    };
    let nodes = super::nodes(&answer);
    assert_eq!(
        crate::say_step::began(&nodes[0]).as_deref(),
        Some("this round began after fixed failed")
    );
}
