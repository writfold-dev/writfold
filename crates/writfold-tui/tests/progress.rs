mod support;

use serde_json::Value;
use support::plan::{B, DECISION, world};
use support::projection::{golden, normalize, verb};
use support::{CLOSED, RUN, TestResult};
use writfold_kb::Verb;
use writfold_tui::app::{App, View};
use writfold_tui::source::{field, flags};
use writfold_tui::{frame, lines};

#[test]
fn progress_once_goldens() -> TestResult {
    let w = world("once")?;
    let mut app = App::new(w.source(), Some(RUN));
    let text = normalize(&w, &frame::once(&mut app, None))?;
    golden("progress-once.txt", &text)?;
    let mut previous = 0;
    for section in ["runs", "progress", "gates", "inbox", "guards", "timeline"] {
        let at = text.find(&format!("== {section} ")).ok_or(section)?;
        assert!(at >= previous, "{text}");
        previous = at;
    }
    for forbidden in [
        "\u{1b}",
        "%",
        "ETA",
        "running",
        "in progress",
        "active",
        " ago",
        "asked ",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}: {text}");
    }
    assert!(text.contains("[x] [!] [-] [>] [ ] [ ]"));
    let text = normalize(&w, &frame::once(&mut app, Some(View::Gates)))?;
    golden("progress-once-view-gates.txt", &text)?;
    assert!(text.contains("== gates "));
    for absent in ["== progress ", "== inbox ", "== guards ", "== timeline "] {
        assert!(!text.contains(absent));
    }
    Ok(())
}

#[test]
fn fixture_records_exact_step_item_and_count_facts() -> TestResult {
    let w = world("facts")?;
    let mut app = App::new(w.source(), Some(RUN));
    assert_eq!(app.current_run(), Some(RUN));
    assert_eq!(app.view, View::Progress);
    let answer = app.detail().ok_or("progress")?;
    let steps: Vec<_> = answer
        .rows
        .iter()
        .filter(|r| field(r, "kind") == "step")
        .collect();
    assert_eq!(
        steps.iter().map(|r| field(r, "status")).collect::<Vec<_>>(),
        ["done", "done", "skipped", "current", "later", "later"]
    );
    assert_eq!(flags(steps[1]), ["stale"]);
    let items: Vec<_> = lines::more(steps[3])
        .filter(|m| field(m, "label") == "item")
        .collect();
    assert_eq!(
        items.iter().map(|m| field(m, "status")).collect::<Vec<_>>(),
        ["supported", "stale", "open", "not-applicable", "open"]
    );
    assert!(field(items[4], "text").contains("optional"));
    for (key, count) in [
        ("done", 2),
        ("skipped", 1),
        ("steps", 6),
        ("current", 4),
        ("current_gates", 2),
        ("current_owed", 2),
        ("stale", 1),
    ] {
        assert_eq!(answer.count(key), Some(count), "{key}");
    }
    let text = content(&app, true);
    assert!(
        text.contains("plan 2 done, 1 skipped of 6 steps; current step 4/6; owes 2 of 2 gates")
    );
    related_panes(&mut app)
}

fn related_panes(app: &mut App) -> TestResult {
    app.load_view(View::Timeline);
    let timeline = app.answer(View::Timeline).ok_or("timeline")?;
    assert_eq!(timeline.count("checkpoints"), Some(3));
    assert!(timeline.rows.iter().any(|r| field(r, "id") == DECISION));
    app.load_view(View::Inbox);
    let inbox = app.answer(View::Inbox).ok_or("inbox")?;
    for (kind, count) in [("draft", 1), ("proposal", 2)] {
        assert_eq!(
            inbox
                .rows
                .iter()
                .filter(|r| field(r, "kind") == kind)
                .count(),
            count
        );
    }
    Ok(())
}

fn content(app: &App, expanded: bool) -> String {
    frame::detail(app, View::Progress, None, expanded, true)
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn block<'a>(text: &'a str, start: &str, next: &str) -> TestResult<&'a str> {
    let (_, after) = text.split_once(start).ok_or("step head")?;
    Ok(after.split_once(next).ok_or("next step")?.0)
}

