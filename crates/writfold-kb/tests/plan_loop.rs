//! Loops (#76): a plan step `loop=<template> max=N [min=N]` repeats its
//! body until its own gates are met. `plan again` records the next round
//! and is refused once the exit holds or every round is used; a spent loop
//! offers no `next:` and is never shown as passed; earlier rounds are
//! history (never `!stale`), so a loop can end `complete`; `plan use
//! <body> --again` cannot go around the bound; `plan advance` waits for
//! `min` rounds. Nothing runs a round: the test plays the agent.
#[path = "guard_status/helpers.rs"]
mod helpers;
mod support;

use std::path::Path;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::plan::PlanVerb;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
/// The exit: passes once `src/fixed` exists.
const FIXED: &str = "schema = \"writfold-gate/1\"\nid = \"fixed\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"test\", \"-f\", \"src/fixed\"]\ntimeout_seconds = 30\nscope = [\"src/**\"]\n";
const REVIEW: &str = "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\n";
const ATTEMPT: &str =
    "schema = \"writfold-plan/1\"\nname = \"attempt\"\nsteps = [\"change: review\"]\n";

fn put(path: &Path, text: &str) -> TestResult {
    std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
    std::fs::write(path, text)?;
    Ok(())
}

/// A world with the `fix` plan: `converge: fixed loop=attempt max=… min=…`.
fn looping(words: &str) -> TestResult<World> {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/gates/fixed.toml"), FIXED)?;
    put(&w.kb.join(".writfold/gates/review.toml"), REVIEW)?;
    put(&w.kb.join(".writfold/plans/attempt.toml"), ATTEMPT)?;
    let fix =
        format!("schema = \"writfold-plan/1\"\nname = \"fix\"\nsteps = [\"converge: {words}\"]\n");
    put(&w.kb.join(".writfold/plans/fix.toml"), &fix)?;
    let (text, exit) = plan(
        &w,
        PlanVerb::Set {
            template: Some("fix".into()),
            from: None,
            run: run(),
            repo: Some(w.code.clone()),
        },
    )?;
    assert_eq!(exit, 0, "{text}");
    Ok(w)
}

fn run() -> Option<String> {
    Some(RUN.into())
}

fn plan(w: &World, verb: PlanVerb) -> TestResult<(String, u8)> {
    match writfold_kb::run(&w.request(Verb::Plan(verb))) {
        Ok(out) => Ok((out.text, out.exit)),
        Err(e) => Ok((e.to_string(), e.exit_code())),
    }
}

fn again(w: &World) -> TestResult<(String, u8)> {
    plan(
        w,
        PlanVerb::Again {
            run: run(),
            repo: Some(w.code.clone()),
        },
    )
}

fn advance(w: &World) -> TestResult<(String, u8)> {
    plan(
        w,
        PlanVerb::Advance {
            run: run(),
            repo: Some(w.code.clone()),
        },
    )
}

fn status(w: &World) -> TestResult<String> {
    Ok(plan(
        w,
        PlanVerb::Status {
            gates: false,
            run: run(),
            repo: Some(w.code.clone()),
        },
    )?
    .0)
}

fn row<'a>(text: &'a str, step: &str) -> Vec<&'a str> {
    let mut lines = text
        .lines()
        .skip_while(|l| !l.starts_with(&format!("step {step} ")));
    let first = lines.next().into_iter();
    first
        .chain(lines.take_while(|l| l.starts_with("  ")))
        .collect()
}

fn gate(w: &World, id: &str) -> TestResult<u8> {
    let verb = GateVerb::Run {
        id: id.into(),
        run: run(),
        repo: Some(w.code.clone()),
    };
    Ok(writfold_kb::run(&w.request(Verb::Gate(verb)))?.exit)
}

fn attest(w: &World) -> TestResult {
    let say = Say {
        kind: Kind::Attested,
        id: "review".into(),
        text: "read the change".into(),
        by: "owner".into(),
        run: run(),
        repo: Some(w.code.clone()),
    };
    writfold_kb::run(&w.request(Verb::Gate(GateVerb::Say(say))))?;
    Ok(())
}

