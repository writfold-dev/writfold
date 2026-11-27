//! `prose.rules` (#127) and advice kept on a pass (#115), RFC
//! 2026-11-25-prose-rules-and-advice: rules kept as data, advice that is
//! shown and recorded but never decides, blocking matches that fail, and
//! an empty rule set that is never a pass.
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
        invocation: None,
    });
    deliver(w, verb, NOW)
}

fn checked(w: &World, at: i64) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Check {
        candidate: None,
        check: None,
        run: Some(RUN.into()),
        mode: None,
    };
    deliver(w, verb, at)
}

fn status(w: &World) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Status {
        candidate: None,
        run: Some(RUN.into()),
        all: false,
    };
    deliver(w, verb, NOW + 600)
}

fn write(w: &World, name: &str, text: &str) -> TestResult {
    let dir = w.kb.join(".writfold/deliveries");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(name), text)?;
    Ok(())
}

fn mode(w: &World, checks: &str, keys: &str) -> TestResult {
    write(
        w,
        "note.toml",
        &format!(
            "schema = \"writfold-delivery-mode/1\"\nmode = \"note\"\nchecks = {checks}\n{keys}"
        ),
    )
}

fn rules(w: &World, advise: &str, block: &str) -> TestResult {
    write(
        w,
        "prose-plain.toml",
        &format!(
            "schema = \"writfold-prose-rules/1\"\nname = \"plain\"\nadvise = [{advise}]\nblock = [{block}]\n"
        ),
    )
}

const FILLER: &str = "\"filler: \\\\bjust\\\\b\"";
const TODO: &str = "\"placeholder: \\\\bTODO\\\\b\"";

/// The JSON of every check receipt under the run's deliveries.
fn receipts(w: &World) -> TestResult<Vec<serde_json::Value>> {
    let mut out = Vec::new();
    let root = w.kb.join(format!("runs/{RUN}/deliveries"));
    for candidate in std::fs::read_dir(root)?.flatten() {
        for file in std::fs::read_dir(candidate.path().join("checks"))?.flatten() {
            out.push(serde_json::from_str(&std::fs::read_to_string(
                file.path(),
            )?)?);
        }
    }
    Ok(out)
}

#[test]
fn advice_is_shown_and_kept_but_never_holds_the_draft_back() -> TestResult {
    let w = world(&[])?;
    rules(&w, FILLER, TODO)?;
    mode(&w, "[\"prose.rules\"]", "prose = \"plain\"\n")?;
    prepare(
        &w,
        "note",
        "We just merged it.\n\n```\njust code TODO\n```\n",
    )?;
    let (text, exit) = checked(&w, NOW + 60)?;
    assert_eq!(exit, 0, "{text}");
    let (shown, _) = status(&w)?;
    assert!(shown.contains("prose.rules [satisfied]"), "{shown}");
    assert!(shown.contains("1 advisory: line 1 filler"), "{shown}");
    let r = &receipts(&w)?[0];
    assert_eq!(r["result"]["outcome"], "pass");
    assert_eq!(r["result"]["findings_total"], 0);
    assert_eq!(r["result"]["advice_total"], 1);
    assert_eq!(r["result"]["advice"][0]["rule"], "filler");
    Ok(())
}

#[test]
fn a_blocking_match_fails_and_an_empty_set_is_unknown() -> TestResult {
    let w = world(&[])?;
    rules(&w, FILLER, TODO)?;
    mode(&w, "[\"prose.rules\"]", "prose = \"plain\"\n")?;
    prepare(&w, "note", "TODO: write this.\n")?;
    let (text, exit) = checked(&w, NOW + 60)?;
    assert_ne!(exit, 0, "{text}");
    assert_eq!(receipts(&w)?[0]["result"]["outcome"], "fail");

    let w = world(&[])?;
    rules(&w, "", "")?;
    mode(&w, "[\"prose.rules\"]", "prose = \"plain\"\n")?;
    prepare(&w, "note", "Fine words.\n")?;
    let (text, exit) = checked(&w, NOW + 60)?;
    assert_ne!(exit, 0, "{text}");
    let r = &receipts(&w)?[0];
    assert_eq!(r["result"]["outcome"], "unknown");
    assert!(r["result"].get("advice").is_none());
    Ok(())
}

