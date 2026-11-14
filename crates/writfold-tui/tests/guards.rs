mod support;

use support::plan::{B, world};
use support::{CLOSED, RUN, TestResult};
use writfold_kb::{Verb, guard::GuardVerb};
use writfold_tui::app::{App, Key, View};
use writfold_tui::frame;
use writfold_tui::source::field;

fn text(app: &App) -> String {
    frame::detail(app, View::Guards, None, true, true)
        .lines
        .into_iter()
        .map(|l| l.text)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn guards_show_the_current_run_even_when_a_complete_run_is_selected() -> TestResult {
    let w = world("guards-current")?;
    let mut app = App::new(w.source(), Some(CLOSED));
    app.show(View::Guards);
    assert_eq!(app.current_run(), Some(RUN));
    assert_eq!(app.run_id(), Some(CLOSED));
    let answer = app.answer(View::Guards).ok_or("guards")?;
    let (direct, _) = w.verb(Verb::Guard(GuardVerb::Status), true)?;
    let direct: serde_json::Value = serde_json::from_str(&direct)?;
    assert_eq!(Some(&answer.rows), direct["rows"].as_array());
    assert_eq!(answer.count("deny"), Some(1));
    assert_eq!(answer.count("nudge"), Some(1));
    let lines = text(&app);
    assert!(
        lines.starts_with(&format!(
            "about the current run {RUN}, not the selected run"
        )),
        "{lines}"
    );
    assert!(
        lines.contains("would-deny") && lines.contains("owes:"),
        "{lines}"
    );
    Ok(())
}

#[test]
fn filtered_lists_do_not_lose_guard_context_and_pointer_changes_refresh_it() -> TestResult {
    let w = world("guards-filter")?;
    let mut app = App::new(w.source(), Some(B));
    app.key(Key::Char('P'));
    assert!(app.runs.rows.iter().all(|r| field(r, "id") == B));
    app.show(View::Guards);
    assert_eq!(app.current_run(), Some(RUN));
    assert!(
        frame::detail(&app, View::Guards, None, true, true)
            .title
            .contains(RUN)
    );
    std::fs::remove_file(w.kb.join(".git/writfold/current-run"))?;
    assert!(app.poll());
    assert_eq!(app.current_run(), None);
    assert_eq!(app.run_id(), Some(B));
    assert!(text(&app).contains("current run none"));
    app.project_only = Some("absent-project".into());
    app.key(Key::Char('r'));
    assert!(app.runs.rows.is_empty());
    assert!(app.answer(View::Guards).is_some());
    assert!(text(&app).contains("would-deny"));
    Ok(())
}

#[test]
fn no_rule_directory_is_an_explicit_empty_guard_view() -> TestResult {
    let w = support::world("guards-empty")?;
    let mut app = App::new(w.source(), None);
    let shown = frame::once(&mut app, Some(View::Guards));
    assert!(shown.contains("== guards (current run none) =="), "{shown}");
    assert!(shown.contains("(no guard rules)"), "{shown}");
    let answer = app.answer(View::Guards).ok_or("guards")?;
    assert_eq!(answer.exit, 0);
    assert_eq!(answer.trailer_text("reason").as_deref(), Some("no-guards"));
    Ok(())
}
