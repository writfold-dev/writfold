mod support;
use support::map;
use support::projection::render;
use support::{NOW, RUN, TestResult};
use writfold_tui::app::{App, Focus, Key, View};
use writfold_tui::graph;
use writfold_tui::source::field;

fn ids(app: &App) -> TestResult<Vec<String>> {
    Ok(
        graph::nodes(app.answer(View::Progress).ok_or("plan answer")?)
            .iter()
            .map(|n| n.id.to_owned())
            .collect(),
    )
}
fn current(app: &App) -> TestResult<String> {
    graph::nodes(app.answer(View::Progress).ok_or("plan answer")?)
        .iter()
        .find(|n| field(n.row, "status") == "current")
        .map(|n| n.id.to_owned())
        .ok_or_else(|| "current node".into())
}

#[test]
fn selection_survives_reask_and_insert_then_falls_back() -> TestResult {
    let w = map::long("selection")?;
    let mut app = App::new(w.source(), Some(RUN));
    render(&mut app, 100, 30)?;
    app.focus = Focus::Detail;
    let original = current(&app)?;
    assert_eq!(app.node.as_deref(), Some(original.as_str()));
    let order = ids(&app)?;
    let index = order.iter().position(|id| id == &original).ok_or("index")?;
    app.key(Key::Char('j'));
    assert_eq!(app.node.as_ref(), order.get(index + 1));
    app.key(Key::Char('k'));
    assert_eq!(app.node.as_deref(), Some(original.as_str()));
    app.key(Key::Char('k'));
    let kept = app.node.clone();
    app.key(Key::Char('r'));
    assert_eq!(app.node, kept);
    map::use_template(&w, "insert", NOW - 5)?;
    app.key(Key::Char('r'));
    assert_eq!(ids(&app)?.len(), 41);
    assert_eq!(app.node, kept);
    app.node = Some("not-a-recorded-step".into());
    app.key(Key::Char('r'));
    assert_eq!(app.node, Some(current(&app)?));
    app.key(Key::Char('k'));
    app.key(Key::Char('.'));
    assert_eq!(app.node, Some(current(&app)?));
    Ok(())
}

#[test]
fn polling_an_insert_preserves_noncurrent_selection() -> TestResult {
    let w = map::long("poll-selection")?;
    let mut app = App::new(w.source(), Some(RUN));
    render(&mut app, 100, 30)?;
    app.focus = Focus::Detail;
    app.key(Key::Char('k'));
    let kept = app.node.clone();
    app.poll();
    map::use_template(&w, "insert", NOW - 5)?;
    let changed = (0..5).fold(false, |changed, _| app.poll() || changed);
    assert!(changed);
    assert_eq!(ids(&app)?.len(), 41);
    assert_eq!(app.node, kept);
    Ok(())
}

#[test]
fn flags_wrap_in_both_directions_and_dot_ignores_stale_done() -> TestResult {
    for w in [map::nested("flags")?, map::out_of_order("flags")?] {
        let mut app = App::new(w.source(), Some(RUN));
        render(&mut app, 160, 40)?;
        app.focus = Focus::Detail;
        let flagged: Vec<String> = graph::nodes(app.detail().ok_or("plan")?)
            .iter()
            .filter(|n| !n.flags.is_empty())
            .map(|n| n.id.into())
            .collect();
        assert!(!flagged.is_empty());
        app.node = flagged.last().cloned();
        for id in &flagged {
            app.key(Key::Char('n'));
            assert_eq!(app.node.as_ref(), Some(id));
        }
        app.node = flagged.first().cloned();
        for id in flagged.iter().rev() {
            app.key(Key::Char('N'));
            assert_eq!(app.node.as_ref(), Some(id));
        }
        app.key(Key::Char('.'));
        assert_eq!(app.node, Some(current(&app)?));
    }
    Ok(())
}
