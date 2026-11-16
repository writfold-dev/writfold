//! The plain surface carries no notation: the token scan agrees with the
//! terse list, no `Said` piece is notation, rendered plain frames match
//! none of the list, and the plain map has no chips.
mod support;

use serde_json::Value;
use support::kb_app::{kb_app, press};
use support::projection::{render, rows, symbols};
use support::terse::found;
use support::{RUN, TestResult, World};
use writfold_tui::app::{App, Focus, Key, View};
use writfold_tui::cells::Glyphs;
use writfold_tui::inspect::plain::{Context, told as inspected};
use writfold_tui::say::{Piece, Told, readable};
use writfold_tui::source::{Reply, field};
use writfold_tui::style::{Depth, Look};
use writfold_tui::{graph, say_gate, say_lists, say_row};

fn worlds(tag: &str) -> TestResult<Vec<World>> {
    Ok(vec![
        support::map::nested(&format!("sweep-nested-{tag}"))?,
        support::map::out_of_order(&format!("sweep-disorder-{tag}"))?,
        support::plan::world(&format!("sweep-plan-{tag}"))?,
    ])
}

fn loaded(w: &World) -> App {
    let mut app = App::new(w.source(), Some(RUN));
    for view in View::ALL {
        app.load_view(view);
    }
    app
}

fn texts(reply: &Reply, out: &mut Vec<String>) {
    out.extend(reply.notes.iter().cloned());
    for row in &reply.rows {
        out.push(field(row, "text").to_owned());
        out.extend(writfold_tui::lines::more(row).map(|m| field(m, "text").to_owned()));
    }
}

#[test]
fn readable_agrees_with_the_sweep() -> TestResult {
    let mut all = Vec::new();
    for w in worlds("agree")? {
        let app = loaded(&w);
        for view in View::ALL {
            texts(app.answer(view).ok_or("answer")?, &mut all);
        }
    }
    let atlas = support::kb_world::atlas("sweep-agree")?;
    let mut app = kb_app(&atlas);
    for keys in ["1", "2", "3", "4", "5", "1l"] {
        press(&mut app, keys);
        render(&mut app, 136, 38)?;
    }
    for reply in app.kb_replies() {
        texts(reply, &mut all);
    }
    let mut refused = 0;
    for text in &all {
        if readable(text) {
            assert_eq!(found(text)?, Vec::<String>::new(), "{text}");
        } else {
            refused += 1;
        }
    }
    assert!(
        refused > 10 && refused < all.len(),
        "{refused} of {}",
        all.len()
    );
    Ok(())
}

fn said(told: &Told, label: &str) -> TestResult {
    for piece in told.lines.iter().flat_map(|p| &p.pieces) {
        if let Piece::Said(text) = piece {
            assert_eq!(found(text)?, Vec::<String>::new(), "{label}: {text}");
        }
    }
    Ok(())
}

#[test]
fn said_pieces_carry_no_notation() -> TestResult {
    for w in worlds("said")? {
        let app = loaded(&w);
        let names = |_: &Value| None;
        let answer = |view| app.answer(view).ok_or("answer");
        said(
            &say_gate::view(answer(View::Gates)?, Glyphs::Unicode),
            "gates",
        )?;
        said(&say_lists::inbox(answer(View::Inbox)?), "inbox")?;
        said(
            &say_lists::guards(answer(View::Guards)?, writfold_tui::cells::Glyphs::Unicode),
            "guards",
        )?;
        said(
            &say_row::timeline(answer(View::Timeline)?, &names),
            "timeline",
        )?;
        for view in [View::Stale, View::Brief, View::Resume] {
            said(
                &say_row::generic(answer(view)?, Glyphs::Ascii, &names),
                "generic",
            )?;
        }
        let nodes = graph::nodes(answer(View::Progress)?);
        for (index, node) in nodes.iter().enumerate() {
            let cx = Context {
                index,
                total: nodes.len(),
                owed: Some(1),
            };
            said(&inspected(node, cx, Glyphs::Unicode), node.id)?;
            let words = writfold_tui::say_step::row(node, true).map(|(t, _)| t);
            assert_eq!(found(&words.unwrap_or_default())?, Vec::<String>::new());
        }
    }
    let atlas = support::kb_world::atlas("sweep-said")?;
    let mut app = kb_app(&atlas);
    for view in "12345".chars() {
        press(&mut app, &view.to_string());
        render(&mut app, 136, 38)?;
        for entry in app.kb_entries().into_iter().filter(|e| !e.header) {
            app.kb.selected.insert(app.kb.view, entry.key.clone());
            app.kb_load();
            let record = app.kb.record(&entry.id).cloned().ok_or("record")?;
            let told =
                writfold_tui::kb_plain_inspect::inspector(&app, &entry, &record, Glyphs::Unicode);
            said(&told, &entry.id)?;
        }
    }
    Ok(())
}

