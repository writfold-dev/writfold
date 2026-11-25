//! `prose.vale` (#117, RFC 2026-11-25-vale-for-documents) against the
//! stand-in in `fixtures/vale/`: alerts weighed by the mode's threshold,
//! advice that never decides, every unreadable run `unknown` with a way
//! to fix it, the run's isolation, and a long document.
mod support;
mod vale_support;

use support::TestResult;
use vale_support::{one, status, vale_world};

#[test]
fn errors_block_and_the_rest_is_advice() -> TestResult {
    let w = vale_world("")?;
    let (exit, text, r) = one(&w, "<!-- standin: advice -->\nThis is very fine.\n")?;
    assert_eq!(exit, 0, "{text}");
    assert_eq!(r["result"]["outcome"], "pass");
    assert_eq!(r["result"]["advice_total"], 2);
    assert_eq!(r["result"]["advice"][0]["rule"], "House.Terms");
    assert_eq!(
        r["result"]["advice"][0]["detail"],
        "column 1: warning: Say KB."
    );
    let note = r["result"]["note"].as_str().unwrap_or_default();
    assert!(
        note.contains("vale 3.28.1; 3 checks; blocks from error; 2 alerts"),
        "{note}"
    );

    let w = vale_world("")?;
    let (exit, _, r) = one(&w, "<!-- standin: errors -->\nTODO\n")?;
    assert_ne!(exit, 0);
    assert_eq!(r["result"]["outcome"], "fail");
    assert_eq!(r["result"]["findings"][0]["rule"], "House.Todo");
    assert_eq!(r["result"]["advice_total"], 1);
    Ok(())
}

#[test]
fn the_threshold_moves_the_line() -> TestResult {
    let w = vale_world("vale_block = \"never\"\n")?;
    let (exit, _, r) = one(&w, "<!-- standin: errors -->\nTODO\n")?;
    assert_eq!((exit, &r["result"]["outcome"]), (0, &"pass".into()));
    assert_eq!(r["result"]["advice_total"], 2);

    let w = vale_world("vale_block = \"warning\"\n")?;
    let (_, _, r) = one(&w, "<!-- standin: advice -->\nvery\n")?;
    assert_eq!(r["result"]["outcome"], "fail");
    assert_eq!(r["result"]["findings"][0]["rule"], "House.Terms");
    Ok(())
}

#[test]
fn every_run_that_cannot_be_read_is_unknown() -> TestResult {
    let cases = [
        ("absent", "did not run", "install vale 3.28.1"),
        (
            "clean version=3.27.0",
            "vale 3.27.0 is installed",
            "move the pin",
        ),
        (
            "config-error",
            "vale exited 2: E100: unable to find StylesPath",
            "correct the profile",
        ),
        ("garbage", "not the --output=JSON --counts shape", ""),
        ("cut", "not the --output=JSON --counts shape", ""),
        (
            "unknown-severity",
            "not the --output=JSON --counts shape",
            "",
        ),
        ("stray-path", "not the --output=JSON --counts shape", ""),
        ("flood", "printed more than", ""),
        ("exit-disagrees", "exit status disagrees", ""),
        ("none-enabled", "loaded no check", "BasedOnStyles"),
        ("no-coverage", "the canary drew no alert", "[*.md]"),
        ("crash", "stopped by a signal", ""),
        ("slow", "ran past its timeout of 2 s", "timeout_seconds"),
    ];
    for (mode, note, fix) in cases {
        let w = vale_world("")?;
        let (exit, text, r) = one(&w, &format!("<!-- standin: {mode} -->\nText.\n"))?;
        assert_ne!(exit, 0, "{mode}: {text}");
        assert_eq!(r["result"]["outcome"], "unknown", "{mode}");
        let got = r["result"]["note"].as_str().unwrap_or_default();
        assert!(got.contains(note), "{mode}: {got}");
        let (shown, _) = status(&w)?;
        assert!(shown.contains("prose.vale [unknown]"), "{mode}: {shown}");
        if !fix.is_empty() {
            assert!(shown.contains(fix), "{mode}: {shown}");
        }
    }
    Ok(())
}

#[test]
fn vale_runs_isolated_in_a_private_directory() -> TestResult {
    let w = vale_world("")?;
    let log = w.base.join("standin.log");
    let text = format!("<!-- standin: clean log={} -->\nText.\n", log.display());
    let (exit, shown, _) = one(&w, &text)?;
    assert_eq!(exit, 0, "{shown}");
    let seen = std::fs::read_to_string(&log)?;
    let config = w.kb.join(".writfold/vale/house/.vale.ini");
    let argv = format!(
        "argv: --no-global --config {} --output=JSON --counts document.md canary.md",
        config.display()
    );
    assert!(seen.contains(&argv), "{seen}");
    let home = (seen.lines().find_map(|l| l.strip_prefix("HOME="))).ok_or("no HOME")?;
    assert!(home.contains("writfold-vale-"), "{home}");
    assert!(seen.contains(&format!("XDG_CONFIG_HOME={home}")), "{seen}");
    assert!(!seen.lines().any(|l| l.starts_with("VALE_")), "{seen}");
    assert!(
        !std::path::Path::new(home).exists(),
        "the private directory is removed"
    );
    Ok(())
}

#[test]
fn a_long_document_keeps_exact_totals() -> TestResult {
    let w = vale_world("")?;
    let body: String = (1..=3000)
        .map(|i| format!("Line {i} is very plain.\n"))
        .collect();
    let (exit, text, r) = one(&w, &format!("<!-- standin: many -->\n{body}"))?;
    assert_eq!(exit, 0, "{text}");
    assert_eq!(r["result"]["advice_total"], 2000);
    assert_eq!(r["result"]["advice"].as_array().map(Vec::len), Some(20));
    // Warnings come before suggestions.
    assert_eq!(
        r["result"]["advice"][0]["detail"],
        "column 1: warning: Avoid very."
    );
    let note = r["result"]["note"].as_str().unwrap_or_default();
    assert!(
        note.contains("20 warning, 1980 suggestion; most: House.Hedges 2000"),
        "{note}"
    );
    assert!(
        note.contains("all alerts: vale --no-global --config "),
        "{note}"
    );
    Ok(())
}
