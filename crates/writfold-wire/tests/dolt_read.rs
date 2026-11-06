mod dolt_cases;

use serde_json::{Value, json};
use writfold_domain::{RowLimit, SchemaEpoch, dolt::DoltReadPin};
use writfold_wire::{
    Budget, Limits,
    dolt::{self, Error},
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const COMMIT: &str = "0123456789abcdefghijklmnopqrstuv";
const GOLDEN: &str = concat!(
    "{\"commit\":\"0123456789abcdefghijklmnopqrstuv\",\"database\":\"db-1\",\"epoch\":\"epoch-1\",\"kb\":\"kb:é\",\"kind\":\"pin\",\"namespace\":\"writfold-dolt-read\",",
    "\"producer\":\"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\",",
    "\"query_digest\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\"realm\":\"realm-1\",\"schema\":1,",
    "\"schema_digest\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"storage_profile\":\"dolt-corpus-read-v1\"}"
);
fn limits() -> Result<Limits> {
    Ok(Limits::new(16_384, 16, 128, 512)?)
}
fn pin() -> Result<DoltReadPin> {
    Ok(DoltReadPin::new(
        "realm-1".parse()?,
        "kb:é".parse()?,
        "db-1".parse()?,
        COMMIT.parse()?,
        SchemaEpoch::new(
            "epoch-1".parse()?,
            "a".repeat(64).parse()?,
            "b".repeat(64).parse()?,
        ),
        "dolt-corpus-read-v1".parse()?,
        "c".repeat(64).parse()?,
    ))
}
fn changed(field: &str, value: Value) -> Result<Vec<u8>> {
    let mut object: Value = serde_json::from_str(GOLDEN)?;
    object[field] = value;
    Ok(serde_json::to_vec(&object)?)
}
fn metadata() -> Value {
    json!({"id":1,"native_commit":COMMIT,"realm":"realm-1","kb":"kb:é","database_id":"db-1",
        "epoch":"epoch-1","schema_digest":"a".repeat(64),"query_digest":"b".repeat(64),
        "storage_profile":"dolt-corpus-read-v1","producer":"c".repeat(64)})
}
fn node(id: &str) -> Value {
    json!({"id":id,"kind":"document","label":"é\r\n雪'); DROP TABLE wf_nodes; --","home_path":"","home_is_null":true})
}
fn edge(id: &str) -> Value {
    json!({"id":id,"source":"a","target":"z","kind":"related"})
}
fn rows(values: Vec<Value>) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&json!({"rows":values}))?)
}

#[test]
fn independent_pin_golden_and_exact_byte_boundary() -> Result {
    let expected = pin()?;
    assert_eq!(dolt::decode_pin(GOLDEN.as_bytes(), limits()?)?, expected);
    assert_eq!(
        dolt::encode_pin(&expected, GOLDEN.len())?,
        GOLDEN.as_bytes()
    );
    assert!(matches!(
        dolt::encode_pin(&expected, GOLDEN.len() - 1),
        Err(Error::Wire(writfold_wire::Error::Limit {
            budget: Budget::Bytes,
            ..
        }))
    ));
    let pretty = serde_json::to_vec_pretty(&serde_json::from_str::<Value>(GOLDEN)?)?;
    assert_eq!(dolt::decode_pin(&pretty, limits()?)?, expected);
    Ok(())
}

