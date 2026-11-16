use super::{Ranking, merge};
use crate::emit::{self, Answer, Frame, Output, Row, Scope};
use crate::index::{IndexState, Pin};
use crate::semantic::Hit;
use serde_json::Value;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn answer(score: f32, last: u32) -> Answer {
    let hits = ["a", "b"]
        .into_iter()
        .map(|id| {
            let row = Row {
                kind: "asset",
                id: id.into(),
                ..Row::default()
            };
            let hit = Hit {
                id: id.into(),
                path: "inventory/assets/a.md".into(),
                first: 3,
                last,
                score,
            };
            (row, hit)
        })
        .collect();
    merge(Answer::default(), hits, Ranking::Lexical)
}

fn frame() -> Frame {
    Frame {
        verb: "find".into(),
        args: vec!["meaning".into(), "semantic=lexical-first".into()],
        budget: 1000,
        cursor: None,
        json: true,
    }
}

fn render(answer: &Answer, frame: &Frame) -> crate::error::Result<Output> {
    let pin = Pin {
        head: None,
        dirty: 0,
        digest: "0".repeat(64),
    };
    emit::emit(
        answer,
        frame,
        &Scope {
            pin: &pin,
            index: IndexState::Fresh,
            examined: 2,
            total: 2,
            unreadable: 0,
        },
    )
}

#[test]
fn text_and_json_report_scores_spans_and_non_evidence() -> TestResult {
    let answer = answer(0.125, 9);
    let json: Value = serde_json::from_str(&render(&answer, &frame())?.text)?;
    assert_eq!(json["rows"][0]["matched_on"], "semantic");
    assert_eq!(json["rows"][0]["semantic"]["score"], 0.125);
    assert_eq!(json["rows"][0]["semantic"]["first"], 3);
    assert_eq!(json["rows"][0]["semantic"]["last"], 9);
    assert_eq!(json["rows"][0]["semantic"]["path"], "inventory/assets/a.md");
    let mut text_frame = frame();
    text_frame.json = false;
    let text = render(&answer, &text_frame)?.text;
    assert!(text.contains("semantic: score=0.125 span=3-9 pointer-only; not evidence"));
    assert!(text.contains("@inventory/assets/a.md:3"));
    for row in &answer.rows {
        assert!(
            row.more
                .iter()
                .all(|more| more.label != "cite" && more.label != "evidence")
        );
        assert!(row.flags.is_empty());
        assert!(row.edge.is_none());
        assert!(row.status.is_none());
    }
    Ok(())
}

#[test]
fn cursors_bind_search_mode_score_and_span() -> TestResult {
    let answer = answer(0.125, 9);
    let mut frame = frame();
    frame.budget = emit::render_row(&answer.rows[0])
        .split('\n')
        .map(|line| (line.len() + 1).div_ceil(4))
        .sum();
    let first: Value = serde_json::from_str(&render(&answer, &frame)?.text)?;
    frame.cursor = Some(first["trailer"]["cursor"].as_str().ok_or("cursor")?.into());
    let second: Value = serde_json::from_str(&render(&answer, &frame)?.text)?;
    assert_eq!(second["rows"][0]["id"], "b");
    assert!(render(&self::answer(0.126, 9), &frame).is_err());
    assert!(render(&self::answer(0.125, 10), &frame).is_err());
    frame.args[1] = "semantic=rrf".into();
    assert!(render(&answer, &frame).is_err());
    frame.args.pop();
    assert!(render(&answer, &frame).is_err());
    Ok(())
}
