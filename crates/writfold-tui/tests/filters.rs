mod support;

use serde_json::Value;
use support::plan::{B, world};
use support::{CLOSED, RUN, TestResult, World};
use writfold_kb::Verb;
use writfold_tui::app::{App, Key};
use writfold_tui::source::field;

fn same_runs(w: &World, app: &App, open: bool, project: Option<&str>) -> TestResult {
    let (text, _) = w.verb(
        Verb::Runs {
            open,
            project: project.map(str::to_owned),
            plan: true,
        },
        true,
    )?;
    let answer: Value = serde_json::from_str(&text)?;
    assert_eq!(Some(&app.runs.rows), answer["rows"].as_array());
    let mut order = app.order.clone();
    order.sort_unstable();
    assert_eq!(order, (0..app.runs.rows.len()).collect::<Vec<_>>());
    Ok(())
}

#[test]
fn filters_are_kb_arguments_and_titles_show_them() -> TestResult {
    let w = world("filters")?;
    let mut app = App::new(w.source(), Some(RUN));
    same_runs(&w, &app, false, None)?;
    let unfiltered = app.runs_title();
    app.key(Key::Char('o'));
    same_runs(&w, &app, true, None)?;
    assert!(app.open_only);
    assert_ne!(app.runs_title(), unfiltered);
    assert!(!app.runs.rows.iter().any(|r| field(r, "id") == CLOSED));
    app.key(Key::Char('P'));
    same_runs(&w, &app, true, Some("harbor"))?;
    assert_eq!(app.project_only.as_deref(), Some("harbor"));
    assert!(app.runs_title().contains("harbor"));
    app.key(Key::Char('o'));
    same_runs(&w, &app, false, Some("harbor"))?;
    app.key(Key::Char('P'));
    same_runs(&w, &app, false, None)?;
    assert_eq!(app.run_id(), Some(RUN));
    assert_eq!(app.runs_title(), unfiltered);
    Ok(())
}

#[test]
fn project_filter_without_a_project_is_a_noticed_noop() -> TestResult {
    let w = world("no-project")?;
    let path = w.kb.join(format!("runs/{B}/manifest.json"));
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    manifest
        .as_object_mut()
        .ok_or("manifest")?
        .remove("project");
    std::fs::write(path, serde_json::to_vec(&manifest)?)?;
    let mut app = App::new(w.source(), Some(B));
    let rows = app.runs.rows.clone();
    app.key(Key::Char('P'));
    assert!(app.project_only.is_none());
    assert!(app.notice.is_some());
    assert_eq!(app.runs.rows, rows);
    assert_eq!(app.run_id(), Some(B));
    Ok(())
}