/// One round's body: review, then advance past `change`.
fn round(w: &World) -> TestResult {
    attest(w)?;
    let (text, exit) = advance(w)?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

#[test]
fn a_loop_goes_around_until_spent_and_is_never_shown_passed() -> TestResult {
    let w = looping("fixed loop=attempt max=2")?;
    round(&w)?;
    // The exit has no verdict yet: run it first, not plan again.
    let text = status(&w)?;
    let converge = row(&text, "fix#1/converge").join("\n");
    assert!(
        converge.contains("loop: attempt, round 1 of at most 2"),
        "{text}"
    );
    assert!(
        converge.contains("next: writfold kb gate run fixed"),
        "{text}"
    );
    assert_eq!(gate(&w, "fixed")?, 2);
    let converge = row(&status(&w)?, "fix#1/converge").join("\n");
    assert!(
        converge.contains("next: writfold kb plan again"),
        "{converge}"
    );
    let (text, exit) = again(&w)?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("round 2 of at most 2 under fix#1/converge"),
        "{text}"
    );
    // The round records what it began on, once, on its first step (#81).
    let began = status(&w)?;
    assert!(
        began.contains("began: after fixed failed (runs/") && began.matches("began:").count() == 1,
        "{began}"
    );
    round(&w)?;
    assert_eq!(gate(&w, "fixed")?, 2);
    let text = status(&w)?;
    let converge = row(&text, "fix#1/converge").join("\n");
    assert!(converge.contains("!spent"), "{converge}");
    assert!(
        converge.contains("no round is left") && !converge.contains("next:"),
        "{converge}"
    );
    let (text, exit) = again(&w)?;
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("has used its 2 rounds"), "{text}");
    let (text, exit) = advance(&w)?;
    assert_ne!(exit, 0, "a spent loop never advances: {text}");
    // The bound cannot be walked around with plan use --again.
    let verb = PlanVerb::Use {
        template: "attempt".into(),
        again: true,
        run: run(),
        repo: Some(w.code.clone()),
    };
    let (text, exit) = plan(&w, verb)?;
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("writfold kb plan again"), "{text}");
    Ok(())
}

#[test]
fn a_met_exit_advances_and_earlier_rounds_are_history() -> TestResult {
    let w = looping("fixed loop=attempt max=3")?;
    round(&w)?;
    assert_eq!(gate(&w, "fixed")?, 2);
    assert_eq!(again(&w)?.1, 0);
    // Round 2 fixes it: round 1's review no longer holds for these bytes.
    put(&w.code.join("src/fixed"), "yes\n")?;
    round(&w)?;
    assert_eq!(gate(&w, "fixed")?, 0);
    let (text, exit) = again(&w)?;
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("owes nothing more"), "{text}");
    let (text, exit) = advance(&w)?;
    assert_eq!(exit, 0, "{text}");
    let text = status(&w)?;
    let first = row(&text, "fix#1/converge/attempt#1/change").join("\n");
    assert!(
        first.contains("an earlier round") && !first.contains("!stale"),
        "{text}"
    );
    let converge = row(&text, "fix#1/converge").join("\n");
    assert!(
        converge.contains("loop: attempt, 2 rounds of at most 3"),
        "{text}"
    );
    assert!(
        text.contains("summary=complete") && !text.contains("complete-stale"),
        "{text}"
    );
    Ok(())
}

