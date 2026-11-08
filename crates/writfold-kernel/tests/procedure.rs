//! Structural procedure admission: an accepted catalog is retained exactly, and
//! each targeted mutation is refused with its rule, field and node attribution.
#[path = "procedure/declarations.rs"]
mod declarations;
#[path = "procedure/fixture.rs"]
mod fixture;
#[path = "procedure/graph.rs"]
mod graph;

use fixture::{TestResult, input};
use writfold_domain::engineering::procedure::ProcedureInput;
use writfold_kernel::procedure::{AdmissionError, Rule, validate};

pub(crate) fn refused(
    mutate: impl FnOnce(&mut ProcedureInput) -> TestResult,
    rule: Rule,
    field: &'static str,
    node: Option<u64>,
) -> TestResult {
    let mut supplied = input()?;
    mutate(&mut supplied)?;
    let retained = supplied.clone();
    let error = match validate(supplied) {
        Ok(_) => return Err(format!("expected {rule:?} refusal in {field}").into()),
        Err(error) => error,
    };
    let expected = AdmissionError {
        rule,
        field,
        node: node.map(fixture::id).transpose()?,
    };
    if error != expected {
        return Err(format!("expected {expected:?}, got {error:?} for {retained:?}").into());
    }
    Ok(())
}

#[test]
fn admits_and_retains_exact_supplied_catalog() -> TestResult {
    let supplied = input()?;
    let checked = validate(supplied.clone())?;
    assert_eq!(checked.input(), &supplied);
    assert_eq!(checked.procedure(), &supplied.procedure);
    assert_eq!(checked.into_input(), supplied);
    Ok(())
}

#[test]
fn admits_leaf_without_catalog() -> TestResult {
    let mut supplied = input()?;
    supplied.procedure = fixture::leaf()?;
    supplied.catalog.clear();
    validate(supplied)?;
    Ok(())
}

#[test]
fn refusal_display_names_rule_field_and_node() -> TestResult {
    let error = AdmissionError {
        rule: Rule::ForwardRoute,
        field: "route",
        node: Some(fixture::id(4)?),
    };
    assert_eq!(
        error.to_string(),
        "procedure ForwardRoute violation in route at node 4"
    );
    Ok(())
}
