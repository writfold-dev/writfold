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

#[test]
fn audit_and_waived_rules_show_their_flags() -> TestResult {
    let w = world("guards-flags")?;
    let rules = w.kb.join(".writfold/guards");
    let commit = rules.join("commit-needs-gates.toml");
    let text_of = std::fs::read_to_string(&commit)?;
    std::fs::write(&commit, format!("{text_of}mode = \"audit\"\n"))?;
    std::fs::write(
        rules.join("bench-waived.toml"),
        "schema = \"writfold-guard-rule/1\"\nid = \"bench-waived\"\nevent = \"pre-tool-use\"\ncommands = [\"git push\"]\nrequire = [\"gate:bench\"]\naction = \"deny\"\n",
    )?;
    let say = writfold_kb::gate::Say {
        kind: writfold_kb::gate::statement::Kind::Waived,
        id: "bench".into(),
        text: "bench is not run here".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.kb.clone()),
    };
    let (out, exit) = w.verb(Verb::Gate(writfold_kb::gate::GateVerb::Say(say)), false)?;
    assert_eq!(exit, 0, "{out}");
    let mut app = App::new(w.source(), Some(RUN));
    app.show(View::Guards);
    let lines = text(&app);
    assert!(lines.contains("!audit"), "{lines}");
    assert!(lines.contains("!waived"), "{lines}");
    assert!(lines.contains("audit="), "{lines}");
    Ok(())
}
