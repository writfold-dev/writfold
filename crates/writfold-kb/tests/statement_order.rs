//! #15: attestations and waivers are ordered by (`seq`, file name), not by
//! their recorded time, so a clock set backwards between two statements
//! cannot make the older one stand (`order::newest`, `newest_is_max`).
mod support;

use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn gate(w: &World, verb: GateVerb, at: i64) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Gate(verb));
    request.now = Some(at);
    w.run(&request)
}

fn attest(w: &World, text: &str, at: i64) -> TestResult<(String, u8)> {
    let verb = GateVerb::Say(Say {
        kind: Kind::Attested,
        id: "review".into(),
        text: text.into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    gate(w, verb, at)
}

#[test]
fn the_later_statement_stands_whatever_the_clock_said() -> TestResult {
    let w = world(&[])?;
    let gates = w.kb.join(".writfold/gates");
    std::fs::create_dir_all(&gates)?;
    std::fs::write(
        gates.join("review.toml"),
        "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\n",
    )?;
    attest(&w, "read it first", NOW + 600)?;
    // The clock moved back ten minutes before the second statement.
    attest(&w, "read it again after the fix", NOW)?;
    let verb = GateVerb::Status {
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    let (text, exit) = gate(&w, verb, NOW + 3600)?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("read it again after the fix"), "{text}");
    assert!(!text.contains("read it first"), "{text}");
    let dir = w.kb.join(format!("runs/{RUN}/statements"));
    let mut seqs: Vec<u64> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(entry?.path())?)?;
        seqs.push(value["seq"].as_u64().ok_or("seq")?);
    }
    seqs.sort_unstable();
    assert_eq!(seqs, [1, 2]);
    Ok(())
}
