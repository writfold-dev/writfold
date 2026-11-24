//! `resume --packet` (#109): required rows first, cut to the budget by the
//! proved kernel; `packet=complete` only with every required row kept; a
//! required row that does not fit is `answer=unknown reason=budget`.
mod support;

use std::path::Path;
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::GateVerb;

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn define(dir: &Path, id: &str, argv: &[&str]) -> TestResult {
    let gates = dir.join(".writfold/gates");
    std::fs::create_dir_all(&gates)?;
    let argv = serde_json::to_string(argv)?;
    let text = format!(
        "schema = \"writfold-gate/1\"\nid = \"{id}\"\nversion = 1\nclass = \"mechanical\"\nargv = {argv}\ntimeout_seconds = 60\nscope = [\"src/ledger.rs\"]\n"
    );
    std::fs::write(gates.join(format!("{id}.toml")), text)?;
    Ok(())
}

fn packet(w: &World, budget: usize) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Packet {
        run: Some(RUN.into()),
    });
    request.cwd = w.code.clone();
    request.now = Some(NOW + 120);
    request.budget = budget;
    w.run(&request)
}

fn gates(w: &World) -> TestResult {
    define(&w.kb, "ok", &["true"])?;
    define(&w.kb, "owed", &["false"])?;
    for id in ["ok", "owed"] {
        let mut request = w.request(Verb::Gate(GateVerb::Run {
            id: id.into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        }));
        request.now = Some(NOW + 60);
        w.run(&request)?;
    }
    Ok(())
}

fn trailer(text: &str) -> &str {
    text.lines().last().unwrap_or_default()
}

#[test]
fn required_rows_come_first_and_a_roomy_packet_is_complete() -> TestResult {
    let w = world(&[])?;
    gates(&w)?;
    let (text, exit) = packet(&w, 4000)?;
    let t = trailer(&text);
    assert!(t.contains("packet=complete"), "{text}");
    assert!(
        t.contains("required_omitted=0 optional_omitted=0"),
        "{text}"
    );
    assert_ne!(exit, 2, "{text}");
    // The owed gate is required, so it comes before the satisfied one.
    let owed = text.find("gate owed [failed]").ok_or(text.clone())?;
    let ok = text.find("gate ok [satisfied]").ok_or(text.clone())?;
    assert!(owed < ok, "{text}");
    assert!(text.starts_with("run "), "{text}");
    Ok(())
}

#[test]
fn optional_rows_are_dropped_first_and_a_missing_required_row_is_unknown() -> TestResult {
    let w = world(&[])?;
    gates(&w)?;
    // The smallest budget that holds every row; one token less drops
    // only the last row, which is optional.
    let (mut lo, mut hi) = (1, 4000);
    while lo < hi {
        let mid = usize::midpoint(lo, hi);
        let (text, _) = packet(&w, mid)?;
        if trailer(&text).contains("optional_omitted=0 ")
            && !trailer(&text).contains("answer=unknown")
        {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    let tight = packet(&w, lo - 1)?;
    let (text, exit) = tight;
    assert!(trailer(&text).contains("packet=complete"), "{text}");
    assert!(!text.contains("gate ok [satisfied]"), "{text}");
    assert!(text.contains("gate owed [failed]"), "{text}");
    assert_ne!(exit, 2, "{text}");

    let (text, exit) = packet(&w, 5)?;
    let t = trailer(&text);
    assert!(t.contains("answer=unknown reason=budget"), "{text}");
    assert!(t.contains("packet=partial"), "{text}");
    assert!(
        text.contains(&format!("writfold kb resume {RUN}")),
        "{text}"
    );
    assert_eq!(exit, 2, "{text}");
    Ok(())
}
