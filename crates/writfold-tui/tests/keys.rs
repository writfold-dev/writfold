mod support;

use support::plan::{B, world};
use support::projection::{local_snapshot, render, snapshot};
use support::{CLOSED, RUN, TestResult};
use writfold_tui::app::{App, Focus, Key, View};

#[test]
fn every_letter_digit_and_tab_reaches_its_view() -> TestResult {
    let w = world("view-keys")?;
    let mut app = App::new(w.source(), Some(RUN));
    assert_eq!(app.view, View::Progress);
    assert_eq!(View::ALL.len(), 8);
    for view in View::ALL {
        for key in [view.key(), view.digit()] {
            app.show(View::Progress);
            assert!(app.key(Key::Char(key)));
            assert_eq!(app.view, view);
            assert!(app.detail().is_some());
        }
    }
    app.show(View::Progress);
    for view in View::ALL.into_iter().skip(1).chain([View::Progress]) {
        app.key(Key::Tab);
        assert_eq!(app.view, view);
    }
    for view in View::ALL.into_iter().rev() {
        app.key(Key::BackTab);
        assert_eq!(app.view, view);
    }
    app.key(Key::Char('g'));
    assert_eq!(app.view, View::Gates);
    app.key(Key::Char('e'));
    assert_eq!(app.view, View::Resume);
    Ok(())
}

#[test]
fn focus_arrows_brackets_and_pages_follow_control_semantics() -> TestResult {
    let w = world("navigation")?;
    let mut app = App::new(w.source(), Some(RUN));
    render(&mut app, 100, 30)?;
    for focus in [Focus::Runs, Focus::Detail] {
        app.focus = focus;
        app.key(Key::Char(']'));
        assert_eq!(app.run_id(), Some(B));
        app.key(Key::Char('['));
        assert_eq!(app.run_id(), Some(RUN));
    }
    app.focus = Focus::Runs;
    app.key(Key::Down);
    assert_eq!(app.run_id(), Some(B));
    app.key(Key::Char('k'));
    assert_eq!(app.run_id(), Some(RUN));
    app.key(Key::Enter);
    assert_eq!(app.focus, Focus::Detail);
    app.scroll = 0;
    app.key(Key::Char('j'));
    assert_eq!(app.scroll, 1);
    app.key(Key::Up);
    assert_eq!(app.scroll, 0);
    for focus in [Focus::Runs, Focus::Detail] {
        app.focus = focus;
        app.key(Key::PageDown);
        assert!(app.scroll > 1);
        app.key(Key::PageUp);
        assert_eq!(app.scroll, 0);
        assert_eq!(app.run_id(), Some(RUN));
    }
    app.key(Key::Esc);
    assert_eq!(app.focus, Focus::Runs);
    Ok(())
}

#[test]
fn expansion_help_and_unbound_letters_do_not_change_records() -> TestResult {
    let w = world("toggles")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.key(Key::Char('x'));
    assert!(app.expand_all);
    app.key(Key::Char('x'));
    assert!(!app.expand_all);
    app.show(View::Gates);
    app.key(Key::Char('x'));
    assert!(!app.expand_all);
    for key in ['a', 'c', 'd', 'f', 'h', 'l', 'm', 'n', 'v', 'w', 'y', 'z'] {
        let before = (app.run(), app.view, app.focus, app.scroll, app.runs.clone());
        assert!(app.key(Key::Char(key)));
        assert_eq!(
            before,
            (app.run(), app.view, app.focus, app.scroll, app.runs.clone())
        );
    }
    app.key(Key::Char('?'));
    assert!(app.help);
    app.key(Key::Char('p'));
    assert!(!app.help);
    assert_eq!(app.view, View::Gates, "help consumes its closing key");
    app.key(Key::Char('?'));
    assert!(!app.key(Key::Char('q')));
    Ok(())
}

#[test]
fn refresh_reasks_the_selected_view() -> TestResult {
    let w = world("refresh-key")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.show(View::Timeline);
    let path = w.kb.join(format!("runs/{RUN}/checkpoint.md"));
    let text = std::fs::read_to_string(&path)?;
    std::fs::write(
        path,
        format!("{text}\n## 2026-11-11T00:00:00Z\n\nA new recorded checkpoint.\n"),
    )?;
    assert_eq!(
        app.detail().ok_or("timeline")?.count("checkpoints"),
        Some(3)
    );
    app.key(Key::Char('r'));
    assert_eq!(app.run_id(), Some(RUN));
    assert_eq!(app.view, View::Timeline);
    assert_eq!(
        app.detail()
            .ok_or("refreshed timeline")?
            .count("checkpoints"),
        Some(4)
    );
    Ok(())
}

#[test]
fn full_key_tour_leaves_kb_and_local_records_byte_identical() -> TestResult {
    let w = world("read-only")?;
    let mut app = App::new(w.source(), Some(RUN));
    let kb = snapshot(&w.kb)?;
    let local = local_snapshot(&w)?;
    let status = support::projection::git_status(&w)?;
    for run in [RUN, B, CLOSED] {
        app = App::new(w.source(), Some(run));
        for view in View::ALL {
            app.key(Key::Char(view.key()));
            for (width, height) in [(160, 40), (100, 30), (70, 20)] {
                render(&mut app, width, height)?;
            }
            writfold_tui::frame::once(&mut app, Some(view));
        }
        for key in ['x', 'x', 'o', 'o', 'P', 'P', 'c', '?', '?', 'r', 'r', 'r'] {
            assert!(app.key(Key::Char(key)));
        }
        for key in [
            Key::Enter,
            Key::Down,
            Key::Up,
            Key::PageDown,
            Key::PageUp,
            Key::Char('['),
            Key::Char(']'),
            Key::Tab,
            Key::BackTab,
            Key::Esc,
        ] {
            assert!(app.key(key));
        }
    }
    assert_eq!(snapshot(&w.kb)?, kb);
    assert_eq!(local_snapshot(&w)?, local);
    assert_eq!(support::projection::git_status(&w)?, status);
    assert!(!app.key(Key::Char('q')));
    Ok(())
}

#[test]
fn help_names_the_json_verbs_and_poll_intervals() {
    let help = writfold_tui::frame::help()
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for command in [
        "writfold kb runs --plan [--open] [--project P] --json",
        "writfold kb plan status --run R --gates --json",
        "writfold kb gate status --run R --json",
        "writfold kb timeline R --json",
        "writfold kb resume R --json",
        "writfold kb guard status --json",
        "writfold kb stale P --cached --json",
        "writfold kb brief P --json",
    ] {
        assert!(help.contains(command), "{command}: {help}");
    }
    for view in View::ALL {
        assert!(help.contains(&format!("{}/{} {}", view.digit(), view.key(), view.name())));
    }
    assert!(help.contains("2 s") && help.contains("10 s"));
    assert!(help.contains("c is unbound"));
}
