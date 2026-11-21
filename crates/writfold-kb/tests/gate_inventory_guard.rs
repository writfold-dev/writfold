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

#[test]
fn gates_met_is_not_met_while_the_run_receipts_cannot_be_listed() -> TestResult {
    let w = world(&[])?;
    rule(&w.kb, "commit", COMMIT)?;
    current(&w)?;
    definition(&w, "unit", UNIT)?;
    assert_eq!(check(&w, "unit")?, 0);
    parity(&w, "pass")?;
    let receipt = walk(&w.kb.join("runs"))?
        .into_iter()
        .find(|p| {
            p.parent()
                .and_then(|d| d.file_name())
                .is_some_and(|n| n == "gates")
        })
        .ok_or("no receipt")?;
    let gates = receipt.parent().ok_or("parent")?.to_path_buf();
    let aside = gates.with_extension("aside");
    std::fs::rename(&gates, &aside)?;
    put(&gates, "not a directory\n")?;
    let denied = parity(&w, "deny")?;
    let owed = row(&denied, "commit")?["more"].to_string();
    assert!(
        owed.contains("invalid receipts or definitions: 1"),
        "{owed}"
    );
    std::fs::remove_file(&gates)?;
    std::fs::rename(&aside, &gates)?;
    parity(&w, "pass")?;
    Ok(())
}

fn walk(dir: &std::path::Path) -> TestResult<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            out.extend(walk(&path)?);
        } else {
            out.push(path);
        }
    }
    Ok(out)
}

#[test]
fn an_unlistable_guards_directory_is_reported_not_read_as_no_rules() -> TestResult {
    let w = world(&[])?;
    current(&w)?;
    let guards = w.kb.join(".writfold/guards");
    if guards.exists() {
        std::fs::remove_dir_all(&guards)?;
    }
    put(&guards, "not a directory\n")?;
    let out = hook(&w, false, "unlisted-guards");
    let said = format!("{}{}", out.stdout, out.stderr);
    assert!(said.contains("could not be listed"), "{said}");
    let mut request = w.request(writfold_kb::Verb::Policy);
    request.cwd = w.code.clone();
    let (text, exit) = w.run(&request)?;
    assert_eq!(exit, 2, "exit {exit}: {text}");
    // The file.unlisted diagnostic goes to the report (stderr in the CLI).
    assert!(text.contains("coverage=partial"), "{text}");
    std::fs::remove_file(&guards)?;
    let out = hook(&w, false, "listed-again");
    assert!(
        !out.stderr.contains("could not be listed"),
        "{}",
        out.stderr
    );
    Ok(())
}
