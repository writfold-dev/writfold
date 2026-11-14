mod support;

use support::plan::world;
use support::projection::{render, rows};
use support::{RUN, TestResult};
use writfold_tui::app::{App, Key};
use writfold_tui::style::{Depth, Environment, Look, choose_theme, depth};
use writfold_tui::theme::ThemeId;

#[test]
fn automatic_depth_table_and_forced_overrides() -> TestResult {
    for (colorterm, no_color, term, expected) in [
        (None, false, None, Depth::Ansi),
        (Some("truecolor"), false, Some("xterm"), Depth::Truecolor),
        (Some("24bit"), false, None, Depth::Truecolor),
        (Some("256color"), false, Some("xterm-256color"), Depth::Ansi),
        (Some(""), false, Some("xterm"), Depth::Ansi),
        (Some("truecolor"), true, None, Depth::None),
        (Some("24bit"), false, Some("dumb"), Depth::None),
        (None, true, Some("dumb"), Depth::None),
    ] {
        let env = Environment {
            colorterm: colorterm.map(str::to_owned),
            no_color,
            term: term.map(str::to_owned),
            theme: None,
        };
        assert_eq!(depth(None, &env)?, expected);
        assert_eq!(depth(Some("auto"), &env)?, expected);
        for (flag, forced) in [
            ("truecolor", Depth::Truecolor),
            ("ansi", Depth::Ansi),
            ("none", Depth::None),
        ] {
            assert_eq!(depth(Some(flag), &env)?, forced);
        }
    }
    let error = depth(Some("256"), &Environment::default())
        .err()
        .ok_or("accepted invalid depth")?;
    for name in ["auto", "truecolor", "ansi", "none"] {
        assert!(error.contains(name), "{error}");
    }
    Ok(())
}

#[test]
fn theme_precedence_and_overridden_unknowns() {
    for (flag, env, config, expected) in [
        (
            Some("slate"),
            Some("carbide"),
            Some("paper"),
            ThemeId::Slate,
        ),
        (None, Some("carbide"), Some("paper"), ThemeId::Carbide),
        (None, None, Some("paper"), ThemeId::Paper),
        (None, None, None, ThemeId::Hone),
        (
            Some("vellum"),
            Some("bad-env"),
            Some("bad-config"),
            ThemeId::Vellum,
        ),
        (
            None,
            Some("paper-dark"),
            Some("bad-config"),
            ThemeId::PaperDark,
        ),
        (Some(" SLATE "), None, None, ThemeId::Slate),
    ] {
        assert_eq!(choose_theme(flag, env, config), (expected, None));
    }
}

#[test]
fn unknown_highest_source_falls_back_without_consulting_lower_sources() -> TestResult {
    for (flag, env, config, name, source) in [
        (
            Some("bad-flag"),
            Some("paper"),
            Some("slate"),
            "bad-flag",
            "--theme",
        ),
        (
            None,
            Some("bad-env"),
            Some("slate"),
            "bad-env",
            "WRITFOLD_THEME",
        ),
        (None, None, Some("bad-config"), "bad-config", "config.toml"),
    ] {
        let (theme, warning) = choose_theme(flag, env, config);
        assert_eq!(theme, ThemeId::Hone);
        let warning = warning.ok_or("missing warning")?;
        for text in [name, source, "using hone", ThemeId::choices()] {
            assert!(warning.contains(text), "{warning}");
        }
    }
    Ok(())
}

#[test]
fn truecolor_key_cycles_every_theme_and_wraps_with_effective_footer() -> TestResult {
    let w = world("theme-cycle")?;
    let mut app = App::new(w.source(), Some(RUN));
    app.look = Look {
        theme: ThemeId::Hone,
        depth: Depth::Truecolor,
    };
    for expected in ThemeId::ALL.into_iter().skip(1).chain([ThemeId::Hone]) {
        assert!(app.key(Key::Char('T')));
        assert_eq!(app.look.theme, expected);
        let buffer = render(&mut app, 160, 40)?;
        assert!(
            rows(&buffer)
                .last()
                .ok_or("footer")?
                .contains(&format!("theme={}", expected.name()))
        );
    }
    Ok(())
}

#[test]
fn degraded_theme_key_keeps_theme_and_explains_truecolor_requirement() -> TestResult {
    let w = world("theme-notice")?;
    let mut app = App::new(w.source(), Some(RUN));
    for (depth, name) in [(Depth::Ansi, "ansi"), (Depth::None, "none")] {
        app.look = Look {
            theme: ThemeId::Paper,
            depth,
        };
        let buffer = render(&mut app, 200, 40)?;
        assert!(
            rows(&buffer)
                .last()
                .ok_or("footer")?
                .contains(&format!("theme={name}"))
        );
        assert!(app.key(Key::Char('T')));
        assert_eq!(
            app.look,
            Look {
                theme: ThemeId::Paper,
                depth
            }
        );
        let buffer = render(&mut app, 200, 40)?;
        assert!(
            rows(&buffer)
                .join("\n")
                .contains("themes need 24-bit colour: COLORTERM=truecolor or --color truecolor")
        );
        app.key(Key::Char('c'));
    }
    Ok(())
}
