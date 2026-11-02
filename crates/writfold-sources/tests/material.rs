use writfold_sources::{
    documents::parse_document,
    material::{MaterialIdentity, material_revision},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
// Exact retained tests/test_authored_design.py::LEGACY_DESIGN input.
const LEGACY: &str = include_str!("../fixtures/legacy-design.md");

#[test]
fn frozen_legacy_and_disjoint_design_hashes_match_python() -> TestResult {
    let legacy = parse_document(LEGACY)?;
    let historical = material_revision(&legacy)?;
    assert!(matches!(historical, MaterialIdentity::Legacy(_)));
    assert_eq!(
        historical.as_str(),
        "1947884be0e451a4959e78aef25b81ebf36cb09e4138b7d937166f8181d5e453"
    );
    let design = legacy.update_fields(&[
        ("workflow_version", Some("3")),
        ("target_source", Some("example/repo")),
    ])?;
    let revision = material_revision(&design)?;
    assert!(matches!(revision, MaterialIdentity::Design(_)));
    assert_eq!(
        revision.as_str(),
        "design-v3:64ac3c312822e38a0737a897434cdba9493a421f7dff49001744f3e1aad29a4a"
    );
    let old_approval = "64ac3c312822e38a0737a897434cdba9493a421f7dff49001744f3e1aad29a4a";
    assert_ne!(revision.as_str(), old_approval);
    assert_eq!(
        material_revision(&design.update_fields(&[("accepted_revision", Some(old_approval))])?)?,
        revision
    );
    assert_eq!(legacy.text(), LEGACY);
    Ok(())
}

#[test]
fn workflow_two_custom_binding_metadata_stays_material_not_authority() -> TestResult {
    let document = parse_document(LEGACY)?.update_fields(&[
        ("target_source", Some("historical custom source metadata")),
        ("binding_actor", Some("historical custom actor metadata")),
        ("binding_observed_at", Some("not an execution timestamp")),
    ])?;
    let before = document.text().to_owned();
    let revision = material_revision(&document)?;
    assert!(matches!(revision, MaterialIdentity::Legacy(_)));
    assert_eq!(
        revision.as_str(),
        "9d0c0842a2f403cc74c1562800bb5e504f23ddf85bae6fce8ef087c2c42a341b"
    );
    assert_eq!(document.text(), before);
    assert_eq!(
        document.field("binding_observed_at"),
        Some("not an execution timestamp")
    );
    Ok(())
}

#[test]
fn only_workflow_three_excludes_concrete_implementation_binding() -> TestResult {
    let legacy = parse_document(LEGACY)?;
    let design = legacy.update_fields(&[("workflow_version", Some("3"))])?;
    let legacy_revision = material_revision(&legacy)?;
    let design_revision = material_revision(&design)?;
    for key in [
        "target_revision",
        "binding_design_revision",
        "binding_source",
        "binding_target_revision",
        "binding_actor",
        "binding_observed_at",
    ] {
        assert_ne!(
            material_revision(&legacy.update_fields(&[(key, Some("new value"))])?)?,
            legacy_revision
        );
        assert_eq!(
            material_revision(&design.update_fields(&[(key, Some("new value"))])?)?,
            design_revision
        );
    }
    assert_ne!(
        material_revision(&design.update_fields(&[("target_source", Some("another/source"))])?)?,
        design_revision
    );
    Ok(())
}

#[test]
fn lifecycle_and_decision_log_do_not_reapprove_changed_material() -> TestResult {
    let document = parse_document(LEGACY)?.update_fields(&[("workflow_version", Some("3"))])?;
    let revision = material_revision(&document)?;
    for key in [
        "status",
        "updated",
        "decided_by",
        "acceptor",
        "executor",
        "verifier",
        "accepted_revision",
        "superseded_by",
        "swept_at",
        "landed",
        "last_actor",
        "regression_note",
    ] {
        assert_eq!(
            material_revision(&document.update_fields(&[(key, Some("recorded"))])?)?,
            revision
        );
    }
    assert_eq!(
        material_revision(&document.append_section("Decision log", "new bookkeeping")?)?,
        revision
    );
    assert_ne!(
        material_revision(&document.update_fields(&[("unknown_extension", Some("intent"))])?)?,
        revision
    );
    assert_ne!(
        material_revision(&document.update_section("Unknown section", "unrecognized material")?)?,
        revision
    );
    assert_ne!(
        material_revision(&document.update_section("Research", "changed design")?)?,
        revision
    );
    Ok(())
}

#[test]
fn field_order_and_newline_projection_preserve_section_order_and_unicode() -> TestResult {
    let first =
        "---\nworkflow_version: 3\nz: 三\na: λ\n---\n# Title\n\n## Context\n雪\n## Extra\nline\n";
    let reordered = first.replace("z: 三\na: λ", "a: λ\nz: 三");
    let revision = material_revision(&parse_document(first)?)?;
    assert_eq!(material_revision(&parse_document(reordered)?)?, revision);
    assert_eq!(
        material_revision(&parse_document(first.replace('\n', "\r\n"))?)?,
        revision
    );
    assert_ne!(
        material_revision(&parse_document(first.replace("# Title", "# Changed"))?)?,
        revision
    );
    assert_ne!(
        material_revision(&parse_document(first.replace(
            "## Context\n雪\n## Extra\nline",
            "## Extra\nline\n## Context\n雪"
        ))?)?,
        revision
    );
    assert_ne!(
        material_revision(&parse_document(first.replace("雪", "雪\u{2028}line"))?)?,
        revision
    );
    Ok(())
}

#[test]
fn unsupported_workflow_cannot_choose_a_material_domain() -> TestResult {
    let legacy = parse_document(LEGACY)?;
    for version in ["", "1", "03", "4", "true"] {
        let unknown = legacy.update_fields(&[("workflow_version", Some(version))])?;
        assert!(material_revision(&unknown).is_err());
    }
    Ok(())
}
