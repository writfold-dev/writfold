//! The impact tree against the verb's own edges, the walk's limits on
//! screen, and the stale badge against `stale --cached`.
mod support;

use ratatui::style::Color;
use serde_json::Value;
use support::TestResult;
use support::kb_app::{kb_app, press};
use support::kb_world::atlas;
use support::projection::{render, rows, symbols};
use writfold_kb::Verb;
use writfold_kb::verbs::impact::Direction;
use writfold_tui::kb_state::KbView;
use writfold_tui::lines::Tone;
use writfold_tui::style::{Depth, Look};

fn json(w: &support::World, verb: Verb) -> TestResult<Value> {
    Ok(serde_json::from_str(&w.verb(verb, true)?.0)?)
}

fn rows_of(body: &Value) -> Vec<Value> {
    body["rows"].as_array().cloned().unwrap_or_default()
}

#[test]
fn tree_parents_follow_the_edge() -> TestResult {
    let w = atlas("tree")?;
    let mut app = kb_app(&w);
    app.kb.subject = Some("harbor-db".into());
    app.kb_show(KbView::Impact);
    let body = json(
        &w,
        Verb::Impact {
            target: "harbor-db".into(),
            depth: 2,
            direction: Direction::In,
            inferred: false,
        },
    )?;
    let parent_of = |id: &str| -> Option<String> {
        rows_of(&body).iter().find(|r| r["id"] == id).map(|r| {
            let edge = &r["edge"];
            let other = if edge["from"] == id {
                &edge["to"]
            } else {
                &edge["from"]
            };
            other.as_str().unwrap_or_default().to_owned()
        })
    };
    let entries = app.kb_entries();
    let mut top = String::from("harbor-db");
    let mut count = 0;
    for entry in entries.iter().filter(|e| !e.header) {
        count += 1;
        if entry.depth == 0 {
            assert_eq!(
                parent_of(&entry.id).as_deref(),
                Some("harbor-db"),
                "{}",
                entry.id
            );
            top = entry.id.clone();
        } else {
            assert_eq!(parent_of(&entry.id), Some(top.clone()), "{}", entry.id);
        }
    }
    assert_eq!(Some(count as u64), body["trailer"]["shown"].as_u64());
    Ok(())
}

#[test]
fn walk_limits_are_on_screen() -> TestResult {
    let w = atlas("limits")?;
    let mut app = kb_app(&w);
    app.look = Look {
        depth: Depth::Truecolor,
        ..Look::default()
    };
    app.kb.subject = Some("harbor-db".into());
    app.kb_show(KbView::Impact);
    let buffer = render(&mut app, 136, 38)?;
    let screen = rows(&buffer);
    let line = "14 records reached · not followed: 1 disputed link,";
    let y = screen.iter().position(|r| r.contains(line)).ok_or(line)?;
    let x = screen[y].chars().position(|c| c == '1').ok_or("x")?;
    let warn = writfold_tui::style::style(app.look, Tone::Warn).fg;
    assert_eq!(Some(buffer[(x as u16, y as u16)].fg), warn);
    assert_ne!(warn, Some(Color::Reset));
    Ok(())
}

#[test]
fn stale_badge_equals_the_verb() -> TestResult {
    let w = atlas("badge")?;
    let mut app = kb_app(&w);
    let body = json(
        &w,
        Verb::Stale {
            scope: None,
            offline: false,
            cached: true,
        },
    )?;
    let stale = body["trailer"]["counts"]["stale"].as_u64().ok_or("stale")?;
    let screen = rows(&render(&mut app, 136, 38)?);
    assert!(
        screen[1].contains(&format!("5 stale {stale} moved")),
        "{}",
        screen[1]
    );
    assert!(!screen[1].contains(" ?"), "{}", screen[1]);
    press(&mut app, "5");
    let text = symbols(&render(&mut app, 136, 38)?);
    assert!(text.contains(&format!("stale ({stale})")), "{text}");
    Ok(())
}
