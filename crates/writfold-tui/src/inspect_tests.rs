use super::*;
use crate::app::Detail;
use crate::cells::State;
use serde_json::json;

fn node(row: &Value) -> Node<'_> {
    Node {
        id: field(row, "id"),
        depth: 0,
        group: "",
        parent: None,
        state: State::Done,
        chips: lines::more(row)
            .filter(|value| field(value, "label") == "gate")
            .map(|value| field(value, "status"))
            .collect(),
        rail_text: "",
        flags: Vec::new(),
        row,
    }
}

fn texts(lines: &[InspectLine]) -> Vec<String> {
    lines.iter().map(|line| line.text.to_string()).collect()
}

fn fixture() -> Value {
    json!({"id":"plan#1/check", "status":"done", "text":"review [stale]",
        "path":"runs/r/plan/done-check.json", "line":1, "flags":["stale"], "more":[
        {"label":"why", "text":"no longer holds: review [stale]"},
        {"label":"splice", "text":"plan#1 @abc from plans/plan.toml",
            "path":"runs/r/plan/splice-abc.json", "line":1},
        {"label":"under", "text":"outer#1/check"},
        {"label":"gate", "text":"review [stale] checklist 9/12 required covered",
            "status":"stale", "path":"runs/r/gates/review.json", "line":3},
        {"label":"item", "text":"first [supported] reason mentions (optional)", "status":"supported"},
        {"label":"item", "text":"extra [supported] (optional) covered", "status":"supported"},
        {"label":"item", "text":"second [not-applicable] no hot path", "status":"not-applicable",
            "path":"runs/r/items/second.json", "line":7},
        {"label":"item", "text":"third [stale] evidence moved", "status":"stale", "flags":["stale"]},
        {"label":"item", "text":"fourth [open] needs evidence", "status":"open"},
        {"label":"read", "text":"instructions for review", "path":".writfold/gates/review.md", "line":1},
        {"label":"next", "text":"writfold kb gate cover review third supported REF --by WHO"},
        {"label":"done", "text":"recorded 2026-11-14T07:07:44Z"}
    ]})
}

#[test]
fn preserves_continuations_and_separate_evidence() {
    let row = fixture();
    let output = content(&node(&row), Glyphs::Unicode, Look::default(), Detail::Full);
    let text = texts(&output);
    assert_eq!(text[0], "plan#1/check [done] !stale");
    for value in lines::more(&row) {
        assert!(text.iter().any(|line| line.contains(field(value, "text"))));
        let path = field(value, "path");
        if !path.is_empty() {
            let expected = format!("@{path}:{}", value["line"]);
            assert!(
                output
                    .iter()
                    .any(|line| line.indent > 0 && line.text.to_string() == expected)
            );
        }
    }
    assert!(text.contains(&"@runs/r/plan/done-check.json:1".into()));
    assert_eq!(
        text.last().map(String::as_str),
        Some("d less   g gates   x text")
    );
}

#[test]
fn segments_keep_order_and_source_count_without_judging() {
    let row = fixture();
    for (glyphs, expected) in [(Glyphs::Unicode, "■ ◆ ! □ "), (Glyphs::Ascii, "x * ! . ")] {
        let output = content(&node(&row), glyphs, Look::default(), Detail::Full);
        assert!(
            texts(&output).contains(&format!("9/12 required covered  {expected}(+1 optional)"))
        );
    }
}

#[test]
fn optional_marker_must_be_exact_prefix() {
    for (text, optional_expected) in [
        ("a [open] (optional) explanation", true),
        ("a [open] (optional)", true),
        ("a [open] explanation (optional)", false),
        ("a [supported] (optional) explanation", false),
        ("a [open] (optional)ity", false),
    ] {
        assert_eq!(
            optional(&json!({"text":text, "status":"open"})),
            optional_expected
        );
    }
}

#[test]
fn no_gates_and_unjudged_text_do_not_invent_standing() {
    for text in ["no gates", "gates: review, unit"] {
        let row = json!({"id":"p#1/a", "status":"current", "text":text});
        let output = texts(&content(
            &node(&row),
            Glyphs::Ascii,
            Look::default(),
            Detail::Full,
        ));
        assert_eq!(output[0], "p#1/a [current]");
        assert!(output.contains(&text.to_owned()));
        assert!(!output.iter().any(|line| line.starts_with("gates (")));
        assert!(
            !output
                .iter()
                .any(|line| line.contains("[open]") || line.contains("[later]"))
        );
    }
}

#[test]
fn skipped_event_path_follows_verbatim_reason() {
    let row = json!({"id":"p#1/a", "status":"skipped", "text":"not needed (by owner, DATE)",
        "path":"runs/r/plan/skip-a.json", "line":1});
    let output = texts(&content(
        &node(&row),
        Glyphs::Unicode,
        Look::default(),
        Detail::Full,
    ));
    assert!(output.windows(2).any(|pair| pair
        == [
            "skip: not needed (by owner, DATE)",
            "@runs/r/plan/skip-a.json:1"
        ]));
}

#[test]
fn calm_keeps_what_needs_attention_and_drops_receipts_and_covered_items() {
    let row = fixture();
    let output = content(&node(&row), Glyphs::Unicode, Look::default(), Detail::Calm);
    let text = texts(&output);
    assert_eq!(text[0], "plan#1/check [done] !stale");
    assert_eq!(text[1], "plan#1/check");
    assert!(!text.iter().any(|line| line.starts_with('@')));
    for kept in [
        "gate review [stale] checklist 9/12 required covered",
        "■ ◆ ! □ (+1 optional)",
        "! third [stale] evidence moved",
        "□ fourth [open] needs evidence",
        "read: instructions for review",
        "done: recorded 2026-11-14T07:07:44Z",
    ] {
        assert!(text.contains(&kept.to_owned()), "{kept}: {text:#?}");
    }
    for dropped in [
        "first [supported]",
        "extra [supported]",
        "second [not-applicable]",
    ] {
        assert!(!text.iter().any(|line| line.contains(dropped)), "{dropped}");
    }
    assert!(
        !text
            .iter()
            .any(|l| l.starts_with("splice:") || l.starts_with("under:"))
    );
    assert_eq!(
        text.last().map(String::as_str),
        Some("d receipts and covered items   g gates   x text")
    );
}
