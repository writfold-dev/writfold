//! The plain level's run-mode frames at the sizes the guide shows: the
//! inspector at 80×24 and the Gates, Inbox, Guards and Timeline views.
mod support;

use support::projection::{golden, normalize, render, symbols};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Focus, Key, View};

#[test]
fn plain_inspector_and_text_view_goldens() -> TestResult {
    let w = support::map::nested("t6-frames")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.focus = Focus::Inspector;
    let mut buffer = render(&mut app, 80, 24)?;
    support::buffers::stable(&mut buffer)?;
    golden("t6-inspector-80x24.txt", &normalize(&w, &symbols(&buffer))?)?;
    for (view, name, sizes) in [
        (View::Gates, "gates", &[(136, 38), (80, 24)][..]),
        (View::Inbox, "inbox", &[(136, 38)][..]),
        (View::Guards, "guards", &[(136, 38)][..]),
        (View::Timeline, "timeline", &[(136, 38)][..]),
    ] {
        for (width, height) in sizes {
            let mut app = App::new(w.source(), Some(RUN));
            app.show(view);
            app.focus = Focus::Detail;
            let text = normalize(&w, &symbols(&render(&mut app, *width, *height)?))?;
            golden(&format!("t6-{name}-{width}x{height}.txt"), &text)?;
            app.key(Key::Char('d'));
            let exact = symbols(&render(&mut app, *width, *height)?);
            assert_ne!(normalize(&w, &exact)?, text, "d changes {name}");
        }
    }
    Ok(())
}

/// The `c` rows of the screen: from the title row to `… closes`.
fn command_rows(screen: &[String]) -> Vec<String> {
    let start = screen
        .iter()
        .position(|r| r.starts_with(" commands for "))
        .unwrap_or(screen.len());
    screen[start..].to_vec()
}

#[test]
fn command_rows_are_whole_and_borderless() -> TestResult {
    let w = support::map::nested("t6-commands")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.focus = Focus::Detail;
    render(&mut app, 136, 38)?;
    app.key(Key::Char('c'));
    let buffer = render(&mut app, 136, 38)?;
    golden("t6-commands-136x38.txt", &normalize(&w, &symbols(&buffer))?)?;
    let plan = app.answer(View::Progress).ok_or("plan")?;
    let node = writfold_tui::graph::nodes(plan)
        .into_iter()
        .find(|n| Some(n.id) == app.node.as_deref())
        .ok_or("selected step")?;
    let commands: Vec<String> = writfold_tui::lines::more(node.row)
        .filter(|m| writfold_tui::source::field(m, "label") == "next")
        .map(|m| writfold_tui::source::field(m, "text").to_owned())
        .collect();
    assert!(!commands.is_empty(), "the current step suggests a command");
    for (width, height) in [(136, 38), (80, 24), (60, 20)] {
        let screen = support::projection::rows(&render(&mut app, width, height)?);
        let rows = command_rows(&screen);
        assert!(rows.len() >= 3, "{width}: {screen:#?}");
        assert!(rows.iter().all(|r| !r.contains('│')), "{rows:#?}");
        let mut joined = String::new();
        for row in &rows {
            let row = row.trim();
            match row.strip_suffix(" \\") {
                Some(head) => {
                    joined.push_str(head);
                    joined.push(' ');
                }
                None => {
                    joined.push_str(row);
                    joined.push('\n');
                }
            }
        }
        for command in &commands {
            assert!(joined.contains(command.as_str()), "{width}: {joined}");
        }
        assert!(rows.iter().any(|r| r.contains(&format!("# run   {RUN}"))));
    }
    let narrow = support::projection::rows(&render(&mut app, 60, 20)?);
    assert!(
        command_rows(&narrow)
            .iter()
            .any(|r| r.trim_end().ends_with(" \\"))
    );
    app.key(Key::Char('x'));
    assert!(!app.commands, "any other key closes the rows");
    Ok(())
}
