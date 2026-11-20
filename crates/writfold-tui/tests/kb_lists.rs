//! What the knowledge-base lists show: exactly the verbs' rows, glyphs
//! looked up on the word, marks from flags and the path join, the tree
//! from each row's edge, the walk's limits, and the stale badge.
mod support;

use serde_json::Value;
use support::kb_app::kb_app;
use support::kb_world::{ACCEPTED, CODE, atlas, git_out};
use support::projection::{render, symbols};
use support::{NOW, TestResult};
use writfold_kb::Verb;
use writfold_kb::verbs::find::Filter;
use writfold_tui::cells::Glyphs;
use writfold_tui::kb_entry::{Entry, glyph, marks};

fn json(w: &support::World, verb: Verb) -> TestResult<Value> {
    Ok(serde_json::from_str(&w.verb(verb, true)?.0)?)
}

fn rows_of(body: &Value) -> Vec<Value> {
    body["rows"].as_array().cloned().unwrap_or_default()
}

fn selectable(app: &writfold_tui::app::App) -> Vec<Entry> {
    app.kb_entries().into_iter().filter(|e| !e.header).collect()
}

#[test]
fn why_lists_exactly_the_verbs_decisions() -> TestResult {
    let w = atlas("why-list")?;
    let mut app = kb_app(&w);
    let found = json(
        &w,
        Verb::Find {
            text: String::new(),
            filter: Filter {
                kinds: vec!["decision".into()],
                project: Some("harbor".into()),
                limit: None,
            },
            signatures: false,
        },
    )?;
    let expected: Vec<(String, String, String)> = rows_of(&found)
        .iter()
        .map(|r| {
            let s = |k: &str| r[k].as_str().unwrap_or_default().to_owned();
            (s("id"), s("text"), s("status"))
        })
        .collect();
    let shown = |app: &writfold_tui::app::App| -> Vec<(String, String, String)> {
        selectable(app)
            .into_iter()
            .map(|e| (e.id, e.label, e.word))
            .collect()
    };
    assert_eq!(shown(&app), expected);
    let decision = writfold_kb::write::record::Decision {
        title: "Retire the flush timer".into(),
        target_source: "src/ledger.rs".into(),
        project: Some("harbor".into()),
        ..Default::default()
    };
    let verb = Verb::Write(writfold_kb::write::Write::Decision(decision));
    writfold_kb::run(&w.request(verb, false, NOW))?;
    for _ in 0..5 {
        app.poll();
    }
    render(&mut app, 136, 38)?;
    assert!(
        shown(&app)
            .iter()
            .any(|(_, label, word)| label == "Retire the flush timer" && word == "draft"),
        "{:?}",
        shown(&app)
    );
    Ok(())
}

#[test]
fn glyph_is_a_lookup_on_the_word() -> TestResult {
    let table = [
        (
            "●",
            &[
                "accepted",
                "applied",
                "verified",
                "done",
                "n/a",
                "concluded",
                "fixed",
                "active",
                "closed",
            ][..],
        ),
        (
            "○",
            &[
                "draft",
                "proposed",
                "landing",
                "deferred",
                "todo",
                "in-progress",
                "blocked",
                "investigating",
                "open",
                "monitoring",
                "suspected",
                "known-error",
                "fix-in-progress",
                "regressed",
            ][..],
        ),
        (
            "–",
            &[
                "rejected",
                "superseded",
                "retired",
                "abandoned",
                "deprecated",
            ][..],
        ),
        (
            "·",
            &[
                "",
                "build",
                "sunsetting",
                "confirmed",
                "explicit",
                "invented-word",
            ][..],
        ),
    ];
    for (shape, words) in table {
        for word in words {
            assert_eq!(glyph(word, Glyphs::Unicode).glyph, shape, "{word}");
        }
    }
    for (word, ascii) in [
        ("done", "[x]"),
        ("todo", "[ ]"),
        ("retired", "[-]"),
        ("odd", "[.]"),
    ] {
        assert_eq!(glyph(word, Glyphs::Ascii).glyph, ascii);
    }
    // The word is printed beside the glyph, whatever it is.
    let w = atlas("glyph-words")?;
    let mut app = kb_app(&w);
    app.key(writfold_tui::app::Key::Char('4'));
    let text = symbols(&render(&mut app, 160, 40)?);
    assert!(text.contains("· Quay batch worker"), "{text}");
    app.key(writfold_tui::app::Key::Char('d'));
    let text = symbols(&render(&mut app, 160, 40)?);
    assert!(text.contains("project  sunsetting"), "{text}");
    Ok(())
}

