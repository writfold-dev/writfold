use super::*;
use writfold_wire::storage_operation::STORAGE_OPERATION_DOMAIN;

#[test]
fn closed_envelope_base_pin_target_and_approval_fields_refuse_malformed_input() -> Result {
    let original: Value = serde_json::from_slice(wrapped("selection")?.canonical_bytes())?;
    for path in ["", "/base", "/base/pin", "/target", "/operation"] {
        for extra in [false, true] {
            let mut value = original.clone();
            let object = value
                .pointer_mut(path)
                .and_then(Value::as_object_mut)
                .ok_or("object required")?;
            if extra {
                object.insert("extra".into(), Value::Null);
            } else {
                let key = object
                    .keys()
                    .next()
                    .ok_or("nonempty object required")?
                    .clone();
                object.remove(&key);
            }
            assert!(StorageOperation::decode(&bytes(&value)?, limits()?).is_err());
        }
    }
    let mut with_commit = original.clone();
    with_commit["target"]["commit"] = json!("01ARZ3NDEKTSV4RRFFQ69G5FAV");
    assert!(StorageOperation::decode(&reidentify(&mut with_commit)?, limits()?).is_err());
    let approval: Value =
        serde_json::from_slice(&fixture("storage-operation", "selection.approval.json")?)?;
    for (field, replacement) in [
        ("namespace", json!("kctx-operation-v3")),
        ("schema", json!(2)),
        ("schema", json!(1.0)),
        ("schema", json!("1")),
        ("schema", json!(true)),
        ("signature", json!("ff".repeat(63))),
        ("signature", json!("gg".repeat(64))),
        ("signature", json!([])),
        ("plan", json!("bad")),
        ("key", json!(null)),
        ("extra", json!(null)),
    ] {
        let mut changed = approval.clone();
        changed[field] = replacement;
        assert!(StorageApproval::decode(&bytes(&changed)?, limits()?).is_err());
    }
    let duplicate = String::from_utf8(bytes(&approval)?)?.replacen("{", "{\"schema\":1,", 1);
    assert!(StorageApproval::decode(duplicate.as_bytes(), limits()?).is_err());
    Ok(())
}

#[test]
fn unknown_versions_duplicate_keys_and_numeric_reinterpretations_refuse() -> Result {
    let original: Value = serde_json::from_slice(wrapped("selection")?.canonical_bytes())?;
    for (pointer, replacement) in [
        ("/namespace", json!("writfold-operation")),
        ("/schema", json!(2)),
        ("/schema", json!(1.0)),
        ("/schema", json!("1")),
        ("/schema", json!(true)),
        ("/target/backend", json!("legacy-dolt")),
        ("/target/native_format", json!(1.0)),
        ("/target/native_format", json!(0)),
        ("/target/native_format", json!(-1)),
        ("/target/native_format", json!("1")),
        ("/base/pin/schema", json!(2)),
        ("/base/pin/native_format", json!(1.0)),
        ("/mapping", json!(true)),
        ("/target/graph", json!("bad/graph")),
        ("/target/schema_digest", json!("A".repeat(64))),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).ok_or("field required")? = replacement;
        assert!(StorageOperation::decode(&reidentify(&mut changed)?, limits()?).is_err());
    }
    let text = String::from_utf8(bytes(&original)?)?;
    for (needle, replacement) in [
        ("\"namespace\":", "\"schema\":1,\"namespace\":"),
        ("\"graph\":", "\"graph\":\"Graph-01\",\"graph\":"),
        ("\"graph\":", "\"\\u0067raph\":\"Graph-01\",\"graph\":"),
        ("\"schema\":1", "\"schema\":1e0"),
        (
            "\"native_format\":1",
            "\"native_format\":18446744073709551616",
        ),
    ] {
        assert!(
            StorageOperation::decode(text.replacen(needle, replacement, 1).as_bytes(), limits()?)
                .is_err()
        );
    }
    Ok(())
}