#[test]
fn min_rounds_are_owed_before_advancing() -> TestResult {
    let w = looping("loop=attempt min=2 max=2")?;
    round(&w)?;
    let (text, exit) = advance(&w)?;
    assert_eq!(exit, 3, "{text}");
    assert!(
        text.contains("owes 1 more rounds (1 of at least 2)"),
        "{text}"
    );
    assert_eq!(again(&w)?.1, 0);
    round(&w)?;
    let (text, exit) = advance(&w)?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

#[test]
fn loop_words_are_checked_where_they_are_written() -> TestResult {
    for (step, code) in [
        ("converge: fixed loop=attempt", "key.missing"),
        ("converge: fixed max=3", "key.not-here"),
        ("converge: fixed loop=attempt max=40", "key.range"),
        ("converge: fixed loop=attempt max=2 min=3", "key.range"),
        ("converge: loop=attempt max=3", "value.no-exit"),
        (
            "converge: fixed use=attempt loop=attempt max=3",
            "value.conflict",
        ),
        ("converge: fixed loop=attempt max=3 max=4", "value.twice"),
    ] {
        let fault = writfold_kb::plan::step_word::read(step).err().ok_or(step)?;
        assert_eq!(fault.code, code, "{step}: {}", fault.why);
    }
    let ok = writfold_kb::plan::step_word::read("converge: fixed loop=attempt max=3 min=2");
    assert!(ok.is_ok_and(|s| s.include.as_deref() == Some("attempt") && s.repeat.is_some()));
    Ok(())
}

#[test]
fn other_readers_show_the_loop() -> TestResult {
    let w = looping("fixed loop=attempt max=3")?;
    round(&w)?;
    assert_eq!(gate(&w, "fixed")?, 2);
    assert_eq!(again(&w)?.1, 0);
    let verb = GateVerb::Lines {
        run: run(),
        repo: Some(w.code.clone()),
    };
    let lines = writfold_kb::run(&w.request(Verb::Gate(verb)))?.text;
    assert!(
        lines.contains("loop fix#1/converge: round 2 of at most 3, open"),
        "{lines}"
    );
    let mut request = w.request(Verb::Plan(PlanVerb::Status {
        gates: false,
        run: run(),
        repo: Some(w.code.clone()),
    }));
    request.json = true;
    let json: serde_json::Value = serde_json::from_str(&writfold_kb::run(&request)?.text)?;
    let rows = json["rows"].as_array().ok_or("rows")?;
    let converge = rows
        .iter()
        .find(|r| r["id"] == "fix#1/converge")
        .ok_or("converge row")?;
    assert_eq!(converge["loop"]["rounds"], 2, "{converge}");
    assert!(
        converge["loop"]["exit"].is_null(),
        "a round under way: no exit judged yet: {converge}"
    );
    let first = rows
        .iter()
        .find(|r| r["id"] == "fix#1/converge/attempt#1/change")
        .ok_or("round 1")?;
    assert_eq!(first["superseded"], true, "{first}");
    assert_eq!(json["trailer"]["counts"]["loops"], 1, "{json}");
    Ok(())
}

#[test]
fn a_loop_that_ends_on_a_statement_is_warned() -> TestResult {
    let w = looping("review loop=attempt max=2")?;
    let verb = Verb::Lint {
        paths: Vec::new(),
        repo: Some(w.code.clone()),
        deny_warnings: false,
        records: false,
    };
    let output = writfold_kb::run(&w.request(verb))?;
    let codes: Vec<&str> = (output.shown.iter())
        .flat_map(|r| r.diagnostics.iter().map(|d| d.code))
        .collect();
    assert!(codes.contains(&"policy.self-exit"), "{codes:?}");
    Ok(())
}

/// A Stop rule requiring `requirement`, the run current, and the loop spent.
fn spent_with_rule(requirement: &str) -> TestResult<World> {
    let w = looping("fixed loop=attempt max=1")?;
    round(&w)?;
    assert_eq!(gate(&w, "fixed")?, 2);
    helpers::current(&w)?;
    let rule = format!(
        "schema = \"writfold-guard-rule/1\"\nid = \"finish\"\nevent = \"stop\"\nrequire = [\"{requirement}\"]\n"
    );
    put(&w.kb.join(".writfold/guards/finish.toml"), &rule)?;
    Ok(w)
}

#[test]
fn a_spent_loop_settles_a_stop_rule_on_step_settled_only() -> TestResult {
    let w = spent_with_rule("step.met")?;
    assert!(
        !helpers::hook(&w, true, "met-session").stdout.is_empty(),
        "step.met still nudges a spent loop"
    );
    let w = spent_with_rule("step.settled")?;
    assert!(
        helpers::hook(&w, true, "settled-session").stdout.is_empty(),
        "a spent loop is settled"
    );
    // A loop with rounds left is nudged under step.settled too.
    let w = looping("fixed loop=attempt max=2")?;
    round(&w)?;
    assert_eq!(gate(&w, "fixed")?, 2);
    helpers::current(&w)?;
    let rule = "schema = \"writfold-guard-rule/1\"\nid = \"finish\"\nevent = \"stop\"\nrequire = [\"step.settled\"]\n";
    put(&w.kb.join(".writfold/guards/finish.toml"), rule)?;
    let nudge = helpers::hook(&w, true, "rounds-left").stdout;
    assert!(nudge.contains("plan again"), "{nudge}");
    Ok(())
}
