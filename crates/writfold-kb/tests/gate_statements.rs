//! G2's statement classes on G1 gates: an `attested` gate runs nothing and
//! is met only by `gate attest`; `gate waive` sets any gate aside with a
//! reason. Both show as themselves in `gate status`, `gate lines` and
//! `resume`, never as satisfied, and stop standing when the content in
//! the gate's scope moves.
mod support;

use std::path::Path;
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn write(dir: &Path, id: &str, text: &str) -> TestResult {
    let gates = dir.join(".writfold/gates");
    std::fs::create_dir_all(&gates)?;
    std::fs::write(gates.join(format!("{id}.toml")), text)?;
    Ok(())
}

fn gate(w: &World, verb: GateVerb, at: i64) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Gate(verb));
    request.now = Some(at);
    w.run(&request)
}

fn say(
    w: &World,
    kind: Kind,
    id: &str,
    text: &str,
) -> writfold_kb::error::Result<writfold_kb::emit::Output> {
    let verb = GateVerb::Say(Say {
        kind,
        id: id.into(),
        text: text.into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    writfold_kb::run(&w.request(Verb::Gate(verb)))
}

fn status(w: &World) -> TestResult<(String, u8)> {
    let verb = GateVerb::Status {
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    gate(w, verb, NOW + 3600)
}

#[test]
fn attested_and_waived_gates_stand_as_themselves() -> TestResult {
    let w = world(&[])?;
    write(
        &w.kb,
        "unit",
        "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"false\"]\ntimeout_seconds = 30\nscope = [\"src/ledger.rs\"]\n",
    )?;
    write(
        &w.kb,
        "review",
        "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\n",
    )?;
    let run = |id: &str| GateVerb::Run {
        id: id.into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    let (text, exit) = gate(&w, run("unit"), NOW)?;
    assert_eq!(exit, 2, "{text}");
    let refused = writfold_kb::run(&w.request(Verb::Gate(run("review"))));
    assert!(refused.is_err_and(|e| e.to_string().contains("is attested")));
    let refused = say(&w, Kind::Attested, "unit", "it looks fine");
    assert!(refused.is_err_and(|e| e.to_string().contains("is a mechanical check")));
    let (text, exit) = status(&w)?;
    assert!(
        exit == 2 && text.contains("gate unit [failed]") && text.contains("gate review [open]"),
        "{text}"
    );
    say(
        &w,
        Kind::Waived,
        "unit",
        "fails on this host only; CI covers it",
    )?;
    say(&w, Kind::Attested, "review", "read the diff")?;
    shown_as_themselves(&w)?;
    let mut request = w.request(Verb::Resume {
        run: Some(RUN.into()),
    });
    request.cwd = w.code.clone();
    let (resumed, _) = w.run(&request)?;
    assert!(
        resumed.contains("gate unit [waived]") && resumed.contains("gate review [attested]"),
        "{resumed}"
    );
    // The content in scope moves: neither statement stands any more.
    std::fs::write(w.code.join("src/ledger.rs"), "pub fn write() {}\n")?;
    let (text, exit) = status(&w)?;
    assert_eq!(exit, 2, "{text}");
    assert!(
        text.contains("gate unit [stale]") && text.contains("gate review [stale]"),
        "{text}"
    );
    assert!(
        text.contains("a waiver exists, but for other content"),
        "{text}"
    );
    Ok(())
}

/// Status and lines once `unit` is waived and `review` attested.
fn shown_as_themselves(w: &World) -> TestResult {
    let (text, exit) = status(w)?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("gate unit [waived] mechanical fail exit=1"),
        "{text}"
    );
    assert!(
        text.contains("  waived: fails on this host only; CI covers it (by owner,"),
        "{text}"
    );
    assert!(
        text.contains("gate review [attested] attested: statements only"),
        "{text}"
    );
    assert!(
        text.contains("  attested: read the diff (by owner,"),
        "{text}"
    );
    assert!(
        text.contains("!waived") && text.contains("!attested") && !text.contains("[satisfied]")
    );
    assert!(
        text.contains("satisfied=0") && text.contains("attested=1 waived=1"),
        "{text}"
    );
    let (lines, exit) = gate(
        w,
        GateVerb::Lines {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
        NOW + 3600,
    )?;
    assert_eq!(exit, 0, "{lines}");
    assert!(
        lines.contains("0 of 2 satisfied, 1 attested, 1 waived"),
        "{lines}"
    );
    assert!(
        lines.contains("unit:   waived by owner: fails on this host only"),
        "{lines}"
    );
    Ok(())
}
