use std::error::Error;
use writfold_domain::{
    ByteOffset, Dependency, ObsOutcome, Observation,
    citation::{CoordinateUnit, Selector},
};
use writfold_wire::{Limits, RawJson, canonical, citation::*, engineering::cbor::RawCbor};

type TestResult<T = (), E = Box<dyn Error>> = Result<T, E>;
const BINDING: &[u8] = include_bytes!("../fixtures/citations/binding.json");

fn limits() -> TestResult<Limits> {
    Ok(Limits::new(8192, 12, 64, 256)?)
}
fn fixture() -> TestResult<CitationDescriptor> {
    Ok(decode_descriptor(BINDING, limits()?)?)
}
fn changed(field: &str, value: serde_json::Value) -> TestResult<CitationDescriptor> {
    let mut object: serde_json::Value = serde_json::from_slice(BINDING)?;
    object[field] = value;
    Ok(decode_descriptor(&serde_json::to_vec(&object)?, limits()?)?)
}
fn bytes(text: &str) -> TestResult<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return Err("odd hex vector".into());
    }
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect()
}

#[test]
fn exact_descriptor_roundtrip_preserves_native_algorithms_role_and_selection() -> TestResult {
    let descriptor = fixture()?;
    let encoded = encode_descriptor(&descriptor, 8192)?;
    let expected = BINDING.strip_suffix(b"\n").ok_or("fixture newline")?;
    assert_eq!(encoded, expected);
    assert_eq!(
        canonical(&RawJson::parse(&encoded, limits()?)?, 8192)?,
        encoded
    );
    assert_eq!(decode_descriptor(&encoded, limits()?)?, descriptor);
    let original = b"line\n";
    let binding = descriptor.bind(original)?;
    assert_eq!(binding.expected(), original);
    assert_eq!(binding.expected().as_ptr(), original.as_ptr());
    assert_eq!(binding.role().as_str(), "support");
    assert_eq!(binding.selector().first().get(), 0);
    assert_eq!(binding.selector().last().get(), 5);
    assert!(matches!(
        descriptor.bind(b"LINE\n"),
        Err(CitationWireError::SelectionDigest)
    ));
    assert!(matches!(
        descriptor.bind(b"line"),
        Err(CitationWireError::SelectionLength)
    ));
    assert!(matches!(
        encode_descriptor(&descriptor, encoded.len() - 1),
        Err(CitationWireError::Wire(writfold_wire::Error::Limit { .. }))
    ));
    Ok(())
}

#[test]
fn independent_canonical_vectors_bind_all_material_and_separate_domains() -> TestResult {
    let descriptor = fixture()?;
    let vectors: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/citations/fingerprints.json"))?;
    let rows = vectors["vectors"].as_array().ok_or("vectors array")?;
    let representation = representation_fingerprint(descriptor.source(), limits()?)?;
    let binding = binding_fingerprint(&descriptor, limits()?)?;
    assert_eq!(
        representation.as_str(),
        rows[0]["digest"].as_str().ok_or("representation digest")?
    );
    assert_eq!(
        binding.as_str(),
        rows[1]["digest"].as_str().ok_or("binding digest")?
    );
    let correspondence_bytes = bytes(
        rows[2]["canonical_hex"]
            .as_str()
            .ok_or("correspondence bytes")?,
    )?;
    let correspondence = RawCbor::parse(&correspondence_bytes, limits()?)?;
    assert_eq!(
        correspondence_payload_fingerprint(&correspondence)?.as_str(),
        rows[2]["digest"].as_str().ok_or("correspondence digest")?
    );
    let representation_bytes = bytes(
        rows[0]["canonical_hex"]
            .as_str()
            .ok_or("representation bytes")?,
    )?;
    assert_ne!(
        correspondence_payload_fingerprint(&RawCbor::parse(&representation_bytes, limits()?)?)?
            .as_str(),
        representation.as_str()
    );
    for (field, value) in [
        ("id", "citation-2"),
        ("release", "release-2"),
        ("context", "other-context"),
        ("role", "contrary"),
    ] {
        let different = changed(field, value.into())?;
        assert_eq!(
            representation_fingerprint(different.source(), limits()?)?,
            representation
        );
        assert_ne!(binding_fingerprint(&different, limits()?)?, binding);
    }
    assert!(binding_fingerprint(&descriptor, Limits::new(1, 12, 64, 256)?).is_err());
    Ok(())
}

