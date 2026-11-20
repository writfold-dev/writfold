//! `kb policy --paths GATE` (policy spec 2.5, build step 14; #10): the
//! paths a gate's scope covers at the subject's present state.
mod checklist;
mod support;

use checklist::{UNIT, put};
use support::{TestResult, World, world};
use writfold_kb::Verb;

fn paths(w: &World, gate: &str) -> writfold_kb::error::Result<writfold_kb::emit::Output> {
    let mut req = w.request(Verb::PolicyPaths { gate: gate.into() });
    req.cwd = w.code.clone();
    writfold_kb::run(&req)
}

#[test]
fn a_gates_scope_is_listed_path_by_path() -> TestResult {
    let w = world(&[])?;
    let wide = UNIT.replace(
        "scope = [\"src/ledger.rs\"]",
        "scope = [\"src/**\", \"!src/batch.rs\"]",
    );
    put(&w.kb.join(".writfold/gates/unit.toml"), &wide)?;
    let out = paths(&w, "unit")?;
    assert_eq!(out.exit, 0, "{}", out.text);
    assert!(out.text.contains("path src/ledger.rs"), "{}", out.text);
    assert!(
        !out.text.contains("path src/batch.rs"),
        "excluded: {}",
        out.text
    );
    assert!(
        out.text
            .contains("Writfold does not infer what else the check depends on"),
        "{}",
        out.text
    );
    // An untracked file in scope is listed: the present state, not HEAD.
    std::fs::write(w.code.join("src/new.rs"), "fn x() {}\n")?;
    let out = paths(&w, "unit")?;
    assert!(out.text.contains("path src/new.rs"), "{}", out.text);
    // An unknown gate is refused, naming the verb that lists them.
    let why = match paths(&w, "nope") {
        Ok(out) => format!("not refused: {}", out.text),
        Err(e) => e.to_string(),
    };
    assert!(why.contains("no gate nope is defined"), "{why}");
    Ok(())
}
