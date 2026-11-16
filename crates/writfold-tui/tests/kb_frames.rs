//! Knowledge-base mode as whole frames: goldens at the sizes the guide
//! shows, the plain `--once` form, no world path on screen, honest words,
//! and the capture world for the website (only with `--features capture-world`).
mod support;

use support::kb_app::{honest, kb_app, press};
use support::kb_world::atlas;
use support::projection::{golden, normalize, render, symbols};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Key};
use writfold_tui::kb_state::KbView;

#[test]
fn kb_frame_goldens() -> TestResult {
    let w = atlas("frames")?;
    let cases: [(&str, u16, u16, &str); 5] = [
        ("t5-why-136x38.txt", 136, 38, "jj"),
        ("t5-why-80x24.txt", 80, 24, ""),
        ("t5-why-detail-136x38.txt", 136, 38, "jjd"),
        ("t5-impact-136x38.txt", 136, 38, "4"),
        ("t5-tracker-100x30.txt", 100, 30, "3"),
    ];
    for (name, width, height, keys) in cases {
        let mut app = kb_app(&w);
        press(&mut app, keys);
        let text = symbols(&render(&mut app, width, height)?);
        assert!(
            !text.contains(&w.base.display().to_string()),
            "{name}: {text}"
        );
        honest(&app, &text)?;
        golden(name, &normalize(&w, &text)?)?;
    }
    Ok(())
}

#[test]
fn the_impact_of_harbor_db_reads_as_a_tree() -> TestResult {
    let w = atlas("impact-frame")?;
    let mut app = kb_app(&w);
    app.key(Key::Char('4'));
    app.kb.subject = Some("harbor-db".into());
    app.kb_show(KbView::Impact);
    let text = symbols(&render(&mut app, 136, 38)?);
    for needle in [
        "what points at Harbor ledger store",
        "┬ Harbor ledger store",
        "│ ○ Cap replay batches at 500 rows",
        "14 records reached · not followed: 1 disputed link,",
    ] {
        assert!(text.contains(needle), "{needle}: {text}");
    }
    Ok(())
}

#[test]
fn every_kb_view_at_both_levels_is_honest_and_pathless() -> TestResult {
    let w = atlas("honest")?;
    let base = w.base.display().to_string();
    for (width, height) in [(160, 40), (136, 38), (100, 30), (80, 24)] {
        let mut app = kb_app(&w);
        for keys in ["1", "2", "3", "4", "5", "1l", "d", "2", "3", "4", "5"] {
            press(&mut app, keys);
            let text = symbols(&render(&mut app, width, height)?);
            assert!(!text.contains(&base), "{width}x{height} {keys}: {text}");
            honest(&app, &text)?;
        }
    }
    Ok(())
}

#[test]
fn once_prints_the_projects_one_view_and_the_footer() -> TestResult {
    let w = atlas("once")?;
    let mut app = App::new(w.source(), Some(RUN));
    let text = writfold_tui::frame::once_kb(&mut app, KbView::Why);
    assert!(
        text.starts_with("== projects ==\n> project project:harbor [build]"),
        "{text}"
    );
    assert!(text.contains("== kb:why harbor ==\n"), "{text}");
    assert!(
        text.lines().last().is_some_and(|l| l.starts_with("== kb=")),
        "{text}"
    );
    assert!(!text.contains('\u{1b}') && !text.contains('●'), "{text}");
    // The KB commit holds the world's path in its tokens, so its id varies.
    let subject = regex::Regex::new(r"@[0-9a-f]{12}\+dirty")?;
    let text = subject
        .replace_all(&normalize(&w, &text)?, "@XXXXXXXXXXXX+dirty")
        .into_owned();
    golden("t5-once-kb-why.txt", &text)?;
    let plain = writfold_tui::once(w.source(), None, None);
    assert!(!plain.contains("== projects =="), "{plain}");
    Ok(())
}

#[test]
fn view_names_parse_with_their_kb_prefix() -> TestResult {
    for view in KbView::ALL {
        let args = ["--view".to_owned(), format!("kb:{}", view.name())];
        let options = writfold_tui::parse(&args)?;
        assert_eq!((options.kb_view, options.view), (Some(view), None));
    }
    let error = writfold_tui::parse(&["--view".into(), "kb:nothing".into()]);
    assert!(error.is_err_and(|e| e.contains("kb:why")));
    Ok(())
}

/// Builds the capture world under `target/t5-pty/kb` and leaves it. A manual
/// step for the website frames, not a check: it compiles only with
/// `--features capture-world`, so the unit gate neither runs nor claims it.
#[cfg(feature = "capture-world")]
#[test]
fn capture_world() -> TestResult {
    use support::kb_world::atlas_at;
    use writfold_tui::app::Focus;
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/t5-pty");
    let w = atlas_at(base)?;
    let mut app = kb_app(&w);
    app.kb.focus = Focus::Detail;
    let text = symbols(&render(&mut app, 136, 38)?);
    assert!(text.contains("why (5)"), "{text}");
    std::mem::forget(w);
    Ok(())
}
