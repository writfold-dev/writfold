use serde_json::{Value, json};
use writfold_domain::{GraphCommitId, RowLimit};
use writfold_wire::{
    Limits,
    omnigraph::{self, Cell, Column, MutationBinding, Projection, QueryTarget, ScalarKind},
};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
const POINT: &[u8] = include_bytes!("../fixtures/omnigraph-native/point.json");
const CHANGE: &[u8] = include_bytes!("../fixtures/omnigraph-native/change.json");
const REFUSAL: &[u8] = include_bytes!("../fixtures/omnigraph-native/precondition.json");
const BEFORE: &str = "hb1.01M6VFBBYG3E72BGDHYSJ56AHQ.1.01M6VFBBYG3E72BGDHYSJ56AHQ";
const AFTER: &str = "hb1.01M6VFBBYG3E72BGDHYSJ56AHQ.2.01M6VFBBZ4NS79VJPB97NC26AV";
const COLUMNS: &[Column<'_>] = &[
    Column {
        name: "native_id",
        kind: ScalarKind::Text,
        nullable: false,
    },
    Column {
        name: "slug",
        kind: ScalarKind::Text,
        nullable: false,
    },
    Column {
        name: "name",
        kind: ScalarKind::Text,
        nullable: false,
    },
    Column {
        name: "ordinal",
        kind: ScalarKind::Integer,
        nullable: false,
    },
    Column {
        name: "active",
        kind: ScalarKind::Boolean,
        nullable: false,
    },
    Column {
        name: "score",
        kind: ScalarKind::Float,
        nullable: false,
    },
    Column {
        name: "note",
        kind: ScalarKind::Text,
        nullable: true,
    },
];

fn limits() -> Result<Limits> {
    Ok(Limits::new(16_384, 64, 128, 512)?)
}

fn point(bytes: &[u8]) -> Result<omnigraph::QueryRows> {
    Ok(omnigraph::decode_query(
        200,
        bytes,
        limits()?,
        RowLimit::new(8)?,
        "point",
        QueryTarget::Branch("retained-original-e1"),
        Projection::new(COLUMNS)?,
    )?)
}

fn change(bytes: &[u8], before: &GraphCommitId) -> Result<omnigraph::ConditionalChange> {
    Ok(omnigraph::decode_change(
        200,
        bytes,
        limits()?,
        MutationBinding {
            query: "change",
            actor: "act-writer",
            predecessor: before,
        },
    )?)
}

#[test]
fn observed_projection_preserves_scalar_types_and_nullable_omission() -> Result {
    let result = point(POINT)?;
    let expected = vec![
        Cell::Text("asset-000000".into()),
        Cell::Text("asset-000000".into()),
        Cell::Text("Asset λ 0".into()),
        Cell::Integer(0),
        Cell::Boolean(true),
        Cell::Float(37.25),
        Cell::Null,
    ];
    assert_eq!(result.rows().collect::<Vec<_>>(), vec![expected.as_slice()]);
    let mut explicit: Value = serde_json::from_slice(POINT)?;
    explicit["rows"][0]["note"] = Value::Null;
    assert_eq!(
        point(&serde_json::to_vec(&explicit)?)?
            .rows()
            .collect::<Vec<_>>(),
        vec![expected.as_slice()]
    );
    Ok(())
}

#[test]
fn projection_cannot_default_missing_required_fields_or_coerce_scalar_types() -> Result {
    let original: Value = serde_json::from_slice(POINT)?;
    for (field, value) in [
        ("ordinal", json!(0.0)),
        ("ordinal", json!(true)),
        ("active", json!(1)),
        ("score", json!(37)),
        ("name", Value::Null),
        ("note", json!(false)),
    ] {
        let mut wrong = original.clone();
        wrong["rows"][0][field] = value;
        assert!(point(&serde_json::to_vec(&wrong)?).is_err());
    }
    let mut missing = original.clone();
    missing["rows"][0]
        .as_object_mut()
        .ok_or("row object")?
        .remove("slug");
    assert!(point(&serde_json::to_vec(&missing)?).is_err());
    let mut extra = original;
    extra["rows"][0]["unknown"] = json!("not part of the query");
    assert!(point(&serde_json::to_vec(&extra)?).is_err());
    Ok(())
}

