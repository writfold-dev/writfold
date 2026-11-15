//! The calm default level: the map and inspector carry no receipts, hashes
//! or timestamps, text follows the marker and the cursor, and `d` brings
//! every omitted fact back. Each test fails when its guarded check goes.
mod support;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use support::projection::{render, rows, symbols};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Detail, Focus, Key, View};
use writfold_tui::cells::Glyphs;
use writfold_tui::source::field;
use writfold_tui::style::Look;
use writfold_tui::{frame, graph, inspect, layout, lines};

fn panel(buffer: &Buffer, area: Rect) -> Vec<String> {
    rows(buffer)
        .iter()
        .skip(usize::from(area.y))
        .take(usize::from(area.height))
        .map(|row| {
            row.chars()
                .skip(usize::from(area.x))
                .take(usize::from(area.width))
                .collect()
        })
        .collect()
}

fn areas(width: u16, height: u16) -> TestResult<(Rect, Rect)> {
    let areas = layout::focused(width, height, true, true);
    Ok((areas.main, areas.inspector.ok_or("inspector")?))
}

/// Receipt, hash and timestamp shapes the calm map and inspector must not show.
fn forbidden(map: &[String], inspector: &[String]) -> TestResult<Vec<String>> {
    let hash = regex::Regex::new(r" @[0-9a-f]{12}")?;
    let mut found = Vec::new();
    for row in map {
        if row.contains("recorded 20") || hash.is_match(row) || row.contains(".writfold/plans/") {
            found.push(row.clone());
        }
    }
    for row in inspector {
        if row.trim_start_matches('│').trim_start().starts_with('@') {
            found.push(row.clone());
        }
    }
    Ok(found)
}

fn inspected(width: u16, height: u16, w: &support::World) -> App {
    let mut app = App::new(w.source(), Some(RUN));
    if layout::class(width, height) == layout::Class::Wide {
        app.focus = Focus::Inspector;
    } else {
        app.focus = Focus::Detail;
    }
    app
}

#[test]
fn default_map_rows_carry_no_timestamps_hashes_or_paths() -> TestResult {
    let w = support::map::nested("calm-default")?;
    for (width, height) in [(136, 38), (160, 40)] {
        let mut app = inspected(width, height, &w);
        let buffer = render(&mut app, width, height)?;
        let (map, inspector) = areas(width, height)?;
        let found = forbidden(&panel(&buffer, map), &panel(&buffer, inspector))?;
        assert!(found.is_empty(), "{width}x{height}: {found:#?}");
    }
    Ok(())
}

#[test]
fn d_restores_every_default_omission() -> TestResult {
    let w = support::map::nested("calm-restore")?;
    let mut app = inspected(160, 40, &w);
    let calm = render(&mut app, 160, 40)?;
    app.key(Key::Char('d'));
    let full = render(&mut app, 160, 40)?;
    let (map, inspector) = areas(160, 40)?;
    let (map, inspector) = (panel(&full, map), panel(&full, inspector));
    let text = symbols(&full);
    let hash = regex::Regex::new(r" @[0-9a-f]{12}")?;
    assert!(map.iter().any(|row| row.contains("recorded 20")), "{text}");
    assert!(map.iter().any(|row| hash.is_match(row)), "{text}");
    assert!(
        map.iter().any(|row| row.contains(".writfold/plans/")),
        "{text}"
    );
    assert!(
        inspector
            .iter()
            .any(|row| row.trim_start_matches('│').trim_start().starts_with('@')),
        "{text}"
    );
    for word in ["last checkpoint:", "splice:", "under:"] {
        assert!(text.contains(word), "{word}: {text}");
    }
    app.key(Key::Char('d'));
    assert_eq!(
        render(&mut app, 160, 40)?,
        calm,
        "a second d returns to calm"
    );
    Ok(())
}

/// Map rows that carry rail text: the verb text a row would show at `d`.
fn texted(app: &App, buffer: &Buffer, width: u16, height: u16) -> TestResult<Vec<String>> {
    let (map, _) = areas(width, height)?;
    let map = panel(buffer, map);
    let mut out = Vec::new();
    for node in graph::nodes(app.answer(View::Progress).ok_or("plan")?) {
        let tail = node.id.rsplit('/').next().unwrap_or(node.id);
        let row = map
            .iter()
            .find(|row| row.contains(&format!(" {tail} ")))
            .ok_or("node row")?;
        let start: String = node.rail_text.chars().take(8).collect();
        if !start.is_empty() && row.contains(&start) {
            out.push(node.id.to_owned());
        }
    }
    Ok(out)
}

