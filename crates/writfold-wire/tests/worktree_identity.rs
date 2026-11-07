use serde_json::Value;
use writfold_domain::{
    engineering::{DispatchStampDigest, EffectIntentDigest},
    worktree::*,
};
use writfold_wire::{Limits, worktree::*};

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn limits() -> TestResult<Limits> {
    Ok(Limits::new(1_000_000, 32, 10_000, 20_000)?)
}
fn fixture() -> TestResult<Value> {
    Ok(serde_json::from_str(include_str!(
        "fixtures/worktree/golden.json"
    ))?)
}
fn vectors(fixture: &Value) -> TestResult<&[Value]> {
    Ok(fixture["vectors"]
        .as_array()
        .ok_or("fixture vectors missing")?
        .as_slice())
}
fn text(value: &Value) -> TestResult<&str> {
    value.as_str().ok_or_else(|| "fixture text missing".into())
}
fn named(fixture: &Value, name: &str, limits: Limits) -> TestResult<Record> {
    let vector = vectors(fixture)?
        .iter()
        .find(|v| v["name"] == name)
        .ok_or("named fixture missing")?;
    Ok(Record::decode(
        &serde_json::to_vec(&vector["envelope"])?,
        limits,
    )?)
}
fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 15)]));
    }
    result
}
fn inputs(
    fixture: &Value,
    limits: Limits,
) -> TestResult<(Recipe, Capture, Candidate, PreparedEdit)> {
    let Record::Recipe(recipe) = named(fixture, "recipe-unicode-crlf", limits)? else {
        return Err("expected recipe fixture".into());
    };
    let Record::Capture(capture) = named(fixture, "capture", limits)? else {
        return Err("expected capture fixture".into());
    };
    let Record::Candidate(candidate) = named(fixture, "candidate", limits)? else {
        return Err("expected candidate fixture".into());
    };
    let Record::PreparedEdit(prepared) = named(fixture, "prepared", limits)? else {
        return Err("expected prepared-edit fixture".into());
    };
    Ok((recipe, capture, candidate, prepared))
}

#[test]
fn independent_golden_bytes_and_identities() -> TestResult {
    let fixture = fixture()?;
    let limits = limits()?;
    for vector in vectors(&fixture)? {
        let record = Record::decode(&serde_json::to_vec(&vector["envelope"])?, limits)?;
        assert_eq!(
            hex(&record.canonical_bytes(limits)?),
            text(&vector["cbor_hex"])?
        );
        assert_eq!(record.identity(limits)?, text(&vector["envelope"]["id"])?);
        let json = record.encode_json(limits)?;
        assert_eq!(Record::decode(&json, limits)?, record);
        let prefix = format!("writfold-worktree/1/{}\0", record.kind());
        assert_eq!(
            format!("{}{}", hex(prefix.as_bytes()), text(&vector["cbor_hex"])?),
            text(&vector["preimage_hex"])?
        );
    }
    Ok(())
}

#[test]
fn adverse_schema_and_cross_reference_vectors() -> TestResult {
    let adverse: Value = serde_json::from_str(include_str!("fixtures/worktree/adverse.json"))?;
    let fixture = fixture()?;
    let limits = limits()?;
    let (recipe, capture, candidate, _) = inputs(&fixture, limits)?;
    for vector in vectors(&adverse)? {
        let decoded = Record::decode(text(&vector["json"])?.as_bytes(), limits);
        if vector["stage"] == "decode" {
            assert!(decoded.is_err(), "{}", vector["name"]);
        } else {
            let Record::PreparedEdit(prepared) = decoded? else {
                return Err("expected adverse prepared edit".into());
            };
            let scope = if vector["stage"] == "expected-scope" {
                Scope::new(
                    "realm-b".parse()?,
                    "run-a".parse()?,
                    "repo-a".parse()?,
                    "worktree-a".parse()?,
                    "binding-a".parse()?,
                )?
            } else {
                capture.scope().clone()
            };
            assert!(
                validate_prepared(&prepared, &recipe, &capture, &candidate, &scope, limits)
                    .is_err(),
                "{}",
                vector["name"]
            );
        }
    }
    Ok(())
}

#[test]
fn exact_recipe_changes_prepared_not_candidate() -> TestResult {
    let fixture = fixture()?;
    let limits = limits()?;
    let (r, c, n, p) = inputs(&fixture, limits)?;
    validate_prepared(&p, &r, &c, &n, c.scope(), limits)?;
    let Record::Recipe(r2) = named(&fixture, "changed-recipe-same-output", limits)? else {
        return Err("expected changed recipe".into());
    };
    let Record::PreparedEdit(p2) = named(&fixture, "prepared-changed-recipe-same-output", limits)?
    else {
        return Err("expected changed preparation".into());
    };
    validate_prepared(&p2, &r2, &c, &n, c.scope(), limits)?;
    assert_ne!(recipe_digest(&r, limits)?, recipe_digest(&r2, limits)?);
    assert_ne!(
        prepared_edit_digest(&p, limits)?,
        prepared_edit_digest(&p2, limits)?
    );
    assert_eq!(p.candidate(), p2.candidate());
    assert_eq!(r.source(), "replace α with β\r\n".as_bytes());
    assert_eq!(c.content(), "α\r\n".as_bytes());
    Ok(())
}