#[test]
fn read_binding_projection_and_counts_cannot_drift() -> Result {
    let original: Value = serde_json::from_slice(POINT)?;
    for (field, value) in [
        ("query_name", json!("another-query")),
        ("row_count", json!(2)),
        ("row_count", json!(1.0)),
        ("columns", json!(["native_id"])),
        ("target", json!({"branch":"main","snapshot":null})),
        ("graph_commit_id", json!("HEAD")),
        ("ranking", json!({"exhaustive":false})),
    ] {
        let mut wrong = original.clone();
        wrong[field] = value;
        assert!(point(&serde_json::to_vec(&wrong)?).is_err());
    }
    let projection = Projection::new(COLUMNS)?;
    assert!(
        omnigraph::decode_query(
            500,
            POINT,
            limits()?,
            RowLimit::new(8)?,
            "point",
            QueryTarget::Branch("retained-original-e1"),
            projection
        )
        .is_err()
    );
    let mut over = original.clone();
    over["row_count"] = json!(2);
    over["rows"].as_array_mut().ok_or("rows")?.push(json!({
        "native_id": "asset-000001", "slug": "asset-000001", "name": "Asset λ 1",
        "ordinal": 1, "active": false, "score": 38.25, "note": null
    }));
    assert!(matches!(
        omnigraph::decode_query(
            200,
            &serde_json::to_vec(&over)?,
            limits()?,
            RowLimit::new(1)?,
            "point",
            QueryTarget::Branch("retained-original-e1"),
            projection
        ),
        Err(omnigraph::NativeError::Budget)
    ));
    let before: GraphCommitId = BEFORE.parse()?;
    assert!(
        omnigraph::decode_query(
            200,
            POINT,
            limits()?,
            RowLimit::new(8)?,
            "point",
            QueryTarget::Snapshot(&before),
            projection
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn exact_snapshot_requires_both_target_and_effective_commit() -> Result {
    let mut snapshot: Value = serde_json::from_slice(POINT)?;
    let pin: GraphCommitId = snapshot["graph_commit_id"].as_str().ok_or("pin")?.parse()?;
    snapshot["target"] = json!({"branch":null,"snapshot":pin.as_str()});
    let encoded = serde_json::to_vec(&snapshot)?;
    let result = omnigraph::decode_query(
        200,
        &encoded,
        limits()?,
        RowLimit::new(8)?,
        "point",
        QueryTarget::Snapshot(&pin),
        Projection::new(COLUMNS)?,
    )?;
    assert_eq!(result.commit(), &pin);
    snapshot["graph_commit_id"] = json!(BEFORE);
    assert!(
        omnigraph::decode_query(
            200,
            &serde_json::to_vec(&snapshot)?,
            limits()?,
            RowLimit::new(8)?,
            "point",
            QueryTarget::Snapshot(&pin),
            Projection::new(COLUMNS)?
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn mutation_acknowledgment_is_bound_to_original_actor_query_and_predecessor() -> Result {
    let before = BEFORE.parse()?;
    let result = change(CHANGE, &before)?;
    assert_eq!(
        (
            result.commit().as_str(),
            result.affected_nodes(),
            result.affected_edges()
        ),
        (AFTER, 1, 0)
    );
    let original: Value = serde_json::from_slice(CHANGE)?;
    for (path, value) in [
        ("/actor_id", json!("someone-else")),
        ("/query_name", json!("another-query")),
        ("/branch", json!("other")),
        ("/commit/actor_id", json!("someone-else")),
        ("/commit/parent_commit_id", json!(AFTER)),
        ("/commit/graph_commit_id", json!(BEFORE)),
        ("/commit/published", json!(false)),
        ("/commit/kind", json!("unpublished")),
        ("/commit/merged_parent_commit_id", json!(BEFORE)),
        ("/affected_nodes", json!(true)),
    ] {
        let mut wrong = original.clone();
        *wrong.pointer_mut(path).ok_or("fixture field")? = value;
        assert!(change(&serde_json::to_vec(&wrong)?, &before).is_err());
    }
    assert!(
        omnigraph::decode_change(
            500,
            CHANGE,
            limits()?,
            MutationBinding {
                query: "change",
                actor: "act-writer",
                predecessor: &before,
            }
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn only_exact_precondition_refusal_can_describe_no_effect() -> Result {
    let before = BEFORE.parse()?;
    let result = omnigraph::decode_precondition_refusal(REFUSAL, limits()?, 412, &before)?;
    assert_eq!(result.actual().map(GraphCommitId::as_str), Some(AFTER));
    let original: Value = serde_json::from_slice(REFUSAL)?;
    for (path, value) in [
        ("/code", json!("recovery_required")),
        ("/disposition/effects", json!("unknown")),
        ("/disposition/action", json!("reconcile")),
        ("/precondition_failure/expected", json!(AFTER)),
        ("/precondition_failure/actual", json!(BEFORE)),
    ] {
        let mut wrong = original.clone();
        *wrong.pointer_mut(path).ok_or("fixture field")? = value;
        assert!(
            omnigraph::decode_precondition_refusal(
                &serde_json::to_vec(&wrong)?,
                limits()?,
                412,
                &before
            )
            .is_err()
        );
    }
    assert!(omnigraph::decode_precondition_refusal(REFUSAL, limits()?, 409, &before).is_err());
    Ok(())
}

#[test]
fn duplicate_keys_and_deep_late_values_are_not_validated_by_earlier_rows() -> Result {
    let duplicate =
        std::str::from_utf8(POINT)?.replace("\"row_count\":1", "\"row_count\":0,\"row_count\":1");
    assert!(matches!(
        point(duplicate.as_bytes())
            .err()
            .and_then(|error| error.downcast::<omnigraph::NativeError>().ok())
            .as_deref(),
        Some(omnigraph::NativeError::Json(
            writfold_wire::Error::DuplicateKey { .. }
        ))
    ));
    let mut deep: Value = Value::Null;
    for _ in 0..40 {
        deep = json!([deep]);
    }
    let mut wrong: Value = serde_json::from_slice(POINT)?;
    wrong["rows"][0]["note"] = deep;
    assert!(point(&serde_json::to_vec(&wrong)?).is_err());
    Ok(())
}

#[test]
fn omitted_and_null_actual_both_represent_no_native_head() -> Result {
    let before = BEFORE.parse()?;
    let mut body: Value = serde_json::from_slice(REFUSAL)?;
    body["precondition_failure"]
        .as_object_mut()
        .ok_or("precondition object")?
        .remove("actual");
    let omitted = omnigraph::decode_precondition_refusal(
        &serde_json::to_vec(&body)?,
        limits()?,
        412,
        &before,
    )?;
    assert!(omitted.actual().is_none());
    body["precondition_failure"]["actual"] = Value::Null;
    let explicit_null = omnigraph::decode_precondition_refusal(
        &serde_json::to_vec(&body)?,
        limits()?,
        412,
        &before,
    )?;
    assert!(explicit_null.actual().is_none());
    Ok(())
}