#[test]
fn collapsed_history_keeps_flagged_steps_fully_expanded() -> TestResult {
    let w = world("collapse")?;
    let app = App::new(w.source(), Some(RUN));
    let collapsed = content(&app, false);
    let expanded = content(&app, true);
    let clean = block(&collapsed, "change#1/research", "change#1/implement")?;
    assert!(clean.contains("done: recorded "));
    assert!(!clean.contains("gate notes"));
    assert!(block(&expanded, "change#1/research", "change#1/implement")?.contains("gate notes"));
    let flagged = block(&collapsed, "change#1/implement", "change#1/benchmarks")?;
    assert_eq!(
        flagged,
        block(&expanded, "change#1/implement", "change#1/benchmarks")?
    );
    for label in ["why:", "gate unit", "gate lint", "done:"] {
        assert!(flagged.contains(label));
    }
    assert!(!block(&collapsed, "change#1/deliver", "change#1/reflect")?.contains("gate unit"));
    assert!(block(&expanded, "change#1/deliver", "change#1/reflect")?.contains("gate unit"));
    Ok(())
}

fn projected_rows(view: View, rows: &[Value]) -> impl Iterator<Item = &Value> {
    rows.iter()
        .filter(move |r| view != View::Inbox || matches!(field(r, "kind"), "draft" | "proposal"))
}

#[test]
fn every_run_and_view_projects_its_json_and_text_answer() -> TestResult {
    let w = world("projection")?;
    for run in [RUN, B, CLOSED] {
        let mut app = App::new(w.source(), Some(run));
        for view in View::ALL {
            app.show(view);
            let command = verb(view, run, app.project().as_deref());
            let (json, exit) = w.verb(command.clone(), true)?;
            let expected: Value = serde_json::from_str(&json)?;
            let answer = app.answer(view).ok_or("view answer")?;
            assert_eq!(Some(&answer.rows), expected["rows"].as_array());
            assert_eq!(answer.trailer["counts"], expected["trailer"]["counts"]);
            assert_eq!(answer.exit, exit);
            let (text, _) = w.verb(command, false)?;
            let shown = frame::detail(&app, view, None, true, true)
                .lines
                .iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            for row in projected_rows(view, &answer.rows) {
                assert!(text.contains(field(row, "text")), "{run} {view:?}: {text}");
                assert!(
                    shown.contains(field(row, "text")),
                    "{run} {view:?}: {shown}"
                );
                for key in ["id", "status"] {
                    assert!(shown.contains(field(row, key)));
                }
                for flag in flags(row) {
                    assert!(shown.contains(&format!("!{flag}")));
                }
                for more in lines::more(row) {
                    assert!(
                        shown.contains(field(more, "text")),
                        "{view:?}: {more}\n{shown}"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn run_positions_and_unplanned_fallback_keep_verb_facts() -> TestResult {
    let w = world("positions")?;
    let mut app = App::new(w.source(), Some(B));
    let (json, _) = w.verb(
        Verb::Runs {
            open: false,
            project: None,
            plan: true,
        },
        true,
    )?;
    let expected: Value = serde_json::from_str(&json)?;
    assert_eq!(Some(&app.runs.rows), expected["rows"].as_array());
    assert_eq!(app.runs.count("planned"), Some(2));
    for row in &app.runs.rows {
        let id = field(row, "id");
        let position = lines::more(row).find(|m| field(m, "label") == "plan");
        if id == B {
            assert!(position.is_none());
            continue;
        }
        let position = field(position.ok_or("position")?, "text");
        let answer = app.source.plan(id);
        for key in ["done", "skipped"] {
            assert!(position.contains(&format!("{key}={}", answer.count(key).ok_or("count")?)));
        }
        let listed = lines::run_item(row)
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(listed.contains(position));
        if id == CLOSED {
            assert!(position.starts_with("complete steps=2 done=2 skipped=0"));
        }
    }
    app.load();
    let fallback = content(&app, true);
    assert!(fallback.contains("has no plan"));
    let gates = app.gate_status().ok_or("fallback gates")?;
    assert_eq!(gates.count("gates"), Some(6));
    for row in &gates.rows {
        assert!(fallback.contains(field(row, "text")));
    }
    Ok(())
}