#[test]
fn stable_key_mapping_is_separate_from_dispatch() -> TestResult {
    let relations: Value = serde_json::from_str(include_str!("fixtures/worktree/relations.json"))?;
    let fixture = fixture()?;
    let limits = limits()?;
    let Record::RequestKey(key) = named(&fixture, "request-key", limits)? else {
        return Err("expected request-key fixture".into());
    };
    let digest = request_key_digest(&key, limits)?;
    let same = &relations[0];
    let intent: EffectIntentDigest = text(&same["intent"])?.parse()?;
    let first: DispatchStampDigest = text(&same["first_stamp"])?.parse()?;
    let second: DispatchStampDigest = text(&same["second_stamp"])?.parse()?;
    assert_ne!(first, second);
    assert_eq!(digest.to_string(), text(&same["key"])?);
    let mapping = RequestMapping::new(digest, intent, "host-effect-a".parse()?);
    assert!(mapping.agrees(&RequestMapping::new(
        digest,
        intent,
        "host-effect-a".parse()?
    )));
    let other: EffectIntentDigest = text(&relations[1]["second_intent"])?.parse()?;
    assert!(!mapping.agrees(&RequestMapping::new(
        digest,
        other,
        "host-effect-a".parse()?
    )));
    assert!(!mapping.agrees(&RequestMapping::new(
        digest,
        intent,
        "host-effect-b".parse()?
    )));
    let other_realm = RequestKey::new(
        key.principal().clone(),
        "realm-b".parse()?,
        key.run().clone(),
        key.opaque_key().to_vec(),
    )?;
    assert_ne!(digest, request_key_digest(&other_realm, limits)?);
    Ok(())
}

#[test]
fn observations_are_checked_data_not_authority() -> TestResult {
    let fixture = fixture()?;
    let limits = limits()?;
    let (_, c, n, p) = inputs(&fixture, limits)?;
    for name in ["confirmed", "no-effect", "unknown", "partial"] {
        let Record::Observation(o) = named(&fixture, &format!("observation-{name}"), limits)?
        else {
            return Err("expected observation fixture".into());
        };
        validate_observation(&o, o.intent(), o.stamp(), &p, &c, &n, limits)?;
        let wrong: DispatchStampDigest =
            format!("wf1:dispatch-stamp:sha256:{}", "00".repeat(32)).parse()?;
        assert!(validate_observation(&o, o.intent(), wrong, &p, &c, &n, limits).is_err());
    }
    Ok(())
}

#[test]
fn strict_limits_are_applied() -> TestResult {
    let fixture = fixture()?;
    let record = named(&fixture, "recipe-unicode-crlf", limits()?)?;
    let bytes = record.encode_json(limits()?)?;
    for bound in [
        Limits::new(8, 32, 100, 100)?,
        Limits::new(10000, 1, 100, 100)?,
        Limits::new(10000, 32, 1, 100)?,
        Limits::new(10000, 32, 100, 1)?,
    ] {
        assert!(Record::decode(&bytes, bound).is_err());
        assert!(record.encode_json(bound).is_err());
    }
    Ok(())
}

#[test]
fn exact_timestamps_bind_same_byte_metadata_changes() -> TestResult {
    let fixture = fixture()?;
    let limits = limits()?;
    let (_, capture, _, _) = inputs(&fixture, limits)?;
    assert_eq!(capture.metadata().mtime().seconds(), -1);
    assert_eq!(capture.metadata().mtime().nanoseconds(), 999_999_999);
    for field in ["mtime", "ctime"] {
        let Record::Capture(changed) = named(
            &fixture,
            &format!("capture-same-bytes-changed-{field}"),
            limits,
        )?
        else {
            return Err("expected changed-metadata capture".into());
        };
        assert_eq!(capture.content(), changed.content());
        assert_ne!(
            capture_digest(&capture, limits)?,
            capture_digest(&changed, limits)?
        );
    }
    Ok(())
}

#[test]
fn pinned_adjacent_edits_and_exact_origins_are_preserved() -> TestResult {
    let fixture = fixture()?;
    let limits = limits()?;
    let (recipe, capture, candidate, prepared) = inputs(&fixture, limits)?;
    let Record::PreparedEdit(adjacent) =
        named(&fixture, "prepared-adjacent-positive-width", limits)?
    else {
        return Err("expected adjacent prepared edit".into());
    };
    validate_prepared(
        &adjacent,
        &recipe,
        &capture,
        &candidate,
        capture.scope(),
        limits,
    )?;
    assert_eq!(adjacent.splices().len(), 2);
    assert_eq!(adjacent.splices()[0].end(), adjacent.splices()[1].start());
    let Record::PreparedEdit(origin) = named(&fixture, "prepared-changed-origin", limits)? else {
        return Err("expected changed-origin prepared edit".into());
    };
    validate_prepared(
        &origin,
        &recipe,
        &capture,
        &candidate,
        capture.scope(),
        limits,
    )?;
    assert_eq!(prepared.candidate(), origin.candidate());
    assert_ne!(
        prepared_edit_digest(&prepared, limits)?,
        prepared_edit_digest(&origin, limits)?
    );
    Ok(())
}