#[test]
fn wrong_kind_profile_units_and_algorithm_cannot_be_silently_reinterpreted() -> TestResult {
    let original: serde_json::Value = serde_json::from_slice(BINDING)?;
    for (pointer, replacement) in [
        ("/kind", serde_json::json!("claim")),
        ("/schema", serde_json::json!(2)),
        ("/schema", serde_json::json!(1.0)),
        (
            "/representation/profile",
            serde_json::json!("native-docs/1"),
        ),
        (
            "/representation/revision/algorithm",
            serde_json::json!("sha256"),
        ),
        (
            "/representation/blob/algorithm",
            serde_json::json!("unknown"),
        ),
        (
            "/representation/capture_digest",
            serde_json::json!("2".repeat(40)),
        ),
        ("/selector/unit", serde_json::json!("utf16")),
        ("/selector/first", serde_json::json!(6)),
        ("/selector/last", serde_json::json!(-1)),
        ("/selector/last", serde_json::json!(5.0)),
        ("/representation/path", serde_json::json!("../outside")),
    ] {
        let mut value = original.clone();
        *value.pointer_mut(pointer).ok_or("fixture pointer")? = replacement;
        assert!(
            decode_descriptor(&serde_json::to_vec(&value)?, limits()?).is_err(),
            "{pointer}"
        );
    }
    let mut unrelated = original.clone();
    unrelated["passed"] = serde_json::json!(true);
    assert!(matches!(
        decode_descriptor(&serde_json::to_vec(&unrelated)?, limits()?),
        Err(CitationWireError::Shape)
    ));
    let mut kind = original;
    kind["kind"] = serde_json::json!("claim");
    assert!(matches!(
        decode_descriptor(&serde_json::to_vec(&kind)?, limits()?),
        Err(CitationWireError::UnsupportedKind)
    ));
    let duplicate = std::str::from_utf8(BINDING)?.replacen("{", "{\"context\":\"other\",", 1);
    assert!(matches!(
        decode_descriptor(duplicate.as_bytes(), limits()?),
        Err(CitationWireError::Wire(
            writfold_wire::Error::DuplicateKey { .. }
        ))
    ));
    Ok(())
}

#[test]
fn same_bytes_at_another_position_have_a_different_binding_identity() -> TestResult {
    let descriptor = fixture()?;
    let later = CitationDescriptor::new(
        descriptor.id().clone(),
        descriptor.release().clone(),
        descriptor.context().clone(),
        descriptor.role().clone(),
        descriptor.source().clone(),
        Selector::new(
            ByteOffset::new(5)?,
            ByteOffset::new(10)?,
            CoordinateUnit::Byte,
        )
        .map_err(|_| "selector")?,
        descriptor.selection_digest().clone(),
    )?;
    assert_eq!(
        descriptor.bind(b"line\n")?.expected(),
        later.bind(b"line\n")?.expected()
    );
    assert_ne!(
        binding_fingerprint(&descriptor, limits()?)?,
        binding_fingerprint(&later, limits()?)?
    );
    let empty = CitationDescriptor::new(
        descriptor.id().clone(),
        descriptor.release().clone(),
        descriptor.context().clone(),
        descriptor.role().clone(),
        descriptor.source().clone(),
        Selector::new(
            ByteOffset::new(5)?,
            ByteOffset::new(5)?,
            CoordinateUnit::Byte,
        )
        .map_err(|_| "selector")?,
        selection_digest(b"")?,
    )?;
    assert_eq!(empty.bind(b"")?.expected(), b"");
    assert_ne!(
        binding_fingerprint(&descriptor, limits()?)?,
        binding_fingerprint(&empty, limits()?)?
    );
    Ok(())
}

#[test]
fn native_blob_ids_remain_distinct_from_retained_observation_content() -> TestResult {
    let original: serde_json::Value = serde_json::from_slice(BINDING)?;
    let capture = selection_digest(b"line\n")?;
    for (algorithm, width) in [("sha1", 40), ("sha256", 64)] {
        let mut value = original.clone();
        for (field, digit) in [("revision", "1"), ("blob", "2")] {
            value["representation"][field] =
                serde_json::json!({"algorithm": algorithm, "value": digit.repeat(width)});
        }
        let descriptor = decode_descriptor(&serde_json::to_vec(&value)?, limits()?)?;
        let reference = descriptor.source().reference();
        assert_eq!(reference.content().as_str(), capture.as_str());
        assert_eq!(descriptor.source().blob().as_str(), "2".repeat(width));
        let observation = Observation::new(
            "observation".parse()?,
            reference.repository().clone(),
            reference.revision().clone(),
            capture.as_str().parse()?,
            0.into(),
            ObsOutcome::Ok,
        );
        let dependencies = [Dependency::Observation(observation)];
        assert!(writfold_kernel::citation::binding_in_footprint(
            &dependencies,
            descriptor.bind(b"line\n")?,
        ));
        let native_as_content = [Dependency::Observation(Observation::new(
            "observation".parse()?,
            reference.repository().clone(),
            reference.revision().clone(),
            descriptor.source().blob().as_str().parse()?,
            0.into(),
            ObsOutcome::Ok,
        ))];
        assert!(!writfold_kernel::citation::binding_in_footprint(
            &native_as_content,
            descriptor.bind(b"line\n")?,
        ));
        // An unchanged excerpt must not collapse different complete captures.
        value["representation"]["capture_digest"] =
            selection_digest(b"line\nsuffix")?.as_str().into();
        let changed = decode_descriptor(&serde_json::to_vec(&value)?, limits()?)?;
        assert_eq!(descriptor.selection_digest(), changed.selection_digest());
        assert_ne!(
            representation_fingerprint(descriptor.source(), limits()?)?,
            representation_fingerprint(changed.source(), limits()?)?
        );
        assert_ne!(
            binding_fingerprint(&descriptor, limits()?)?,
            binding_fingerprint(&changed, limits()?)?
        );
        assert!(!writfold_kernel::citation::binding_in_footprint(
            &dependencies,
            changed.bind(b"line\n")?,
        ));
    }
    Ok(())
}
