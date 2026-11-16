//! The two modes: run mode gains only the `m kb` unit, each mode keeps its
//! own place, letters keep their meaning, and `f` follows a run to the
//! record its checkpoint names and back. Each test fails when its guarded
//! check goes.
mod support;

use support::kb_app::{kb_app, press};
use support::kb_world::{ACCEPTED, atlas};
use support::projection::{normalize, render, rows, symbols};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Focus, Key, View};
use writfold_tui::kb_state::{KbView, Mode};

fn golden_text(name: &str) -> TestResult<String> {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/pre-t5")
        .join(name);
    Ok(std::fs::read_to_string(file)?)
}

#[test]
fn run_mode_gains_only_the_mode_unit() -> TestResult {
    let w = support::map::nested("t5-run-mode")?;
    // The exact level is the pre-T6 detailed frame, so the pre-T5 detailed
    // golden still differs from it only by the mode unit.
    for (name, width, height) in [("t4-detail-136x38.txt", 136, 38)] {
        let mut app = App::new(w.source(), Some(RUN));
        app.focus = Focus::Detail;
        app.key(Key::Char('d'));
        let mut buffer = render(&mut app, width, height)?;
        support::buffers::stable(&mut buffer)?;
        let screen = rows(&buffer);
        let text = normalize(&w, &symbols(&buffer))?;
        let shown = 1;
        assert_eq!(text.matches("m kb").count(), shown, "{name}: {text}");
        assert_eq!(
            screen[1].contains("m kb"),
            shown == 1,
            "{name}: {}",
            screen[1]
        );
        for kb in ["projects (", "why (", "knowledge base", "m runs"] {
            assert!(!text.contains(kb), "{name}: {kb}");
        }
        let blanked = text.replacen("m kb", "    ", 1);
        let pre: Vec<String> = golden_text(name)?
            .lines()
            .map(|l| l.trim_end().to_owned())
            .collect();
        let now: Vec<String> = blanked.lines().map(|l| l.trim_end().to_owned()).collect();
        assert_eq!(now, pre, "{name}");
    }
    Ok(())
}

#[test]
fn m_round_trips_each_modes_place() -> TestResult {
    let w = atlas("round-trip")?;
    let mut app = App::new(w.source(), Some(RUN));
    render(&mut app, 136, 38)?;
    app.key(Key::Char(']'));
    app.focus = Focus::Detail;
    app.key(Key::Char('x'));
    app.key(Key::Char('j'));
    let run = (
        app.run(),
        app.node.clone(),
        app.view,
        app.map,
        app.scroll,
        app.focus,
    );
    app.key(Key::Char('m'));
    assert_eq!(app.mode, Mode::Kb);
    press(&mut app, "3jj");

    render(&mut app, 136, 38)?;
    let kb = (
        app.kb.view,
        app.kb_entry().map(|e| e.key),
        app.kb.focus,
        app.kb_scope(),
    );
    app.key(Key::Char('m'));
    assert_eq!(app.mode, Mode::Runs);
    assert_eq!(
        (
            app.run(),
            app.node.clone(),
            app.view,
            app.map,
            app.scroll,
            app.focus
        ),
        run
    );
    app.key(Key::Char('m'));
    assert_eq!(
        (
            app.kb.view,
            app.kb_entry().map(|e| e.key),
            app.kb.focus,
            app.kb_scope()
        ),
        kb
    );
    // Another run view, scrolled, survives the round trip too.
    app.key(Key::Char('m'));
    app.key(Key::Char('t'));
    app.key(Key::Char('j'));
    let timeline = (app.view, app.scroll, app.run());
    app.key(Key::Char('m'));
    app.key(Key::Char('m'));
    assert_eq!((app.view, app.scroll, app.run()), timeline);
    Ok(())
}

#[test]
fn letters_keep_their_meaning() -> TestResult {
    let w = atlas("letters")?;
    let mut app = kb_app(&w);
    app.key(Key::Char('3'));
    assert_eq!((app.mode, app.kb.view), (Mode::Kb, KbView::Tracker));
    app.key(Key::Char('t'));
    assert_eq!((app.mode, app.view), (Mode::Runs, View::Timeline));
    app.key(Key::Char('m'));
    app.key(Key::Char('g'));
    assert_eq!((app.mode, app.view), (Mode::Runs, View::Gates));
    app.key(Key::Char('m'));
    for key in ['x', 'X', 'o', 'P', '.'] {
        app.key(Key::Char(key));
        assert_eq!(
            (app.mode, app.kb.view, app.notice.clone()),
            (Mode::Kb, KbView::Tracker, None)
        );
    }
    Ok(())
}

#[test]
fn follow_both_ways() -> TestResult {
    let w = atlas("follow")?;
    let mut app = App::new(w.source(), Some(support::CLOSED));
    render(&mut app, 136, 38)?;
    // A run whose checkpoints name no record.
    app.key(Key::Char('f'));
    assert_eq!(app.mode, Mode::Kb);
    assert_eq!(
        app.notice.as_deref(),
        Some("this run's checkpoints name no record")
    );
    app.key(Key::Char('m'));
    select(&mut app, RUN)?;
    app.key(Key::Char('f'));
    assert_eq!((app.mode, app.kb.view), (Mode::Kb, KbView::Why));
    assert_eq!(app.kb_entry().map(|e| e.id).as_deref(), Some(ACCEPTED));
    // Back: the run named by the checkpoint row, neither the selection
    // nor the first listed run (B, whose checkpoint is newest).
    app.key(Key::Char('m'));
    select(&mut app, support::plan::B)?;
    let first = app.order.first().map(|i| id_at(&app, *i));
    assert_eq!(first.as_deref(), Some(support::plan::B));
    app.key(Key::Char('m'));
    app.key(Key::Char('f'));
    assert_eq!((app.mode, app.view), (Mode::Runs, View::Timeline));
    assert_eq!(app.run().as_deref(), Some(RUN));
    // A record only B's checkpoint names.
    app.key(Key::Char('m'));
    app.kb.subject = Some("harbor-db".into());
    app.kb_show(KbView::Impact);
    app.kb
        .selected
        .insert(KbView::Impact, "sig:fsync-stall".into());
    app.key(Key::Char('f'));
    assert_eq!(app.run().as_deref(), Some(support::plan::B));
    // The naming run filtered out of the list: a notice, nothing changes.
    app.key(Key::Char('P'));
    assert!(app.order.iter().all(|i| id_at(&app, *i) != RUN));
    app.key(Key::Char('m'));
    app.kb_show(KbView::Why);
    app.kb.selected.insert(KbView::Why, ACCEPTED.into());
    app.key(Key::Char('f'));
    assert_eq!((app.mode, app.kb.view), (Mode::Kb, KbView::Why));
    assert_eq!(
        app.notice.as_deref(),
        Some("no listed run names this record")
    );
    Ok(())
}

fn id_at(app: &App, index: usize) -> String {
    app.runs
        .rows
        .get(index)
        .map(|r| writfold_tui::source::field(r, "id").to_owned())
        .unwrap_or_default()
}

fn select(app: &mut App, run: &str) -> TestResult {
    for _ in 0..app.order.len() {
        if app.run().as_deref() == Some(run) {
            return Ok(());
        }
        app.key(Key::Char(']'));
    }
    for _ in 0..app.order.len() {
        if app.run().as_deref() == Some(run) {
            return Ok(());
        }
        app.key(Key::Char('['));
    }
    Err(format!("{run} not listed").into())
}
