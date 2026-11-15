mod support;

use support::map;
use support::projection::{local_snapshot, render, rows, snapshot};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Focus, Key, View};
use writfold_tui::cells::State;
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
fn long_map_keeps_selection_and_two_neighbor_rows_visible() -> TestResult {
    let w = map::long("viewport")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.focus = Focus::Detail;
    // Group-relative ids, so whole ids can be matched on screen.
    app.level = writfold_tui::app::Detail::Full;
    let order = ids(&app)?;
    for key in [Key::Char('j'), Key::Char('k')] {
        for _ in 0..25 {
            app.key(key);
            let buffer = render(&mut app, 100, 30)?;
            let selected = app.node.as_ref().ok_or("selection")?;
            let index = order.iter().position(|id| id == selected).ok_or("index")?;
            let area = writfold_tui::layout::focused(100, 30, true, true).main;
            let lines: Vec<String> = rows(&buffer)
                .iter()
                .map(|line| {
                    line.chars()
                        .skip(usize::from(area.x))
                        .take(usize::from(area.width))
                        .collect()
                })
                .collect();
            let y = lines
                .iter()
                .position(|line| line.contains(selected))
                .ok_or("visible node")?;
            for distance in 1..=2 {
                if let Some(before) = index.checked_sub(distance) {
                    assert!(y >= distance && lines[y - distance].contains(&order[before]));
                }
                if let Some(after) = order.get(index + distance) {
                    assert!(
                        lines
                            .get(y + distance)
                            .is_some_and(|line| line.contains(after))
                    );
                }
            }
        }
    }
    app.key(Key::Char('.'));
    assert_eq!(app.node, Some(current(&app)?));
    Ok(())
}

#[test]
fn focus_keys_follow_three_panes_at_every_breakpoint() -> TestResult {
    let w = map::nested("focus")?;
    for (width, height) in [(160, 40), (100, 30), (72, 20)] {
        for (right, left) in [
            (Key::Char('l'), Key::Char('h')),
            (Key::Right, Key::Left),
            (Key::Enter, Key::Esc),
        ] {
            let mut app = App::new(w.source(), Some(RUN));
            render(&mut app, width, height)?;
            if width < 90 {
                assert_eq!(app.focus, Focus::Detail);
                app.key(Key::Char('0'));
                assert_eq!(app.focus, Focus::Runs);
            }
            app.key(right);
            assert_eq!(app.focus, Focus::Detail);
            app.key(right);
            assert_eq!(app.focus, Focus::Inspector);
            app.key(Key::Char('j'));
            assert_eq!(app.inspect_scroll, 1);
            app.key(Key::Char('k'));
            assert_eq!(app.inspect_scroll, 0);
            app.key(left);
            assert_eq!(app.focus, Focus::Detail);
            app.key(left);
            assert_eq!(app.focus, Focus::Runs);
            if width < 90 {
                assert!(app.runs_view);
            }
        }
    }
    Ok(())
}

#[test]
fn map_text_expansion_and_unbound_c_are_independent() -> TestResult {
    let w = map::nested("toggle")?;
    let mut app = App::new(w.source(), Some(RUN));
    render(&mut app, 160, 40)?;
    app.focus = Focus::Detail;
    assert!(app.map_visible());
    let selected = app.node.clone();
    app.key(Key::Char('X'));
    assert!(!app.expand_all);
    app.key(Key::Char('x'));
    assert!(!app.map_visible());
    app.key(Key::Char('X'));
    assert!(app.expand_all);
    app.key(Key::Char('x'));
    assert!(app.map_visible());
    let before = support::projection::symbols(&render(&mut app, 160, 40)?);
    app.key(Key::Char('c'));
    assert_eq!(
        support::projection::symbols(&render(&mut app, 160, 40)?),
        before
    );
    assert_eq!(app.node, selected);
    Ok(())
}

#[test]
fn disorder_nodes_project_each_own_state_not_position() -> TestResult {
    let w = map::out_of_order("own-state")?;
    let app = App::new(w.source(), Some(RUN));
    let answer = app.detail().ok_or("plan")?;
    let nodes = graph::nodes(answer);
    let current = nodes
        .iter()
        .position(|n| field(n.row, "status") == "current")
        .ok_or("current")?;
    assert!(
        nodes[..current]
            .iter()
            .any(|n| field(n.row, "status") == "skipped")
    );
    assert!(
        nodes[current + 1..]
            .iter()
            .any(|n| field(n.row, "status") == "done")
    );
    for node in nodes {
        let expected = match field(node.row, "status") {
            "done" if node.flags.contains(&"stale") => State::Stale,
            "done" => State::Done,
            "skipped" => State::Skipped,
            "current" => State::Current,
            "later" => State::Later,
            other => return Err(format!("unexpected fixture state: {other}").into()),
        };
        assert_eq!(node.state, expected, "{}", node.id);
    }
    Ok(())
}

#[test]
fn new_key_tour_keeps_kb_digest_and_local_records_identical() -> TestResult {
    let w = map::nested("read-only")?;
    let mut app = App::new(w.source(), Some(RUN));
    let kb = snapshot(&w.kb)?;
    let local = local_snapshot(&w)?;
    let status = support::projection::git_status(&w)?;
    for size in [(160, 40), (100, 30), (72, 20)] {
        render(&mut app, size.0, size.1)?;
        for key in [
            Key::Char('0'),
            Key::Char('l'),
            Key::Char('j'),
            Key::Char('k'),
            Key::Char('n'),
            Key::Char('N'),
            Key::Char('.'),
            Key::Right,
            Key::Char('j'),
            Key::Char('k'),
            Key::Left,
            Key::Enter,
            Key::Esc,
            Key::Char('h'),
            Key::Char('l'),
            Key::Char('x'),
            Key::Char('X'),
            Key::Char('x'),
            Key::Char('c'),
            Key::Char('?'),
            Key::Esc,
            Key::Char('r'),
        ] {
            assert!(app.key(key));
            render(&mut app, size.0, size.1)?;
        }
    }
    assert_eq!(snapshot(&w.kb)?, kb);
    assert_eq!(local_snapshot(&w)?, local);
    assert_eq!(support::projection::git_status(&w)?, status);
    Ok(())
}
