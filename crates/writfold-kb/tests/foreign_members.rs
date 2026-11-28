//! Foreign members in two more readers (#190): a file the reader cannot
//! take in the guard rule directory and in the gate definition directory
//! is reported and counted, never read as "no such rule" or "no gate".
mod support;

use support::foreign::assert_foreign_counted;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::guard::GuardVerb;

fn put(w: &World, path: &str) -> TestResult {
    let file = w.kb.join(path);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, "this is not a definition\n")?;
    Ok(())
}

#[test]
fn a_foreign_file_among_guard_rules_is_counted_invalid() -> TestResult {
    assert_foreign_counted(
        &[".writfold/guards/foreign.toml"],
        || world(&[]),
        put,
        |w| w.run(&w.request(Verb::Guard(GuardVerb::Status))),
        |text, code| code == 2 && text.contains("invalid=1"),
    )
}

#[test]
fn a_foreign_file_among_gate_definitions_is_reported() -> TestResult {
    assert_foreign_counted(
        &[".writfold/gates/foreign.toml"],
        || world(&[]),
        put,
        |w| {
            let mut request = w.request(Verb::Policy);
            request.json = true;
            w.run(&request)
        },
        |text, code| code != 0 && text.contains("foreign.toml"),
    )
}
