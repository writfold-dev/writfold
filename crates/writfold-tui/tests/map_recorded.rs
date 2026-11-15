mod support;
use support::projection::{render, symbols};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Focus, Key, View};
use writfold_tui::{lines, source::field};

#[test]
fn recorded_position_event_and_checkpoint_quote_their_verbs() -> TestResult {
    let w = support::map::nested("recorded-block")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.focus = Focus::Detail;
    let plan = app.answer(View::Progress).ok_or("plan")?;
    let position = format!(
        "step {} of {}",
        plan.count("current").ok_or("current")?,
        plan.count("steps").ok_or("steps")?
    );
    let event = app
        .answer(View::Timeline)
        .ok_or("timeline")?
        .rows
        .iter()
        .rev()
        .find(|row| field(row, "kind") == "plan")
        .ok_or("plan event")?;
    let event = format!("last plan event: {}", field(event, "text"));
    let checkpoint = lines::more(app.row().ok_or("run")?)
        .find(|row| field(row, "label") == "checkpoint")
        .ok_or("checkpoint")?;
    let checkpoint = format!("last checkpoint: {}", field(checkpoint, "text"));
    // The summary states the position at both levels; the event and
    // checkpoint are quoted whole at the detailed level.
    let calm = symbols(&render(&mut app, 320, 40)?);
    assert!(calm.contains(&position), "{position}\n{calm}");
    assert!(!calm.contains("last checkpoint:"), "{calm}");
    app.key(Key::Char('d'));
    let text = symbols(&render(&mut app, 320, 40)?);
    for exact in [position, event, checkpoint] {
        assert!(text.contains(&exact), "{exact}\n{text}");
    }
    Ok(())
}

#[test]
fn minimum_narrow_surface_switches_between_map_and_inspector() -> TestResult {
    let w = support::map::nested("minimum-map")?;
    let mut app = App::new(w.source(), Some(RUN));
    let text = symbols(&render(&mut app, 60, 12)?);
    assert_eq!(app.focus, Focus::Detail);
    assert!(text.contains('▶'));
    app.key(Key::Right);
    let text = symbols(&render(&mut app, 60, 12)?);
    assert_eq!(app.focus, Focus::Inspector);
    assert!(text.contains("step 7/11"));
    assert!(text.contains("g gates"), "{text}");
    app.key(Key::Esc);
    assert!(symbols(&render(&mut app, 60, 12)?).contains('▶'));
    app.key(Key::Char('0'));
    app.key(Key::Char('p'));
    assert_eq!(app.focus, Focus::Detail);
    app.key(Key::Enter);
    assert_eq!(app.focus, Focus::Inspector);
    Ok(())
}

#[test]
fn dumb_terminal_keeps_ascii_shapes_when_colour_is_forced() -> TestResult {
    let w = support::plan::world("dumb-forced")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.dumb = true;
    app.look.depth = writfold_tui::style::Depth::Truecolor;
    app.focus = Focus::Detail;
    let text = symbols(&render(&mut app, 160, 40)?);
    assert!(text.contains("[>]") && text.contains("[x]") && text.contains("[!]"));
    assert!(!text.contains('▶') && !text.contains('✓'));
    Ok(())
}
