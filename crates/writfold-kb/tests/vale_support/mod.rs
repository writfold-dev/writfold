//! A `prose.vale` world for the Vale tests (#117, #132): a `note` mode,
//! the profile `house` with its tree, and the stand-in executable in
//! `fixtures/vale/` (not Vale).
#![allow(
    dead_code,
    reason = "shared by several test binaries; each one uses only part of it"
)]
use crate::support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::Prepare;

pub const RUN: &str = "run-20261101T090000Z-a1b2c3";

pub fn standin() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/vale/bin/vale");
    writfold_kb::deliver::vale::use_program(path);
}

pub fn deliver(w: &World, verb: DeliverVerb, at: i64) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Deliver(verb));
    request.now = Some(at);
    w.run(&request)
}

pub fn prepare(w: &World, text: &str) -> TestResult<(String, u8)> {
    let from = w.base.join("message.md");
    std::fs::write(&from, text)?;
    let verb = DeliverVerb::Prepare(Prepare {
        mode: "note".into(),
        from,
        audience: Some("internal".into()),
        destination: Some("docs".into()),
        run: Some(RUN.into()),
        invocation: None,
    });
    deliver(w, verb, NOW)
}

pub fn checked(w: &World) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Check {
        candidate: None,
        check: None,
        run: Some(RUN.into()),
        mode: None,
    };
    deliver(w, verb, NOW + 60)
}

pub fn status(w: &World) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Status {
        candidate: None,
        run: Some(RUN.into()),
        all: false,
    };
    deliver(w, verb, NOW + 600)
}

pub fn write(w: &World, path: &str, text: &str) -> TestResult {
    let path = w.kb.join(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

/// The `note` mode with `checks = ["prose.vale"]` and `keys`.
pub fn mode(w: &World, keys: &str) -> TestResult {
    let text = format!(
        "schema = \"writfold-delivery-mode/1\"\nmode = \"note\"\nchecks = [\"prose.vale\"]\n{keys}"
    );
    write(w, ".writfold/deliveries/note.toml", &text)
}

pub const PROFILE: &str = "schema = \"writfold-vale-profile/1\"\nname = \"house\"\nversion = \"3.28.1\"\ncanary = \"This is TODO.\"\ntimeout_seconds = 2\n";
pub const INI: &str =
    "StylesPath = styles\nMinAlertLevel = suggestion\n\n[*.md]\nBasedOnStyles = House\n";
pub const HEDGES: &str =
    "extends: existence\nmessage: \"Avoid '%s'.\"\nlevel: suggestion\ntokens:\n  - very\n";

/// A world with the profile `house`, its tree and the `note` mode.
pub fn vale_world(keys: &str) -> TestResult<World> {
    standin();
    let w = world(&[])?;
    write(&w, ".writfold/deliveries/vale-house.toml", PROFILE)?;
    write(&w, ".writfold/vale/house/.vale.ini", INI)?;
    write(&w, ".writfold/vale/house/styles/House/Hedges.yml", HEDGES)?;
    mode(&w, &format!("vale = \"house\"\n{keys}"))?;
    Ok(w)
}

/// The JSON of every check receipt under the run's deliveries.
pub fn receipts(w: &World) -> TestResult<Vec<serde_json::Value>> {
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

/// Prepare `text`, check it, and return the check's exit status, its
/// output and its one receipt.
pub fn one(w: &World, text: &str) -> TestResult<(u8, String, serde_json::Value)> {
    prepare(w, text)?;
    let (shown, exit) = checked(w)?;
    let mut all = receipts(w)?;
    let r = all.pop().ok_or("no receipt")?;
    Ok((exit, shown, r))
}
