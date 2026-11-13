//! `writfold watch` over the watch world: the `--once` frame (golden),
//! the terminal layout (ratatui's test backend), the keys, and that the
//! projection shows exactly the states and counts the verbs print.
mod support;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use std::collections::BTreeMap;
use support::projection::golden;
use support::{CLOSED, RUN, TestResult, stable, world};
use writfold_kb::Verb;
use writfold_kb::gate::GateVerb;
use writfold_tui::app::{App, Focus, Key, View};
use writfold_tui::source::field;
use writfold_tui::style::Environment;

#[test]
fn once_prints_the_frame() -> TestResult {
    let w = world("once")?;
    let text = stable(&writfold_tui::once(w.source(), None, Some(View::Resume)));
    golden("watch-once.txt", &text)?;
    // Open runs come first although the closed run is newer.
    let open = text.find(&format!("[open] {RUN}")).ok_or("no open run")?;
    let closed = text
        .find(&format!("[closed] {CLOSED}"))
        .ok_or("no closed run")?;
    assert!(open < closed, "{text}");
    assert!(!text.contains("running") && !text.contains('%'), "{text}");
    Ok(())
}

fn render(app: &mut App) -> TestResult<ratatui::buffer::Buffer> {
    let mut terminal = Terminal::new(TestBackend::new(160, 40))?;
    terminal.draw(|frame| writfold_tui::view::draw(frame, app))?;
    Ok(terminal.backend().buffer().clone())
}

/// Every row of the buffer as text.
fn rows(buffer: &ratatui::buffer::Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect()
}

/// Where `needle` starts on screen.
fn find(rows: &[String], needle: &str) -> Option<(u16, u16)> {
    rows.iter().enumerate().find_map(|(y, row)| {
        row.find(needle).map(|byte| {
            let x = row[..byte].chars().count();
            (
                u16::try_from(x).unwrap_or_default(),
                u16::try_from(y).unwrap_or_default(),
            )
        })
    })
}

#[test]
fn the_layout_has_runs_left_gates_right_and_the_pin_below() -> TestResult {
    let w = world("layout")?;
    let mut app = App::new(w.source(), None);
    app.show(View::Gates);
    let buffer = render(&mut app)?;
    let screen = rows(&buffer);
    let text = screen.join("\n");
    let (runs_x, _) = find(&screen, " runs (2) ").ok_or(text.clone())?;
    let (gates_x, _) = find(&screen, &format!(" gates {RUN}")).ok_or(text.clone())?;
    assert!(runs_x < 10 && gates_x > 30, "{text}");
    let (x, y) = find(&screen, "> [open] run-").ok_or(text.clone())?;
    assert!(find(&screen, "[closed] run-20261104").is_some_and(|(_, c)| c > y));
    assert!(x < gates_x);
    let summary = "gates: gates=4 satisfied=1 open=1 stale=1 failed=1";
    let (x, y) = find(&screen, summary).ok_or(text.clone())?;
    assert_eq!(buffer[(x, y)].fg, Color::Red, "stale and failed are red");
    let (x, y) = find(&screen, "gate fail [failed]").ok_or(text.clone())?;
    assert_eq!(buffer[(x, y)].fg, Color::Red);
    let (x, y) = find(&screen, "gate pass [satisfied]").ok_or(text.clone())?;
    assert_eq!(buffer[(x, y)].fg, Color::Green);
    let last = screen.last().ok_or("empty screen")?;
    assert!(last.contains("kb=") && last.contains("index="), "{last}");
    assert!(app.key(Key::Char('?')));
    let screen = rows(&render(&mut app)?);
    assert!(find(&screen, " help ").is_some(), "{}", screen.join("\n"));
    Ok(())
}

#[test]
fn keys_only_move_the_view() -> TestResult {
    let w = world("keys")?;
    let mut app = App::new(w.source(), Some(CLOSED));
    assert_eq!(app.run().as_deref(), Some(CLOSED));
    assert!(app.key(Key::Char('k')));
    assert_eq!(app.run().as_deref(), Some(RUN));
    assert!(app.key(Key::Up));
    assert_eq!(app.run().as_deref(), Some(RUN), "stops at the top");
    assert!(app.key(Key::Char('s')));
    assert_eq!(app.view, View::Stale);
    let stale = app.detail().ok_or("stale not asked")?;
    assert!(stale.rows.iter().any(|r| field(r, "kind") == "cite"));
    assert!(app.key(Key::Char('b')));
    assert_eq!(app.view, View::Brief);
    let brief = app.detail().ok_or("brief not asked")?;
    assert!(
        brief
            .rows
            .iter()
            .any(|r| field(r, "id") == "project:harbor")
    );
    assert!(app.key(Key::Enter));
    assert_eq!(app.focus, Focus::Detail);
    assert!(app.key(Key::Char('j')));
    assert_eq!((app.scroll, app.run().as_deref()), (1, Some(RUN)));
    assert!(!app.poll(), "the pin has not moved");
    std::fs::write(w.kb.join("harbor/NOTES.md"), "# Notes\n")?;
    assert!(app.poll(), "a new KB file moves the pin");
    assert_eq!(app.run().as_deref(), Some(RUN), "the selection is kept");
    assert!(!app.key(Key::Char('q')));
    let (status, _) = w.verb(
        Verb::Runs {
            open: false,
            project: None,
            plan: false,
        },
        false,
    )?;
    assert!(
        status.contains(&format!("run {CLOSED} [closed]")),
        "nothing was written"
    );
    Ok(())
}

