use serde_json::{Value, json};
use writfold_domain::{GraphCommitId, RowLimit};
use writfold_wire::{
    Limits,
    omnigraph::{NativeError, decode_complete_history, decode_retention},
};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
const HISTORY: &[u8] = include_bytes!("../fixtures/omnigraph-native/history.json");
const RETAIN: &[u8] = include_bytes!("../fixtures/omnigraph-native/retain.json");
const HEAD: &str = "hb1.01M6VZRYMBQGBC7VEJY756RNM7.2.01M6VZRYN4EMT4139091130RBB";
const BEFORE: &str = "hb1.01M6VZRYMBQGBC7VEJY756RNM7.1.01M6VZRYMBQGBC7VEJY756RNM7";
const ROOT: &str = "01M6VZRYGYKZAAY4JZARPD0BS6";
fn limits() -> Result<Limits> {
    Ok(Limits::new(16_384, 64, 128, 512)?)
}
fn history(value: &Value) -> Result<Vec<GraphCommitId>> {
    Ok(decode_complete_history(
        200,
        &serde_json::to_vec(value)?,
        limits()?,
        &HEAD.parse()?,
        RowLimit::new(16)?,
    )?)
}

#[test]
fn native_complete_history_preserves_the_exact_first_parent_chain() -> Result {
    let expected = vec![HEAD.parse()?, BEFORE.parse()?, ROOT.parse()?];
    assert_eq!(
        decode_complete_history(200, HISTORY, limits()?, &HEAD.parse()?, RowLimit::new(3)?)?,
        expected
    );
    assert!(matches!(
        decode_complete_history(200, HISTORY, limits()?, &HEAD.parse()?, RowLimit::new(2)?),
        Err(NativeError::Budget)
    ));
    assert!(
        decode_complete_history(200, HISTORY, limits()?, &BEFORE.parse()?, RowLimit::new(3)?)
            .is_err()
    );
    assert!(
        decode_complete_history(503, HISTORY, limits()?, &HEAD.parse()?, RowLimit::new(3)?)
            .is_err()
    );
    Ok(())
}

#[test]
fn partial_reordered_and_cyclic_histories_are_not_complete() -> Result {
    let original: Value = serde_json::from_slice(HISTORY)?;
    let mut paged = original.clone();
    paged["next_page_token"] = json!("more");
    assert!(history(&paged).is_err());
    let mut missing = original.clone();
    missing["commits"]
        .as_array_mut()
        .ok_or("commits")?
        .remove(1);
    assert!(history(&missing).is_err());
    let mut truncated = original.clone();
    truncated["commits"].as_array_mut().ok_or("commits")?.pop();
    assert!(history(&truncated).is_err());
    let mut reordered = original.clone();
    reordered["commits"]
        .as_array_mut()
        .ok_or("commits")?
        .swap(0, 1);
    assert!(history(&reordered).is_err());
    // All adjacent links agree, but one identity is assigned two materials.
    let mut cyclic = original;
    cyclic["commits"][1]["parent_commit_id"] = json!(HEAD);
    let mut repeated = cyclic["commits"][0].clone();
    repeated["parent_commit_id"] = json!(ROOT);
    cyclic["commits"]
        .as_array_mut()
        .ok_or("commits")?
        .insert(2, repeated);
    assert!(history(&cyclic).is_err());
    Ok(())
}

#[test]
fn unsupported_history_kinds_and_late_deep_fields_refuse() -> Result {
    let original: Value = serde_json::from_slice(HISTORY)?;
    for (path, value) in [
        ("/commits/0/published", json!(false)),
        ("/commits/0/kind", json!("unpublished")),
        ("/commits/0/merged_parent_commit_id", json!(ROOT)),
        ("/commits/0/root_commit_id", json!(HEAD)),
        ("/commits/2/graph_manifest_version", json!(true)),
        ("/commits/2/created_at", json!(1.0)),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(path).ok_or("fixture field")? = value;
        assert!(history(&changed).is_err());
    }
    let mut deep = Value::Null;
    for _ in 0..40 {
        deep = json!([deep]);
    }
    let mut late = original;
    late["commits"][2]["actor_id"] = deep;
    assert!(history(&late).is_err());
    Ok(())
}

#[test]
fn retention_acknowledgment_cannot_change_actor_branch_or_original_pin() -> Result {
    let before = BEFORE.parse()?;
    assert_eq!(
        decode_retention(
            200,
            RETAIN,
            limits()?,
            "retained-native-smoke",
            "act-writer",
            &before
        )?,
        before
    );
    let original: Value = serde_json::from_slice(RETAIN)?;
    for (key, value) in [
        ("actor_id", json!("someone-else")),
        ("name", json!("different")),
        ("from", json!("feature")),
        ("from_commit_id", json!(HEAD)),
        ("uri", Value::Null),
    ] {
        let mut changed = original.clone();
        changed[key] = value;
        assert!(
            decode_retention(
                200,
                &serde_json::to_vec(&changed)?,
                limits()?,
                "retained-native-smoke",
                "act-writer",
                &before
            )
            .is_err()
        );
    }
    assert!(
        decode_retention(
            409,
            RETAIN,
            limits()?,
            "retained-native-smoke",
            "act-writer",
            &before
        )
        .is_err()
    );
    Ok(())
}