#[test]
fn editing_a_rule_stales_the_check() -> TestResult {
    let w = world(&[])?;
    rules(&w, FILLER, TODO)?;
    mode(&w, "[\"prose.rules\"]", "prose = \"plain\"\n")?;
    prepare(&w, "note", "Plain words.\n")?;
    let (text, exit) = checked(&w, NOW + 60)?;
    assert_eq!(exit, 0, "{text}");
    // Moving a rule from advice to blocking changes what the check means.
    rules(&w, "", &format!("{FILLER}, {TODO}"))?;
    let (shown, _) = status(&w)?;
    assert!(shown.contains("prose.rules [stale]"), "{shown}");
    Ok(())
}

#[test]
fn the_mode_key_is_checked() -> TestResult {
    let w = world(&[])?;
    for (checks, keys, want) in [
        ("[\"prose.rules\"]", "", "prose.rules needs prose"),
        (
            "[\"length.bounded\"]",
            "max_lines = 3\nprose = \"plain\"\n",
            "checks does not list prose.rules",
        ),
        ("[\"prose.rules\"]", "prose = \"missing\"\n", "no rule set"),
    ] {
        mode(&w, checks, keys)?;
        let why = match prepare(&w, "note", "x\n") {
            Ok((text, _)) => text,
            Err(e) => e.to_string(),
        };
        assert!(why.contains(want), "{keys}: {why}");
    }
    Ok(())
}

#[test]
fn a_script_gate_keeps_its_output_on_a_pass_as_advice() -> TestResult {
    let w = world(&[])?;
    let gates = w.kb.join(".writfold/gates");
    std::fs::create_dir_all(&gates)?;
    std::fs::write(
        gates.join("lint.toml"),
        "schema = \"writfold-gate/1\"\nid = \"lint\"\nversion = 1\nclass = \"mechanical\"\nsubject = \"candidate\"\nargv = [\"sh\", \"-c\", \"echo '1: [filler] just'\", \"lint\", \"{candidate}\"]\ntimeout_seconds = 10\n",
    )?;
    write(
        &w,
        "note.toml",
        "schema = \"writfold-delivery-mode/1\"\nmode = \"note\"\nchecks = []\ngates = [\"lint\"]\n",
    )?;
    prepare(&w, "note", "We just merged it.\n")?;
    let (text, exit) = checked(&w, NOW + 60)?;
    assert_eq!(exit, 0, "{text}");
    let r = &receipts(&w)?[0];
    assert_eq!(r["result"]["outcome"], "pass");
    assert_eq!(r["result"]["advice"][0]["rule"], "filler");
    assert_eq!(r["result"]["advice"][0]["detail"], "just");
    Ok(())
}

/// #135: a document's front matter is not prose, and with more matches than
/// a receipt keeps the note tallies them by rule.
#[test]
fn front_matter_is_skipped_and_many_matches_are_tallied() -> TestResult {
    let w = world(&[])?;
    rules(&w, &format!("{FILLER}, \"hedge: \\\\bmaybe\\\\b\""), TODO)?;
    mode(&w, "[\"prose.rules\"]", "prose = \"plain\"\n")?;
    let body: String = (0..30)
        .map(|i| match i % 3 {
            0 => "Maybe so.\n",
            _ => "We just did.\n",
        })
        .collect();
    prepare(&w, "note", &format!("---\ntitle: TODO just\n---\n{body}"))?;
    let (text, exit) = checked(&w, NOW + 60)?;
    assert_eq!(exit, 0, "the TODO in front matter is not prose: {text}");
    let r = &receipts(&w)?[0];
    assert_eq!(r["result"]["outcome"], "pass");
    assert_eq!(r["result"]["advice_total"], 30);
    assert_eq!(r["result"]["advice"][0]["line"], 4);
    let note = r["result"]["note"].as_str().unwrap_or_default();
    assert!(note.ends_with("; by rule: 20 filler, 10 hedge"), "{note}");
    Ok(())
}