#[test]
fn pin_field_sets_types_and_nominal_domains_are_closed() -> Result {
    let object: Value = serde_json::from_str(GOLDEN)?;
    for field in object.as_object().ok_or("object required")?.keys() {
        let mut missing = object.clone();
        missing
            .as_object_mut()
            .ok_or("object required")?
            .remove(field);
        assert_eq!(
            dolt::decode_pin(&serde_json::to_vec(&missing)?, limits()?),
            Err(Error::InvalidShape)
        );
        if field != "schema" {
            assert!(matches!(
                dolt::decode_pin(&changed(field, Value::Null)?, limits()?),
                Err(Error::InvalidField { field: actual }) if actual == field
            ));
        }
    }
    assert_eq!(
        dolt::decode_pin(&changed("extra", json!(1))?, limits()?),
        Err(Error::InvalidShape)
    );
    for field in [
        "realm",
        "kb",
        "database",
        "commit",
        "epoch",
        "schema_digest",
        "query_digest",
        "storage_profile",
        "producer",
    ] {
        assert_eq!(
            dolt::decode_pin(&changed(field, json!(""))?, limits()?),
            Err(Error::InvalidIdentity { field })
        );
    }
    for invalid in [
        "HEAD",
        "0123456789ABCDEFGHIJKLMNOPQRSTUV",
        "'; SELECT 1; --",
    ] {
        assert_eq!(
            dolt::decode_pin(&changed("commit", json!(invalid))?, limits()?),
            Err(Error::InvalidIdentity { field: "commit" })
        );
    }
    assert_eq!(
        dolt::decode_pin(
            &changed("namespace", json!("writfold-storage-pin"))?,
            limits()?
        ),
        Err(Error::UnsupportedNamespace)
    );
    assert_eq!(
        dolt::decode_pin(&changed("kind", json!("release"))?, limits()?),
        Err(Error::UnsupportedKind)
    );
    for number in ["1.0", "1e0", "-1", "true", "\"1\"", "null"] {
        let bytes = GOLDEN.replace("\"schema\":1", &format!("\"schema\":{number}"));
        assert_eq!(
            dolt::decode_pin(bytes.as_bytes(), limits()?),
            Err(Error::InvalidField { field: "schema" })
        );
    }
    assert_eq!(
        dolt::decode_pin(&changed("schema", json!(2))?, limits()?),
        Err(Error::UnsupportedVersion)
    );
    let duplicate = GOLDEN.replacen('{', "{\"schema\":1,", 1);
    assert!(
        matches!(dolt::decode_pin(duplicate.as_bytes(), limits()?), Err(Error::Wire(writfold_wire::Error::DuplicateKey { key, .. })) if key == "schema")
    );
    Ok(())
}

#[test]
fn legacy_namespace_stays_separate() -> Result {
    let legacy = include_bytes!("../fixtures/storage/legacy-dolt.json");
    assert_eq!(
        dolt::decode_pin(legacy, limits()?),
        Err(Error::InvalidShape)
    );
    assert_eq!(
        writfold_wire::storage::decode_pin(GOLDEN.as_bytes(), limits()?),
        Err(writfold_wire::storage::StorageWireError::UnsupportedNamespace)
    );
    let old = writfold_wire::storage::decode_pin(legacy, limits()?)?;
    assert_eq!(
        writfold_wire::storage::encode_pin(&old, legacy.len())?,
        legacy
    );
    Ok(())
}

#[test]
fn scalar_cardinality_and_complete_metadata_identity() -> Result {
    assert_eq!(
        dolt::decode_metadata(&rows(vec![metadata()])?, limits()?)?,
        pin()?
    );
    assert_eq!(
        dolt::decode_root(&rows(vec![json!({"native_commit":COMMIT})])?, limits()?)?.as_str(),
        COMMIT
    );
    for count in [0, 2] {
        assert_eq!(
            dolt::decode_metadata(&rows(vec![metadata(); count])?, limits()?),
            Err(Error::RowCount)
        );
        assert_eq!(
            dolt::decode_root(
                &rows(vec![json!({"native_commit":COMMIT}); count])?,
                limits()?
            ),
            Err(Error::RowCount)
        );
    }
    assert_eq!(
        dolt::decode_metadata(b"{}", limits()?),
        Err(Error::InvalidShape)
    );
    assert_eq!(
        dolt::decode_root(b"{}", limits()?),
        Err(Error::InvalidShape)
    );
    for id in [json!(0), json!(2), json!(true), json!("1"), json!(1.0)] {
        let mut row = metadata();
        row["id"] = id;
        assert_eq!(
            dolt::decode_metadata(&rows(vec![row])?, limits()?),
            Err(Error::InvalidField { field: "id" })
        );
    }
    for (field, value, expected) in [
        (
            "native_commit",
            json!("HEAD"),
            Error::InvalidIdentity {
                field: "native_commit",
            },
        ),
        (
            "database_id",
            json!(""),
            Error::InvalidIdentity {
                field: "database_id",
            },
        ),
        (
            "realm",
            json!({"nested":[]}),
            Error::InvalidField { field: "realm" },
        ),
    ] {
        let mut row = metadata();
        row[field] = value;
        assert_eq!(
            dolt::decode_metadata(&rows(vec![row])?, limits()?),
            Err(expected)
        );
    }
    Ok(())
}