/// `gate <id> [<state>]` per row of a text answer.
fn states(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|l| l.strip_prefix("gate "))
        .filter_map(|l| {
            let (id, rest) = l.split_once(" [")?;
            Some((id.to_owned(), rest.split(']').next()?.to_owned()))
        })
        .collect()
}

fn run_projection(w: &support::World, app: &App) -> TestResult {
    let (runs, _) = w.verb(
        Verb::Runs {
            open: false,
            project: None,
            plan: true,
        },
        true,
    )?;
    let runs: serde_json::Value = serde_json::from_str(&runs)?;
    assert_eq!(Some(&app.runs.rows), runs["rows"].as_array());
    let mut order = app.order.clone();
    order.sort_unstable();
    assert_eq!(order, (0..app.runs.rows.len()).collect::<Vec<_>>());
    let (text, _) = w.verb(
        Verb::Runs {
            open: false,
            project: None,
            plan: true,
        },
        false,
    )?;
    let listed: Vec<&str> = text.lines().filter(|l| l.starts_with("run ")).collect();
    for row in &app.runs.rows {
        let line = format!("run {} [{}]", field(row, "id"), field(row, "status"));
        assert!(
            listed.iter().any(|l| l.starts_with(&line)),
            "{line}\n{text}"
        );
    }
    Ok(())
}

#[test]
fn the_projection_equals_the_verbs() -> TestResult {
    let w = world("projection")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.show(View::Resume);
    app.load_view(View::Gates);
    run_projection(&w, &app)?;
    // Gates: the run pane's rows have the states `gate status` prints, and
    // the summary has its counts.
    let resume = app.detail().ok_or("resume not asked")?;
    let shown: BTreeMap<String, String> = resume
        .rows
        .iter()
        .filter(|r| field(r, "kind") == "gate")
        .map(|r| (field(r, "id").to_owned(), field(r, "status").to_owned()))
        .collect();
    let status = Verb::Gate(GateVerb::Status {
        run: Some(RUN.into()),
        repo: None,
    });
    let (printed, exit) = w.verb(status, false)?;
    assert_eq!(exit, 2, "{printed}");
    assert_eq!(shown, states(&printed));
    assert_eq!(shown.len(), 4);
    let trailer = printed.lines().last().ok_or("no trailer")?;
    let summary = writfold_tui::side::gate_summary(app.gate_status().ok_or("no gates")?);
    for part in summary.text.trim_start_matches("gates: ").split(' ') {
        assert!(
            trailer.contains(&format!(" {part} ")),
            "{part} in {trailer}"
        );
    }
    let (resumed, _) = w.verb(
        Verb::Resume {
            run: Some(RUN.into()),
        },
        false,
    )?;
    assert_eq!(states(&resumed), shown);
    Ok(())
}

#[test]
fn a_pipe_gets_one_plain_frame() -> TestResult {
    let w = world("pipe")?;
    let args = ["--kb".to_owned(), w.kb.display().to_string()];
    let frame = writfold_tui::main(&args, w.kb.clone(), false, Environment::default())?
        .stdout
        .ok_or("no frame")?;
    assert!(
        frame.starts_with("== runs (2 listed, 1 not closed) =="),
        "{frame}"
    );
    assert!(!frame.contains('\u{1b}'), "plain text, no terminal codes");
    let bad = writfold_tui::main(
        &["--write".to_owned()],
        w.kb.clone(),
        false,
        Environment::default(),
    );
    assert_eq!(bad, Err("unknown argument --write".to_owned()));
    let bad = writfold_tui::parse(&["--view".into(), "missing".into()])
        .err()
        .ok_or("unknown view accepted")?;
    for view in View::ALL {
        assert!(bad.contains(view.name()), "{bad}");
    }
    Ok(())
}
