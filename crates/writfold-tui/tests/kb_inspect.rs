//! The record inspector, `a`/`v`, the KB bars, the empty KB, and that
//! visiting every KB view writes nothing and starts no checker.
mod support;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::collections::BTreeSet;
use support::TestResult;
use support::kb_app::{kb_app, press};
use support::kb_world::{ACCEPTED, atlas};
use support::projection::{render, rows};
use writfold_tui::app::{App, Key};
use writfold_tui::bars::Segment;
use writfold_tui::kb_bars;
use writfold_tui::layout::{self, Class};
use writfold_tui::style::{Depth, Look};

fn inspector(buffer: &Buffer, width: u16, height: u16) -> TestResult<Vec<String>> {
    let area: Rect = layout::focused(width, height, true, true)
        .inspector
        .ok_or("inspector")?;
    Ok(rows(buffer)
        .iter()
        .skip(usize::from(area.y + 1))
        .take(usize::from(area.height.saturating_sub(2)))
        .map(|row| {
            row.chars()
                .skip(usize::from(area.x + 1))
                .take(usize::from(area.width - 2))
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect())
}

fn select(app: &mut App, id: &str) {
    app.kb.selected.insert(app.kb.view, id.to_owned());
    app.kb_show(app.kb.view);
}

#[test]
fn inspector_plain_and_exact() -> TestResult {
    let w = atlas("inspector")?;
    let mut app = kb_app(&w);
    select(&mut app, ACCEPTED);
    let plain_buffer = render(&mut app, 160, 48)?;
    let plain = inspector(&plain_buffer, 160, 48)?;
    let position = regex::Regex::new(r"\.(md|json):\d+$")?;
    assert!(!plain.iter().any(|l| position.is_match(l)), "{plain:#?}");
    for line in [
        "Sources · 1 of 2 has moved",
        "  ! src/batch.rs:2-4 in demo-service",
        "    moved to 4-6",
        "decision · decided by ops-review on 2026-03-09",
    ] {
        assert!(plain.iter().any(|l| l == line), "{line}: {plain:#?}");
    }
    assert!(
        !plain.iter().any(|l| l.contains("ledger.rs:3-5")),
        "only moved sources: {plain:#?}"
    );
    app.key(Key::Char('d'));
    let exact = inspector(&render(&mut app, 160, 48)?, 160, 48)?;
    let sources: Vec<_> = exact.iter().filter(|l| position.is_match(l)).collect();
    assert!(
        !sources.is_empty(),
        "positions at the exact level: {exact:#?}"
    );
    assert!(exact.iter().any(|l| l == "cites 2  ● !"), "{exact:#?}");
    let full = inspector(&render(&mut app, 160, 120)?, 160, 120)?.join("\n");
    for needle in [
        "src/ledger.rs:3-5",
        "[fresh]",
        "mentions",
        "proposal.md:24",
        "status: coverage complete, records",
        "why: coverage",
    ] {
        assert!(full.contains(needle), "{needle}: {full}");
    }
    app.key(Key::Char('d'));
    assert_eq!(render(&mut app, 160, 48)?, plain_buffer);
    Ok(())
}

#[test]
fn draft_why_says_unknown() -> TestResult {
    let w = atlas("draft")?;
    let mut app = kb_app(&w);
    select(&mut app, "proposal/decision/2026-04-01-shard-ledger");
    let text = inspector(&render(&mut app, 160, 48)?, 160, 48)?.join("\n");
    assert!(
        text.contains("answer unknown · no-accepted-decision"),
        "{text}"
    );
    assert!(
        text.contains("absence in the KB is not evidence of absence"),
        "{text}"
    );
    Ok(())
}

#[test]
fn impact_of_a_file() -> TestResult {
    let w = atlas("impact-file")?;
    let mut app = kb_app(&w);
    select(&mut app, ACCEPTED);
    app.key(Key::Char('a'));
    assert_eq!(app.kb_subject().as_deref(), Some(ACCEPTED));
    app.key(Key::Char('v'));
    let code = "code:demo-service:src/batch.rs";
    select(&mut app, code);
    app.key(Key::Char('a'));
    assert_eq!(app.kb_subject().as_deref(), Some(code));
    press(&mut app, "v");
    let citing: BTreeSet<String> = app
        .kb_entries()
        .into_iter()
        .filter(|e| !e.header && e.kind == "decision" && e.depth == 0)
        .map(|e| e.label)
        .collect();
    let expected: BTreeSet<String> = [
        "Batch ledger writes in the API",
        "Cap replay batches at 500 rows",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(citing, expected);
    let asked: Vec<_> = ["out", "both", "in"]
        .iter()
        .filter(|d| {
            app.kb_reply(&writfold_tui::kb_state::Ask::Walk(
                code.into(),
                match **d {
                    "out" => writfold_kb::verbs::impact::Direction::Out,
                    "both" => writfold_kb::verbs::impact::Direction::Both,
                    _ => writfold_kb::verbs::impact::Direction::In,
                },
            ))
            .is_some()
        })
        .collect();
    assert_eq!(asked, [&"out", &"both"]);
    press(&mut app, "v");
    assert_eq!(app.kb.direction, writfold_kb::verbs::impact::Direction::In);
    Ok(())
}

fn words(parts: &[Segment]) -> BTreeSet<String> {
    let text: String = parts.iter().map(|p| p.text.as_str()).collect();
    text.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn kb_bars_never_cut_a_token() -> TestResult {
    let w = atlas("bars")?;
    let mut app = kb_app(&w);
    for width in 60..=200 {
        let screen = rows(&render(&mut app, width, 40)?);
        let narrow = layout::class(width, 40) == Class::Narrow;
        let bars = [
            (&screen[0], words(&kb_bars::title(&app, 1000, narrow))),
            (&screen[1], words(&kb_bars::tabs(&app, 1000, narrow))),
            (&screen[39], words(&kb_bars::footer(&app, 1000, narrow))),
        ];
        for (row, whole) in bars {
            for word in row.split_whitespace() {
                assert!(whole.contains(word), "{width}: {word:?} cut in {row:?}");
            }
        }
    }
    Ok(())
}

#[test]
fn kb_chips_degrade_to_brackets() -> TestResult {
    let w = atlas("chips")?;
    let mut app = kb_app(&w);
    for depth in [Depth::Ansi, Depth::None] {
        app.look = Look {
            depth,
            ..Look::default()
        };
        let screen = rows(&render(&mut app, 136, 38)?);
        assert!(screen[0].contains("[build]"), "{}", screen[0]);
        assert!(screen[1].contains("[1 why]"), "{}", screen[1]);
        assert!(screen[37].contains("[read-only]"), "{}", screen[37]);
    }
    app.look = Look {
        depth: Depth::None,
        ..Look::default()
    };
    let text = rows(&render(&mut app, 136, 38)?).join("\n");
    assert!(
        text.contains("[x] Batch ledger writes in the API"),
        "{text}"
    );
    assert!(!text.contains('●') && !text.contains('┬'), "{text}");
    Ok(())
}
