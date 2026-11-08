#[path = "support/ledger_heads.rs"]
mod fixtures;
use fixtures::Result;
use sha2::{Digest, Sha256};
use writfold_wire::Limits;
use writfold_wire::engineering::ledger::{decode_head, encode_head};

fn limits() -> Result<Limits> {
    Ok(Limits::new(65536, 8, 32, 128)?)
}

#[test]
fn independently_encoded_heads_preserve_kind_identity_and_counter_domains() -> Result {
    let (positive, _) = fixtures::fixtures()?;
    for vector in positive {
        assert_eq!(
            Sha256::digest(&vector.bytes).as_slice(),
            vector.checksum,
            "{}",
            vector.name
        );
        assert_eq!(
            decode_head(&vector.bytes, limits()?)?,
            vector.head,
            "{}",
            vector.name
        );
        assert_eq!(
            encode_head(&vector.head, limits()?)?.as_bytes(),
            vector.bytes,
            "{}",
            vector.name
        );
    }
    Ok(())
}

#[test]
fn schema_confusion_counter_boundaries_and_incomplete_input_refuse() -> Result {
    let (_, negative) = fixtures::fixtures()?;
    for vector in negative {
        assert!(
            decode_head(&vector.bytes, limits()?).is_err(),
            "{}",
            vector.name
        );
    }
    Ok(())
}

#[test]
fn head_decoding_and_encoding_respect_the_callers_byte_budget() -> Result {
    let (positive, _) = fixtures::fixtures()?;
    for vector in positive {
        let short = Limits::new(vector.bytes.len() - 1, 8, 32, 128)?;
        assert!(decode_head(&vector.bytes, short).is_err());
        assert!(encode_head(&vector.head, short).is_err());
        let exact = Limits::new(vector.bytes.len(), 8, 32, 128)?;
        assert_eq!(decode_head(&vector.bytes, exact)?, vector.head);
        assert_eq!(encode_head(&vector.head, exact)?.as_bytes(), vector.bytes);
    }
    Ok(())
}