fn accepted_marks(app: &writfold_tui::app::App) -> TestResult<writfold_tui::kb_entry::Marks> {
    let entry = selectable(app)
        .into_iter()
        .find(|e| e.id == ACCEPTED)
        .ok_or("accepted")?;
    Ok(marks(&entry, app.kb.stale.as_ref()))
}

#[test]
fn marks_come_from_flags_and_the_path_join() -> TestResult {
    let w = atlas("marks")?;
    let mut app = kb_app(&w);
    let entry = selectable(&app)
        .into_iter()
        .find(|e| e.id == ACCEPTED)
        .ok_or("accepted")?;
    assert!(
        entry.flags.is_empty(),
        "the listing row itself carries no flag"
    );
    assert!(accepted_marks(&app)?.moved, "joined from stale --cached");
    let code = w.base.join(CODE);
    let second = git_out(&code, &["rev-parse", "HEAD"])?;
    support::git(&code, &["reset", "-q", "--hard", "HEAD~1"])?;
    app.key(writfold_tui::app::Key::Char('r'));
    render(&mut app, 136, 38)?;
    let fresh = accepted_marks(&app)?;
    assert!(!fresh.moved && !fresh.unknown, "{fresh:?}");
    support::git(&code, &["reset", "-q", "--hard", &second])?;
    std::fs::rename(&code, w.base.join("moved-away"))?;
    app.key(writfold_tui::app::Key::Char('r'));
    render(&mut app, 136, 38)?;
    assert!(
        accepted_marks(&app)?.unknown,
        "an unresolvable repository is ?, never nothing"
    );
    let text = symbols(&render(&mut app, 136, 38)?);
    assert!(
        text.contains(" ? accepted") || text.contains("!? accepted"),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_recorded_flag_sets_no_mark_in_the_stale_view() -> TestResult {
    // A verdict read from a check receipt reaches the view as one more
    // flag on a `stale --cached` row: the rows and their marks are drawn
    // exactly as without it, and the flag itself marks nothing.
    let w = atlas("recorded-flag")?;
    let mut app = kb_app(&w);
    app.key(writfold_tui::app::Key::Char('5'));
    let words = symbols(&render(&mut app, 136, 38)?);
    app.key(writfold_tui::app::Key::Char('d'));
    let plain = symbols(&render(&mut app, 136, 38)?);
    let reply = app.kb.stale.as_mut().ok_or("no stale reply")?;
    assert!(!reply.rows.is_empty(), "the atlas has stale rows");
    for row in &mut reply.rows {
        if let Some(flags) = row["flags"].as_array_mut() {
            flags.push(Value::from("recorded"));
        }
    }
    let labelled = symbols(&render(&mut app, 136, 38)?);
    app.key(writfold_tui::app::Key::Char('d'));
    // The plain level has no phrase for the flag and shows nothing new.
    assert_eq!(symbols(&render(&mut app, 136, 38)?), words);
    // The list rows and marks are the same; only the inspector's flag
    // line names it.
    let changed: Vec<(&str, &str)> = plain
        .lines()
        .zip(labelled.lines())
        .filter(|(a, b)| a != b)
        .collect();
    assert!(
        changed
            .iter()
            .any(|(b, a)| b.contains("│expired ") && a.contains("│expired recorded ")),
        "{labelled}"
    );
    let list = |line: &str| line.chars().take(78).collect::<String>();
    for (before, after) in changed {
        assert_eq!(list(before), list(after));
    }
    let none = writfold_tui::kb_entry::flag_marks(["recorded"]);
    assert_eq!(none, writfold_tui::kb_entry::Marks::default());
    Ok(())
}
