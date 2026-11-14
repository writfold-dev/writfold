mod support;

use support::map;
use support::projection::{golden, normalize, render, rows, symbols};
use support::{RUN, TestResult};
use writfold_tui::app::Key;
use writfold_tui::app::{App, Focus, View};
use writfold_tui::source::field;
use writfold_tui::{graph, lines};

fn honest(text: &str) -> TestResult {
    // Whole words deliberately allow the required phrase "liveness claim".
    let forbidden = regex::Regex::new(
        r"(?i)\b(running|working|active|idle|stalled|live|in\s+progress|elapsed|relative|ago)\b|\d+(?:\.\d+)?\s*%",
    )?;
    assert!(
        !forbidden.is_match(text),
        "forbidden language in frame: {text}"
    );
    Ok(())
}

#[test]
fn nested_and_disorder_map_buffer_goldens() -> TestResult {
    for (name, w) in [
        ("nested", map::nested("frames")?),
        ("out-of-order", map::out_of_order("frames")?),
    ] {
        for (width, height) in [(160, 40), (100, 30), (72, 20)] {
            let mut app = App::new(w.source(), Some(RUN));
            app.focus = Focus::Detail;
            let mut buffer = render(&mut app, width, height)?;
            support::buffers::stable(&mut buffer)?;
            let text = normalize(&w, &symbols(&buffer))?;
            honest(&text)?;
            golden(&format!("t3-{name}-{width}x{height}.txt"), &text)?;
        }
    }
    Ok(())
}

fn checklist(app: &App) -> TestResult<String> {
    graph::nodes(app.answer(View::Progress).ok_or("plan")?)
        .iter()
        .find(|node| lines::more(node.row).any(|line| field(line, "label") == "item"))
        .map(|node| node.id.to_owned())
        .ok_or_else(|| "checklist node".into())
}

#[test]
fn selected_checklist_inspector_buffer_golden() -> TestResult {
    let w = map::nested("inspector-frame")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.node = Some(checklist(&app)?);
    app.focus = Focus::Inspector;
    let mut buffer = render(&mut app, 160, 40)?;
    support::buffers::stable(&mut buffer)?;
    let text = normalize(&w, &symbols(&buffer))?;
    honest(&text)?;
    assert!(text.contains("required covered"));
    golden("t3-checklist-inspector-160x40.txt", &text)?;
    Ok(())
}

#[test]
fn checklist_segments_do_not_scale_with_inspector_width() -> TestResult {
    let w = map::nested("segments")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.node = Some(checklist(&app)?);
    app.focus = Focus::Inspector;
    for (width, height) in [(160, 40), (100, 30)] {
        app.inspect_scroll = 0;
        let mut found = false;
        for _ in 0..40 {
            let buffer = render(&mut app, width, height)?;
            let area = writfold_tui::layout::focused(width, height, true, true)
                .inspector
                .ok_or("inspector area")?;
            let panel: Vec<String> = rows(&buffer)
                .iter()
                .map(|row| {
                    row.chars()
                        .skip(usize::from(area.x))
                        .take(usize::from(area.width))
                        .collect()
                })
                .collect();
            if let Some(line) = panel.iter().find(|line| line.contains('■')) {
                let segments: String = line.chars().filter(|c| "■◆!□".contains(*c)).collect();
                assert_eq!(segments, "■!□◆", "{width}: {line}");
                found = true;
                break;
            }
            app.key(Key::Char('j'));
        }
        assert!(found, "required segments visible at {width}x{height}");
    }
    Ok(())
}

#[test]
fn no_subject_keeps_notes_without_claiming_completion() -> TestResult {
    let w = map::nested("no-subject-frame")?;
    let mut app = App::new(map::no_subject(&w), Some(RUN));
    app.focus = Focus::Detail;
    let answer = app.detail().ok_or("plan")?;
    assert!(
        !answer.notes.is_empty(),
        "fixture must exercise missing subject notes"
    );
    let subject_note = answer
        .notes
        .iter()
        .find(|note| note.contains("subject"))
        .ok_or("subject note")?
        .clone();
    for (width, height) in [(160, 40), (100, 30), (72, 20)] {
        let text = symbols(&render(&mut app, width, height)?);
        honest(&text)?;
        assert!(!text.contains("every step is done or skipped"));
        assert!(!text.contains("all steps done"));
        let prefix: String = subject_note.chars().take(20).collect();
        assert!(
            text.contains(&prefix),
            "missing note {subject_note}: {text}"
        );
    }
    Ok(())
}

#[test]
fn recorded_marker_and_later_chips_match_verb_rows_in_actual_map() -> TestResult {
    let w = support::plan::world("map-evidence")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.focus = Focus::Detail;
    let buffer = render(&mut app, 160, 40)?;
    let area = writfold_tui::layout::focused(160, 40, true, true).main;
    let rendered: Vec<String> = rows(&buffer)
        .iter()
        .map(|line| {
            line.chars()
                .skip(usize::from(area.x))
                .take(usize::from(area.width))
                .collect()
        })
        .collect();
    let nodes = graph::nodes(app.detail().ok_or("plan")?);
    for node in nodes {
        let row = rendered
            .iter()
            .find(|row| row.contains(node.id))
            .ok_or("map row")?;
        assert_eq!(
            row.contains('▶'),
            field(node.row, "status") == "current",
            "{row}"
        );
        if node.flags.contains(&"stale") && field(node.row, "status") == "done" {
            assert!(row.contains('!'));
            assert!(!row.contains('▶'));
        }
        if node.chips.contains(&"later") {
            assert!(row.contains('·'), "later gates are not judged: {row}");
        }
    }
    honest(&symbols(&buffer))?;
    Ok(())
}