fn metrics(value: &Value) -> (usize, usize, usize) {
    let mut pending = vec![(value, 0)];
    let (mut depth, mut entries, mut nodes) = (0, 0, 0);
    while let Some((value, parent_depth)) = pending.pop() {
        nodes += 1;
        match value {
            Value::Array(values) => {
                depth = depth.max(parent_depth + 1);
                entries = entries.max(values.len());
                pending.extend(values.iter().map(|value| (value, parent_depth + 1)));
            }
            Value::Object(values) => {
                depth = depth.max(parent_depth + 1);
                entries = entries.max(values.len());
                pending.extend(values.values().map(|value| (value, parent_depth + 1)));
            }
            _ => {}
        }
    }
    (depth, entries, nodes)
}

#[test]
fn prepare_and_decode_apply_identical_whole_envelope_budgets_including_prefix() -> Result {
    for name in ["bootstrap", "selection", "legacy-selection"] {
        let operation = wrapped(name)?;
        let value: Value = serde_json::from_slice(operation.canonical_bytes())?;
        let (depth, entries, nodes) = metrics(&value);
        let max_bytes = operation.canonical_bytes().len() + STORAGE_OPERATION_DOMAIN.len();
        let exact = Limits::new(max_bytes, depth, entries, nodes)?;
        assert_eq!(
            StorageOperation::decode(operation.canonical_bytes(), exact)?,
            operation
        );
        assert_eq!(
            StorageOperation::prepare(
                operation.operation().clone(),
                operation.base().clone(),
                operation.target().clone(),
                operation.mapping().clone(),
                exact
            )?,
            operation
        );
        for limits in [
            Limits::new(max_bytes - 1, depth, entries, nodes)?,
            Limits::new(max_bytes, depth - 1, entries, nodes)?,
            Limits::new(max_bytes, depth, entries - 1, nodes)?,
            Limits::new(max_bytes, depth, entries, nodes - 1)?,
            Limits::new(0, depth, entries, nodes)?,
        ] {
            assert!(StorageOperation::decode(operation.canonical_bytes(), limits).is_err());
            assert!(
                StorageOperation::prepare(
                    operation.operation().clone(),
                    operation.base().clone(),
                    operation.target().clone(),
                    operation.mapping().clone(),
                    limits
                )
                .is_err()
            );
        }
        let mut spaced = operation.canonical_bytes().to_vec();
        spaced.push(b' ');
        assert!(StorageOperation::decode(&spaced, exact).is_err());
    }
    Ok(())
}

#[test]
fn exact_large_native_format_and_escaped_target_kb_survive_canonicalization() -> Result {
    for format in [1, 9_007_199_254_740_993, u64::MAX] {
        let t = target()?;
        let t = StorageTarget::new(
            "kb:\n\u{0}\"\\é".parse()?,
            t.graph().clone(),
            t.epoch().clone(),
            t.profile().clone(),
            NativeFormatVersion::new(format)?,
            t.producer().clone(),
        );
        let operation =
            StorageOperation::prepare(inner("v3-admission")?, None, t, None, limits()?)?;
        let value: Value = serde_json::from_slice(operation.canonical_bytes())?;
        assert_eq!(value["target"]["native_format"].as_u64(), Some(format));
        assert_eq!(
            StorageOperation::decode(operation.canonical_bytes(), limits()?)?,
            operation
        );
    }
    Ok(())
}

#[test]
fn deep_untrusted_operation_is_refused_and_destroyed_on_small_stack() -> Result {
    let operation = wrapped("bootstrap")?;
    let original = String::from_utf8(operation.canonical_bytes().to_vec())?;
    let inner = std::str::from_utf8(operation.operation().canonical_bytes())?;
    let deep = format!("{}0{}", "[".repeat(512), "]".repeat(512));
    let malicious = original.replace(inner, &deep);
    let limits = Limits::new(1_000_000, 1024, 10_000, 100_000)?;
    let refused = std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || StorageOperation::decode(malicious.as_bytes(), limits).is_err())?
        .join()
        .map_err(|_| "untrusted nested operation exhausted native stack")?;
    assert!(refused);
    Ok(())
}
