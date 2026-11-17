//! Shared steps for the plan-delivery tests: a world with the
//! `deliver-step` overlay, the plan and deliver verbs on its run, and the
//! goldens with candidate and event names made stable.
#![allow(dead_code)]
use crate::support::{NOW, TestResult, World, world};
use std::path::{Path, PathBuf};
use writfold_kb::Verb;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::Prepare;
use writfold_kb::gate::statement::Kind;
use writfold_kb::plan::PlanVerb;

pub const RUN: &str = "run-20261101T090000Z-a1b2c3";
/// A payload every check of `pr-comment` passes for the external audience.
pub const CLEAN: &str = "Batching is live in staging.\n\nThe flush is measured next.\n";
/// The same with an internal host the deny list refuses (line 3).
pub const LEAKY: &str =
    "Batching is live in staging.\n\nDetails are on build01.corp.example.com today.\n";

/// Candidate, receipt and event names end in six random hex digits.
pub fn stable(text: &str) -> String {
    let text = text.replace(RUN, "<RUN>");
    let names = regex::Regex::new(r"(\d{8}T\d{6}Z)-[0-9a-f]{6}").ok();
    let text = names.map_or(text.clone(), |re| {
        re.replace_all(&text, "$1-XXXXXX").into_owned()
    });
    text.replace("<RUN>", RUN)
}

pub fn golden(name: &str, text: &str, exit: u8) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let actual = format!("{}# exit={exit}\n", stable(text));
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(&file, &actual)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&file).map_err(|e| format!("{name}: {e}"))?;
    if expected != actual {
        return Err(
            format!("golden {name} differs\n--- expected\n{expected}--- actual\n{actual}").into(),
        );
    }
    Ok(())
}

pub fn put(w: &World, path: &str, text: &str) -> TestResult {
    let path = w.kb.join(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

/// The fixture with the delivery overlay.
pub fn fixture() -> TestResult<World> {
    world(&["deliver-step"])
}

pub fn call(w: &World, verb: Verb) -> TestResult<(String, u8)> {
    w.run(&w.request(verb))
}

pub fn set(w: &World, template: &str) -> TestResult<(String, u8)> {
    call(
        w,
        Verb::Plan(PlanVerb::Set {
            template: Some(template.into()),
            from: None,
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        }),
    )
}

pub fn status(w: &World, gates: bool) -> TestResult<(String, u8)> {
    call(
        w,
        Verb::Plan(PlanVerb::Status {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
            gates,
        }),
    )
}

pub fn advance(w: &World) -> TestResult<(String, u8)> {
    call(
        w,
        Verb::Plan(PlanVerb::Advance {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        }),
    )
}

pub fn deliver(w: &World, verb: DeliverVerb, at: i64) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Deliver(verb));
    request.now = Some(at);
    w.run(&request)
}

/// Prepare a candidate of `mode` from `text`; returns its ID.
pub fn prepare(w: &World, mode: &str, text: &str, at: i64) -> TestResult<String> {
    let from = w.base.join(format!("draft-{at}.md"));
    std::fs::write(&from, text)?;
    let before = ids(w)?;
    let p = Prepare {
        mode: mode.into(),
        from,
        audience: Some("external".into()),
        destination: Some("harbor#42".into()),
        run: Some(RUN.into()),
    };
    let (out, exit) = deliver(w, DeliverVerb::Prepare(p), at)?;
    assert_eq!(exit, 0, "{out}");
    ids(w)?
        .into_iter()
        .find(|id| !before.contains(id))
        .ok_or_else(|| format!("no new candidate\n{out}").into())
}

pub fn check(w: &World, id: &str, at: i64) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Check {
        candidate: Some(id.into()),
        check: None,
        run: Some(RUN.into()),
        mode: None,
    };
    deliver(w, verb, at)
}

/// `deliver attest` (`kind` attested) or `deliver waive`.
pub fn say(w: &World, kind: Kind, id: &str, check: &str, at: i64) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Say {
        kind,
        candidate: Some(id.into()),
        check: check.into(),
        text: "read before posting".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        mode: None,
    };
    deliver(w, verb, at)
}

pub fn deliver_status(w: &World) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Status {
        candidate: None,
        run: Some(RUN.into()),
        all: false,
    };
    deliver(w, verb, NOW + 9000)
}

/// The run's candidate IDs, by name.
pub fn ids(w: &World) -> TestResult<Vec<String>> {
    let dir = w.kb.join(format!("runs/{RUN}/deliveries"));
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    Ok(out)
}

pub fn payload(w: &World, id: &str) -> PathBuf {
    w.kb.join(format!("runs/{RUN}/deliveries/{id}/candidate.md"))
}

/// The plan event files of the run, by name.
pub fn events(w: &World) -> Vec<String> {
    let dir = w.kb.join(format!("runs/{RUN}/plan"));
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();
    names
}

/// The `met` entry for `gate` in the done record of `step`.
pub fn met(w: &World, step: &str, gate: &str) -> TestResult<serde_json::Value> {
    let name = events(w)
        .into_iter()
        .find(|n| n.starts_with("done-") && n.contains(step))
        .ok_or_else(|| format!("no done record for {step}"))?;
    let path = w.kb.join(format!("runs/{RUN}/plan/{name}"));
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let entries = value["met"].as_array().cloned().unwrap_or_default();
    entries
        .into_iter()
        .find(|m| m["gate"] == gate)
        .ok_or_else(|| format!("no {gate} in {name}").into())
}

/// A KB with `change` set and its first step (owing nothing) advanced.
pub fn at_delivery() -> TestResult<World> {
    let w = fixture()?;
    let (text, exit) = set(&w, "change")?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = advance(&w)?;
    assert!(
        exit == 0 && text.contains("change#1/implement [done]"),
        "{text}"
    );
    Ok(w)
}

/// The text of the row starting `prefix`, with its continuation lines.
pub fn row<'a>(text: &'a str, prefix: &str) -> Vec<&'a str> {
    let mut lines = text.lines().skip_while(|l| !l.starts_with(prefix));
    let first = lines.next().into_iter();
    first
        .chain(lines.take_while(|l| l.starts_with("  ")))
        .collect()
}