#[test]
fn text_follows_current_and_cursor() -> TestResult {
    let w = support::map::nested("calm-text")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.focus = Focus::Detail;
    let buffer = render(&mut app, 160, 40)?;
    let current = graph::nodes(app.answer(View::Progress).ok_or("plan")?)
        .iter()
        .find(|n| field(n.row, "status") == "current")
        .map(|n| n.id.to_owned())
        .ok_or("current")?;
    assert_eq!(texted(&app, &buffer, 160, 40)?, vec![current.clone()]);
    app.key(Key::Char('j'));
    let selected = app.node.clone().ok_or("selection")?;
    assert_ne!(selected, current);
    let buffer = render(&mut app, 160, 40)?;
    assert_eq!(texted(&app, &buffer, 160, 40)?, vec![current, selected]);
    Ok(())
}

#[test]
fn nothing_is_lost() -> TestResult {
    let w = support::map::nested("calm-lost")?;
    let mut app = App::new(w.source(), Some(RUN));
    render(&mut app, 160, 40)?;
    app.load_view(View::Gates);
    let mut elsewhere: Vec<String> = Vec::new();
    for view in [View::Gates, View::Progress] {
        let content = frame::detail(&app, view, Some(70), true, false);
        elsewhere.extend(content.lines.into_iter().map(|l| l.text));
    }
    for node in graph::nodes(app.answer(View::Progress).ok_or("plan")?) {
        let mut shown = elsewhere.clone();
        let detailed = inspect::content(&node, Glyphs::Unicode, Look::default(), Detail::Full);
        shown.extend(detailed.iter().map(|l| l.text.to_string()));
        let shown = shown.join("\n");
        for value in std::iter::once(node.row).chain(lines::more(node.row)) {
            let text = field(value, "text");
            assert!(shown.contains(text), "{}: {text}", node.id);
            let path = field(value, "path");
            assert!(
                path.is_empty() || shown.contains(&format!("@{path}")),
                "{path}"
            );
        }
    }
    Ok(())
}

#[test]
fn calm_run_line_is_a_prefix_of_the_verb_plan_text() -> TestResult {
    let w = support::map::nested("calm-runs")?;
    let app = App::new(w.source(), Some(RUN));
    let mut planned = 0;
    for row in &app.runs.rows {
        let calm = lines::run_item(row, Detail::Calm);
        assert_eq!(calm[0].text, field(row, "text"));
        let Some(plan) = lines::more(row).find(|m| field(m, "label") == "plan") else {
            continue;
        };
        let words = calm[1].text.rsplit("  ").next().ok_or("plan words")?;
        assert!(field(plan, "text").starts_with(words), "{words}");
        assert!(!words.contains('='), "{words}");
        planned += 1;
    }
    assert!(planned >= 2, "fixture has planned runs");
    Ok(())
}

#[test]
fn tab_badges_equal_verb_counts() -> TestResult {
    let w = support::map::nested("calm-badges")?;
    let mut app = App::new(w.source(), Some(RUN));
    let buffer = render(&mut app, 160, 40)?;
    let tabs = &rows(&buffer)[1];
    let gates = app.answer(View::Gates).ok_or("gates")?;
    let tab = |name: &str, next: &str| -> TestResult<String> {
        let start = tabs.find(name).ok_or("tab")?;
        let end = tabs.find(next).ok_or("next tab")?;
        Ok(tabs[start..end].trim().to_owned())
    };
    let mut expected = String::from("2 gates");
    for (name, mark) in [
        ("stale", "!"),
        ("failed", "×"),
        ("open", "○"),
        ("unknown", "?"),
    ] {
        if let Some(n) = gates.count(name).filter(|n| *n > 0) {
            expected.push_str(&format!(" {mark}{n}"));
        }
    }
    assert_eq!(tab("2 gates", "3 timeline")?, expected);
    let inbox = app.answer(View::Inbox).ok_or("inbox")?;
    let total = inbox
        .rows
        .iter()
        .filter(|r| matches!(field(r, "kind"), "draft" | "proposal"))
        .count();
    let expected = if total > 0 {
        format!("4 inbox {total}")
    } else {
        "4 inbox".into()
    };
    assert_eq!(tab("4 inbox", "5 guards")?, expected);
    let guards = app.answer(View::Guards).ok_or("guards")?;
    let mut expected = String::from("5 guards");
    for name in ["deny", "ask"] {
        if let Some(n) = guards.count(name).filter(|n| *n > 0) {
            expected.push_str(&format!(" {name} {n}"));
        }
    }
    assert_eq!(tab("5 guards", "6 stale")?, expected);
    assert!(
        !tabs.contains("ask 0") && !tabs.contains("deny 0"),
        "{tabs}"
    );
    Ok(())
}