#[test]
fn every_decoder_applies_all_raw_budgets() -> Result {
    let inputs = [
        GOLDEN.as_bytes().to_vec(),
        rows(vec![metadata()])?,
        rows(vec![json!({"native_commit":COMMIT})])?,
        rows(vec![node("a")])?,
        rows(vec![edge("a")])?,
    ];
    for (index, bytes) in inputs.iter().enumerate() {
        for (bound, expected) in [
            (Limits::new(bytes.len() - 1, 16, 128, 512)?, Budget::Bytes),
            (Limits::new(16_384, 0, 128, 512)?, Budget::Depth),
            (Limits::new(16_384, 16, 0, 512)?, Budget::ContainerEntries),
            (Limits::new(16_384, 16, 128, 0)?, Budget::Nodes),
        ] {
            let error = match index {
                0 => dolt::decode_pin(bytes, bound).map(|_| ()),
                1 => dolt::decode_metadata(bytes, bound).map(|_| ()),
                2 => dolt::decode_root(bytes, bound).map(|_| ()),
                3 => dolt::decode_nodes(bytes, bound, RowLimit::new(1)?).map(|_| ()),
                _ => dolt::decode_edges(bytes, bound, RowLimit::new(1)?).map(|_| ()),
            };
            assert!(
                matches!(error, Err(Error::Wire(writfold_wire::Error::Limit { budget, .. })) if budget == expected),
                "decoder {index}: {expected:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn row_field_sets_and_scalar_containers_are_closed() -> Result {
    for field in ["id", "kind", "label", "home_path", "home_is_null"] {
        let mut row = node("a");
        row.as_object_mut().ok_or("object required")?.remove(field);
        assert_eq!(
            dolt::decode_nodes(&rows(vec![row])?, limits()?, RowLimit::new(1)?),
            Err(Error::InvalidShape)
        );
    }
    for field in ["id", "source", "target", "kind"] {
        let mut row = edge("a");
        row.as_object_mut().ok_or("object required")?.remove(field);
        assert_eq!(
            dolt::decode_edges(&rows(vec![row])?, limits()?, RowLimit::new(1)?),
            Err(Error::InvalidShape)
        );
    }
    let mut extra_node = node("a");
    extra_node["extra"] = json!(false);
    let mut extra_edge = edge("a");
    extra_edge["extra"] = json!(false);
    assert_eq!(
        dolt::decode_nodes(&rows(vec![extra_node])?, limits()?, RowLimit::new(1)?),
        Err(Error::InvalidShape)
    );
    assert_eq!(
        dolt::decode_edges(&rows(vec![extra_edge])?, limits()?, RowLimit::new(1)?),
        Err(Error::InvalidShape)
    );
    for field in [
        "id",
        "native_commit",
        "realm",
        "kb",
        "database_id",
        "epoch",
        "schema_digest",
        "query_digest",
        "storage_profile",
        "producer",
    ] {
        let mut row = metadata();
        row.as_object_mut().ok_or("object required")?.remove(field);
        assert_eq!(
            dolt::decode_metadata(&rows(vec![row])?, limits()?),
            Err(Error::InvalidShape)
        );
    }
    for bytes in [
        b"[]".as_slice(),
        b"{\"rows\":{}}",
        b"{\"rows\":[{}]}",
        b"{\"rows\":[[]]}",
        b"{\"rows\":[],\"extra\":0}",
    ] {
        assert_eq!(
            dolt::decode_metadata(bytes, limits()?),
            Err(Error::InvalidShape)
        );
        assert_eq!(
            dolt::decode_root(bytes, limits()?),
            Err(Error::InvalidShape)
        );
    }
    assert_eq!(
        dolt::decode_root(&rows(vec![json!({"native_commit":null})])?, limits()?),
        Err(Error::InvalidField {
            field: "native_commit"
        })
    );
    assert_eq!(
        dolt::decode_root(&rows(vec![json!({"native_commit":"HEAD"})])?, limits()?),
        Err(Error::InvalidIdentity {
            field: "native_commit"
        })
    );
    Ok(())
}
