//! A guard requiring `gates.met` never passes over an unreadable gate
//! directory (#56): the hook and `guard status` agree, and the owed lines
//! name the invalid definition. Repairing the directory restores the pass.
#[path = "guard_status/helpers.rs"]
mod helpers;
mod support;

use helpers::*;
use support::{TestResult, world};

#[test]
fn gates_met_is_not_met_while_a_gate_directory_cannot_be_listed() -> TestResult {
    let w = world(&[])?;
    rule(&w.kb, "commit", COMMIT)?;
    current(&w)?;
    definition(&w, "unit", UNIT)?;
    assert_eq!(check(&w, "unit")?, 0);
    parity(&w, "pass")?;
    put(&w.code.join(".writfold/gates"), "not a directory\n")?;
    let denied = parity(&w, "deny")?;
    let owed = row(&denied, "commit")?["more"].to_string();
    assert!(
        owed.contains("invalid receipts or definitions: 1"),
        "{owed}"
    );
    std::fs::remove_file(w.code.join(".writfold/gates"))?;
    parity(&w, "pass")?;
    Ok(())
}