/// Field values the records' authors wrote (`[fsync-stall]`): quoted
/// whole, never rewritten, so the sweep leaves them out.
fn authored(app: &App) -> Vec<String> {
    let mut out = Vec::new();
    for reply in app.kb_replies() {
        for row in &reply.rows {
            for more in writfold_tui::lines::more(row) {
                let text = field(more, "text");
                if field(more, "label") == "field"
                    && let Some((_, value)) = text.split_once('=')
                {
                    out.push(value.to_owned());
                }
            }
        }
    }
    out
}

fn sweep(app: &mut App, label: &str) -> TestResult {
    for (width, height) in [(136, 38), (100, 30), (80, 24)] {
        let mut text = symbols(&render(app, width, height)?);
        for value in authored(app) {
            text = text.replace(&value, "");
        }
        // The ASCII glyph set draws its state marks in brackets.
        for glyph in ["[x]", "[!]", "[-]", "[>]", "[ ]", "[#]", "[.]"] {
            text = text.replace(glyph, "   ");
        }
        let hits = found(&text)?;
        assert!(
            hits.is_empty(),
            "{label} {width}x{height}: {hits:?}\n{text}"
        );
    }
    Ok(())
}

#[test]
fn plain_surface_has_no_terse_tokens() -> TestResult {
    for dumb in [false, true] {
        for w in worlds(if dumb { "ascii" } else { "unicode" })? {
            let mut app = App::new(w.source(), Some(RUN));
            app.look = Look {
                depth: Depth::Truecolor,
                ..Look::default()
            };
            app.dumb = dumb;
            for view in View::ALL {
                app.show(view);
                app.focus = Focus::Detail;
                sweep(&mut app, view.name())?;
            }
            app.show(View::Progress);
            app.focus = Focus::Inspector;
            for _ in 0..12 {
                sweep(&mut app, "inspector")?;
                app.key(Key::Char('j'));
            }
        }
    }
    let atlas = support::kb_world::atlas("sweep-surface")?;
    let mut app = kb_app(&atlas);
    app.look.depth = Depth::Truecolor;
    for keys in ["1", "2", "3", "4", "5"] {
        press(&mut app, keys);
        for _ in 0..8 {
            sweep(&mut app, keys)?;
            press(&mut app, "lj");
            app.key(Key::Esc);
        }
    }
    Ok(())
}

#[test]
fn plain_map_has_no_chips() -> TestResult {
    let w = support::map::nested("sweep-chips")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.focus = Focus::Detail;
    let buffer = render(&mut app, 136, 38)?;
    let map = writfold_tui::layout::focused(136, 38, true, true).main;
    for row in rows(&buffer)
        .iter()
        .skip(usize::from(map.y))
        .take(usize::from(map.height))
    {
        let cells: String = row
            .chars()
            .skip(usize::from(map.x))
            .take(usize::from(map.width))
            .collect();
        assert!(!cells.contains('●') && !cells.contains('◆'), "{cells}");
        assert_eq!(found(&cells)?, Vec::<String>::new(), "{cells}");
    }
    app.key(Key::Char('d'));
    let exact = symbols(&render(&mut app, 136, 38)?);
    assert!(exact.contains('●'), "the exact map keeps its chips");
    Ok(())
}
