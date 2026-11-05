use serde_json::{Value, json};
use writfold_domain::engineering::dependency::LockError;
use writfold_wire::{
    Limits,
    engineering::dependency::{self, Error},
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn limits() -> TestResult<Limits> {
    Ok(Limits::new(65_536, 16, 256, 4096)?)
}

fn fixture() -> TestResult<Value> {
    Ok(serde_json::from_str(include_str!(
        "../fixtures/engineering/dependency-lock.json"
    ))?)
}

fn decode(input: &Value) -> TestResult<writfold_domain::engineering::dependency::DependencyLock> {
    Ok(dependency::decode(&serde_json::to_vec(input)?, limits()?)?)
}

fn digest(input: &Value) -> TestResult<String> {
    Ok(dependency::digest(&decode(input)?, limits()?)?.to_string())
}

fn refuses(input: &Value) -> TestResult {
    assert!(dependency::decode(&serde_json::to_vec(input)?, limits()?).is_err());
    Ok(())
}

fn lock_error(input: &Value, expected: LockError) -> TestResult {
    assert!(matches!(
        dependency::decode(&serde_json::to_vec(input)?, limits()?),
        Err(Error::Lock(actual)) if actual == expected
    ));
    Ok(())
}

#[test]
fn flat_lock_matches_independent_bytes_and_fingerprint() -> TestResult {
    let fixture = fixture()?;
    let lock = decode(&fixture["input"])?;
    let encoded = dependency::canonical(&lock, limits()?)?;
    let hex: String = encoded
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(hex, fixture["canonical_hex"]);
    assert_eq!(
        dependency::digest(&lock, limits()?)?.to_string(),
        fixture["fingerprint"]
    );
    assert_eq!(lock.realm().as_str(), "realm:test");
    assert_eq!(lock.packages()[0].exports()[0].name().as_str(), "main");
    assert_eq!(lock.edges()[0].source().export().as_str(), "main");
    Ok(())
}

#[test]
fn set_permutations_and_exact_duplicates_have_one_identity() -> TestResult {
    let fixture = fixture()?;
    let original = fixture["input"].clone();
    let mut input = original.clone();
    let packages = input["packages"].as_array_mut().ok_or("packages")?;
    packages.reverse();
    for package in packages.iter_mut() {
        let exports = package["exports"].as_array_mut().ok_or("exports")?;
        exports.reverse();
        exports.push(exports[0].clone());
    }
    packages.push(packages[0].clone());
    let edges = input["edges"].as_array_mut().ok_or("edges")?;
    edges.reverse();
    edges.push(edges[0].clone());
    let lock = decode(&input)?;
    assert_eq!(lock.packages().len(), 2);
    assert_eq!(lock.edges().len(), 3);
    assert_eq!(digest(&input)?, digest(&original)?);
    Ok(())
}

#[test]
fn conflicting_keys_and_unresolved_exact_endpoints_refuse() -> TestResult {
    let original = fixture()?["input"].clone();
    let mut input = original.clone();
    let mut conflicting = input["packages"][0].clone();
    conflicting["exports"][0]["rank"] = json!(3);
    input["packages"]
        .as_array_mut()
        .ok_or("packages")?
        .push(conflicting);
    lock_error(&input, LockError::ConflictingPackage)?;

    let mut input = original.clone();
    input["packages"][0]["exports"]
        .as_array_mut()
        .ok_or("exports")?
        .push(json!({"name":"main","rank":3}));
    lock_error(&input, LockError::ConflictingExport)?;

    let mut input = original.clone();
    input["edges"][0]["target"]["payload"] =
        json!(format!("wf1:package:sha256:{}", "33".repeat(32)));
    lock_error(&input, LockError::MissingPackage)?;
    let mut input = original.clone();
    input["edges"][0]["source"]["export"] = json!("missing");
    lock_error(&input, LockError::MissingExport)?;
    let mut input = original;
    input["packages"] = json!([]);
    lock_error(&input, LockError::EmptyPackages)
}

#[test]
fn reference_cycles_survive_but_call_recursion_and_equal_ranks_refuse() -> TestResult {
    let original = fixture()?["input"].clone();
    // The golden vector contains a two-package documentation cycle plus a descending call.
    assert_eq!(digest(&original)?, fixture()?["fingerprint"]);
    let mut input = original.clone();
    input["edges"][1]["kind"] = json!("invoke_procedure");
    lock_error(&input, LockError::RecursiveCall)?;
    let mut input = original.clone();
    let mut reverse = input["edges"][0].clone();
    let source = reverse["source"].take();
    reverse["source"] = reverse["target"].take();
    reverse["target"] = source;
    input["edges"].as_array_mut().ok_or("edges")?.push(reverse);
    lock_error(&input, LockError::RecursiveCall)?;
    let mut input = original.clone();
    input["edges"][0]["target"] = input["edges"][0]["source"].clone();
    lock_error(&input, LockError::RecursiveCall)?;
    let mut input = original;
    input["packages"][0]["exports"][0]["rank"] = json!(u64::MAX);
    input["packages"][1]["exports"][0]["rank"] = json!(0);
    assert_ne!(digest(&input)?, fixture()?["fingerprint"]);
    Ok(())
}

#[test]
fn realm_revision_export_and_typed_edge_remain_material() -> TestResult {
    let original = fixture()?["input"].clone();
    let expected = digest(&original)?;
    let mut input = original.clone();
    input["realm"] = json!("realm:other");
    assert_ne!(digest(&input)?, expected);
    let mut input = original.clone();
    input["packages"][1]["exports"][1]["rank"] = json!(9);
    assert_ne!(digest(&input)?, expected);
    for kind in [
        "require_context",
        "select_route",
        "execute_adapter",
        "produce_artifact",
    ] {
        let mut input = original.clone();
        input["edges"][1]["kind"] = json!(kind);
        assert_ne!(digest(&input)?, expected);
    }
    let mut input = original;
    let mut revision = input["packages"][0].clone();
    revision["payload"] = json!(format!("wf1:package:sha256:{}", "33".repeat(32)));
    input["packages"]
        .as_array_mut()
        .ok_or("packages")?
        .push(revision);
    let lock = decode(&input)?;
    assert_eq!(lock.packages().len(), 3);
    assert_ne!(dependency::digest(&lock, limits()?)?.to_string(), expected);
    Ok(())
}

#[test]
fn exact_text_and_empty_graph_do_not_receive_ambient_defaults() -> TestResult {
    let mut input = fixture()?["input"].clone();
    input["edges"] = json!([]);
    input["packages"] = json!([{"id":"é\r\n", "payload":format!("wf1:package:sha256:{}", "11".repeat(32)), "exports":[]}]);
    let first = digest(&input)?;
    assert_eq!(
        decode(&input)?.packages()[0].reference().id().as_str(),
        "é\r\n"
    );
    input["packages"][0]["id"] = json!("e\u{301}\r\n");
    assert_ne!(digest(&input)?, first);
    input["packages"][0]["id"] = json!("");
    refuses(&input)
}

#[test]
fn closed_schema_and_wrong_fingerprint_domains_refuse() -> TestResult {
    let original = fixture()?["input"].clone();
    for (field, value) in [
        ("namespace", json!("other")),
        ("schema_version", json!(2)),
        ("schema_version", json!("1")),
        ("kind", json!("package")),
        ("realm", json!(null)),
        ("extra", json!(false)),
    ] {
        let mut input = original.clone();
        input[field] = value;
        refuses(&input)?;
    }
    for pointer in ["/packages/0/payload", "/edges/0/source/payload"] {
        let mut input = original.clone();
        *input.pointer_mut(pointer).ok_or("pointer")? =
            json!(format!("wf1:blob:sha256:{}", "11".repeat(32)));
        refuses(&input)?;
    }
    for pointer in [
        "/packages/0",
        "/packages/0/exports/0",
        "/edges/0",
        "/edges/0/target",
    ] {
        let mut input = original.clone();
        input.pointer_mut(pointer).ok_or("pointer")?["extra"] = json!(false);
        refuses(&input)?;
    }
    let mut input = original.clone();
    input["edges"][0]["kind"] = json!("read");
    refuses(&input)?;
    let mut input = original;
    input["packages"][0]["exports"][0]["rank"] = json!(-1);
    refuses(&input)
}

#[test]
fn duplicate_keys_and_noninteger_schema_or_rank_refuse() -> TestResult {
    let original = serde_json::to_string(&fixture()?["input"])?;
    for raw in [
        original.replacen(
            "\"schema_version\":1",
            "\"schema_version\":1,\"schema_version\":1",
            1,
        ),
        original.replacen("\"schema_version\":1", "\"schema_version\":1.0", 1),
        original.replacen("\"schema_version\":1", "\"schema_version\":1e0", 1),
        original.replacen("\"rank\":2", "\"rank\":2.0", 1),
        original.replacen("\"rank\":2", "\"rank\":18446744073709551616", 1),
    ] {
        assert!(dependency::decode(raw.as_bytes(), limits()?).is_err());
    }
    Ok(())
}

#[test]
fn both_boundaries_enforce_all_resource_budgets() -> TestResult {
    let input = serde_json::to_vec(&fixture()?["input"])?;
    let lock = dependency::decode(&input, limits()?)?;
    let output = dependency::canonical(&lock, limits()?)?;
    assert!(dependency::decode(&input, Limits::new(input.len() - 1, 16, 256, 4096)?).is_err());
    assert!(
        dependency::canonical(
            &lock,
            Limits::new(output.as_bytes().len() - 1, 16, 256, 4096)?
        )
        .is_err()
    );
    for bounds in [
        Limits::new(65_536, 1, 256, 4096)?,
        Limits::new(65_536, 16, 1, 4096)?,
        Limits::new(65_536, 16, 256, 1)?,
    ] {
        assert!(dependency::decode(&input, bounds).is_err());
        assert!(dependency::canonical(&lock, bounds).is_err());
    }
    Ok(())
}
