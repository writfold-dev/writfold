//! Differential agreement between Lean `validateProcedure` and Rust
//! `writfold_kernel::procedure::validate` on the exported Lean case corpus.
//!
//! Required direction: a Lean refusal is always a Rust refusal, and a Rust
//! admission is always Lean-admitted. Rust may be stricter; any Lean-valid case
//! that Rust refuses must be listed explicitly below. This is executable
//! agreement on finite cases, not a refinement proof.
#[path = "procedure_lean/embed.rs"]
mod embed;

use embed::Fallible;
use serde_json::Value;
use writfold_domain::engineering::procedure::ProcedureInput;
use writfold_kernel::procedure::validate;

const CASES: &str = include_str!("fixtures/procedure-lean-cases.json");

/// Lean-valid cases Rust is expected to refuse because of a stricter rule.
const RUST_STRICTER: &[&str] = &[];

fn rust_admits(case: &Value) -> Fallible<bool> {
    let mut registry = case
        .get("registry")
        .and_then(Value::as_array)
        .ok_or("registry")?
        .iter()
        .map(embed::signature)
        .collect::<Fallible<Vec<_>>>()?;
    registry.push(embed::gate_signature()?);
    let embedded = (|| -> Fallible<ProcedureInput> {
        let procedure = embed::procedure(case.get("procedure").ok_or("procedure")?, &registry)?;
        let catalog = case
            .get("catalog")
            .and_then(Value::as_array)
            .ok_or("catalog")?
            .iter()
            .map(|p| embed::procedure(p, &registry))
            .collect::<Fallible<Vec<_>>>()?;
        Ok(ProcedureInput {
            procedure,
            catalog,
            registry: registry.clone(),
        })
    })();
    // A value outside the Rust domain (for example zero cost) is a refusal.
    Ok(match embedded {
        Ok(input) => validate(input).is_ok(),
        Err(_) => false,
    })
}

#[test]
fn lean_and_rust_agree_on_every_exported_case() -> Fallible<()> {
    let document: Value = serde_json::from_str(CASES)?;
    if document.get("schema").and_then(Value::as_str) != Some("writfold-procedure-lean-cases/1") {
        return Err("unexpected Lean case schema".into());
    }
    let cases = document
        .get("cases")
        .and_then(Value::as_array)
        .ok_or("cases")?;
    let (mut lean_valid, mut lean_refused, mut violations, mut stricter) =
        (0, 0, Vec::new(), Vec::new());
    for case in cases {
        let name = case.get("name").and_then(Value::as_str).ok_or("name")?;
        let lean = case
            .get("lean_valid")
            .and_then(Value::as_bool)
            .ok_or("lean_valid")?;
        let rust = rust_admits(case)?;
        if lean {
            lean_valid += 1
        } else {
            lean_refused += 1
        }
        match (lean, rust) {
            (false, true) => violations.push(name),
            (true, false) if !RUST_STRICTER.contains(&name) => stricter.push(name),
            _ => {}
        }
    }
    if !violations.is_empty() {
        return Err(format!("Rust admitted Lean-refused cases: {violations:?}").into());
    }
    if !stricter.is_empty() {
        return Err(format!("Rust refused unlisted Lean-valid cases: {stricter:?}").into());
    }
    // Non-vacuity: both outcomes occur in the corpus.
    if lean_valid < 2 || lean_refused < 30 {
        return Err(format!("corpus too small: {lean_valid} valid, {lean_refused} refused").into());
    }
    Ok(())
}
