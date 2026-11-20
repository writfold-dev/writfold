//! `length.bounded` (delivery-modes second round, D18; #12): a fourth
//! built-in check on the whole payload, never on by default.
mod support;

use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::Prepare;

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn deliver(w: &World, verb: DeliverVerb, at: i64) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Deliver(verb));
    request.now = Some(at);
    w.run(&request)
}

fn prepare(w: &World, mode: &str, text: &str) -> TestResult<(String, u8)> {
    let from = w.base.join("message.txt");
    std::fs::write(&from, text)?;
    let verb = DeliverVerb::Prepare(Prepare {
        mode: mode.into(),
        from,
        audience: Some("internal".into()),
        destination: Some("harbor main".into()),
        run: Some(RUN.into()),
    });
    deliver(w, verb, NOW)
}

fn checked(w: &World) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Check {
        candidate: None,
        check: None,
        run: Some(RUN.into()),
        mode: None,
    };
    deliver(w, verb, NOW + 60)
}

fn mode(w: &World, name: &str, keys: &str) -> TestResult {
    let dir = w.kb.join(".writfold/deliveries");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join(format!("{name}.toml")),
        format!("schema = \"writfold-delivery-mode/1\"\nmode = \"{name}\"\nchecks = [\"length.bounded\"]\n{keys}"),
    )?;
    Ok(())
}

#[test]
fn a_payload_over_its_bound_fails_naming_the_line() -> TestResult {
    let w = world(&[])?;
    mode(&w, "short", "max_lines = 2\nmax_chars = 1000\n")?;
    let (text, exit) = prepare(&w, "short", "one\ntwo\nthree\n")?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = checked(&w)?;
    assert_eq!(exit, 2, "{text}");
    assert!(text.contains("length.bounded"), "{text}");
    assert!(text.contains("3 lines; this mode takes at most 2 (max_lines)"), "{text}");
    // Within both bounds it passes.
    let w = world(&[])?;
    mode(&w, "short", "max_lines = 2\nmax_chars = 9\n")?;
    prepare(&w, "short", "one\ntwo\n")?;
    let (text, exit) = checked(&w)?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

#[test]
fn length_bounded_without_a_bound_is_refused_and_built_ins_do_not_run_it() -> TestResult {
    let w = world(&[])?;
    mode(&w, "short", "")?;
    let why = match prepare(&w, "short", "x\n") {
        Ok((text, _)) => text,
        Err(e) => e.to_string(),
    };
    assert!(why.contains("length.bounded needs max_chars or max_lines"), "{why}");
    // A built-in mode's defaults do not include it.
    let w = world(&[])?;
    let long = "word ".repeat(20) + "\n";
    prepare(&w, "pr-comment", &long.repeat(200))?;
    let (text, _) = checked(&w)?;
    assert!(!text.contains("length.bounded"), "{text}");
    Ok(())
}
