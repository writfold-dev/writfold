//! The plain level drops no fact: everything the verbs answered is on the
//! exact level or in the command rows, the exact level is the pre-T6
//! detailed frame byte for byte, and `--once` output is unchanged. These
//! compare against the committed files directly; bless never rewrites them.
mod support;

use std::path::Path;
use support::kb_app::{kb_app, press};
use support::projection::{normalize, render, rows, symbols};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Detail, Focus, Key, View};
use writfold_tui::kb_state::KbView;
use writfold_tui::source::field;

fn committed(name: &str) -> TestResult<String> {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    Ok(std::fs::read_to_string(file)?)
}

#[test]
fn exact_level_is_unchanged() -> TestResult {
    let w = support::map::nested("t6-exact")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.focus = Focus::Detail;
    app.key(Key::Char('d'));
    let mut buffer = render(&mut app, 136, 38)?;
    support::buffers::stable(&mut buffer)?;
    let text = normalize(&w, &symbols(&buffer))?;
    assert_eq!(text, committed("t4-detail-136x38.txt")?);
    let w = support::kb_world::atlas("t6-exact-kb")?;
    let mut app = kb_app(&w);
    press(&mut app, "jjd");
    let text = normalize(&w, &symbols(&render(&mut app, 136, 38)?))?;
    assert_eq!(text, committed("t5-why-detail-136x38.txt")?);
    Ok(())
}

#[test]
fn once_is_unchanged() -> TestResult {
    let w = support::world("t6-once")?;
    let text = support::stable(&writfold_tui::once(w.source(), None, Some(View::Resume)));
    assert_eq!(text, committed("watch-once.txt")?);
    let w = support::plan::world("t6-once-progress")?;
    let mut app = App::new(w.source(), Some(RUN));
    let text = normalize(&w, &writfold_tui::frame::once(&mut app, None))?;
    assert_eq!(text, committed("progress-once.txt")?);
    let text = normalize(&w, &writfold_tui::frame::once(&mut app, Some(View::Gates)))?;
    assert_eq!(text, committed("progress-once-view-gates.txt")?);
    let w = support::kb_world::atlas("t6-once-kb")?;
    let mut app = App::new(w.source(), Some(RUN));
    let text = writfold_tui::frame::once_kb(&mut app, KbView::Why);
    let subject = regex::Regex::new(r"@[0-9a-f]{12}\+dirty")?;
    let text = subject
        .replace_all(&normalize(&w, &text)?, "@XXXXXXXXXXXX+dirty")
        .into_owned();
    assert_eq!(text, committed("t5-once-kb-why.txt")?);
    Ok(())
}

/// Everything the exact level and the command rows show, as one text.
fn shown(app: &mut App) -> TestResult<String> {
    let mut out = String::new();
    app.level = Detail::Exact;
    for view in [View::Gates, View::Inbox, View::Guards, View::Timeline] {
        app.show(view);
        app.focus = Focus::Detail;
        out.push_str(&symbols(&render(app, 220, 120)?));
    }
    app.show(View::Progress);
    app.key(Key::Char('x'));
    out.push_str(&symbols(&render(app, 220, 120)?));
    app.key(Key::Char('x'));
    for view in [View::Gates, View::Inbox] {
        app.show(view);
        app.key(Key::Char('c'));
        out.push_str(&rows(&render(app, 220, 120)?).join("\n"));
        app.key(Key::Esc);
    }
    Ok(out)
}

#[test]
fn d_restores_every_fact() -> TestResult {
    let w = support::map::nested("t6-facts")?;
    let mut app = App::new(w.source(), Some(RUN));
    render(&mut app, 136, 38)?;
    let shown = shown(&mut app)?;
    let mut facts: Vec<String> = Vec::new();
    for view in [View::Gates, View::Progress, View::Inbox] {
        let answer = app.answer(view).ok_or("answer")?;
        for row in &answer.rows {
            facts.push(field(row, "id").to_owned());
            if !field(row, "path").is_empty() {
                facts.push(format!("@{}", field(row, "path")));
            }
            for more in writfold_tui::lines::more(row) {
                if matches!(field(more, "label"), "next" | "confirm" | "accept") {
                    facts.push(field(more, "text").to_owned());
                }
            }
        }
    }
    assert!(facts.len() > 20, "{facts:#?}");
    for fact in facts.iter().filter(|f| !f.is_empty()) {
        let flat = shown.replace(['\n', '│'], "");
        let found = shown.contains(fact.as_str()) || flat.contains(fact.as_str());
        assert!(found, "{fact} is on neither the exact level nor the c rows");
    }
    Ok(())
}
