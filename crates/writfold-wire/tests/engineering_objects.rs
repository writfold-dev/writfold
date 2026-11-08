use serde_json::{Value, json};
use writfold_domain::engineering::objects::RealmIdentity;
use writfold_wire::{
    Error, Limits,
    engineering::{
        blob_fingerprint,
        objects::{ObjectError, decode_identity, encode_identity},
    },
};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
const IDENTITY: &[u8] = br#"{"instance":"instance-a","kb":"kb-a","namespace":"writfold-realm","profile":"writfold-experimental-alpha/1","realm":"realm-a","schema":1}"#;
fn limits() -> Result<Limits> {
    Ok(Limits::new(4096, 32, 32, 128)?)
}

#[test]
fn raw_blob_identity_matches_independent_cbor_length_boundary_vectors() {
    // Independently generated with RFC 8949 byte-string headers and Python
    // stdlib SHA-256, not the Rust encoder or the shared header helper.
    for (length, expected) in [
        (
            0,
            "bb5bb04fecf2ef966f7f5ee7bc602a5e070e14ad12e0d073077596167c6aa81e",
        ),
        (
            23,
            "d08c1c47d882498768b1035db09b140ba6a9b1f28d083a4ab22632b6e38ba4f2",
        ),
        (
            24,
            "57d1d289929db74655a7422b1dd442415eb3122cb1eacdf32c1c578302e74369",
        ),
        (
            255,
            "21837793ded05a7095d6181dbf6f71f7eb3c069eb0b6a4811981c4292e3aaecf",
        ),
        (
            256,
            "4a1409c925385be0c8643103e59bc02b1c2cdcfafc242c70827446bfb10e11df",
        ),
        (
            65535,
            "d8dfc196b9f13ee95d6a3beb087a94a3825ee1890b6308c32fb01d12444ef80b",
        ),
        (
            65536,
            "2531d0504de06e00f9c058cd3519d91154deb1c25cd3c9fc4eac5d84a87fd96f",
        ),
    ] {
        assert_eq!(
            blob_fingerprint(&vec![b'a'; length]).to_string(),
            format!("wf1:blob:sha256:{expected}")
        );
    }
    assert_eq!(
        blob_fingerprint(b"\0\xff\r\n").to_string(),
        "wf1:blob:sha256:862c6e185c80687944a35ff3631bc5e9fa345f413b4368626bee68c3a3dfc4cd"
    );
}

#[test]
fn realm_identity_has_exact_closed_canonical_bytes() -> Result {
    let identity = RealmIdentity::new("realm-a".parse()?, "kb-a".parse()?, "instance-a".parse()?);
    assert_eq!(encode_identity(&identity, IDENTITY.len())?, IDENTITY);
    assert_eq!(decode_identity(IDENTITY, limits()?)?, identity);
    assert!(encode_identity(&identity, IDENTITY.len() - 1).is_err());
    let pretty = serde_json::to_vec_pretty(&serde_json::from_slice::<Value>(IDENTITY)?)?;
    assert_eq!(decode_identity(&pretty, limits()?)?, identity);
    Ok(())
}

#[test]
fn foreign_profiles_unknown_fields_and_ambiguous_identity_bytes_refuse() -> Result {
    let original: Value = serde_json::from_slice(IDENTITY)?;
    for (field, value) in [
        ("namespace", json!("writfold-storage-pin")),
        ("profile", json!("writfold-engineering/1")),
        ("schema", json!(2)),
        ("schema", json!(1.0)),
        ("schema", json!(true)),
        ("realm", json!("")),
        ("instance", Value::Null),
        ("extra", json!("unreviewed")),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        assert!(decode_identity(&serde_json::to_vec(&changed)?, limits()?).is_err());
    }
    let mut missing = original;
    missing
        .as_object_mut()
        .ok_or("identity object")?
        .remove("instance");
    assert!(decode_identity(&serde_json::to_vec(&missing)?, limits()?).is_err());
    let duplicate = std::str::from_utf8(IDENTITY)?.replace(
        "\"realm\":\"realm-a\"",
        "\"realm\":\"foreign\",\"realm\":\"realm-a\"",
    );
    assert!(matches!(
        decode_identity(duplicate.as_bytes(), limits()?),
        Err(ObjectError::Wire(Error::DuplicateKey { .. }))
    ));
    assert!(decode_identity(IDENTITY, Limits::new(IDENTITY.len() - 1, 32, 32, 128)?).is_err());
    Ok(())
}
