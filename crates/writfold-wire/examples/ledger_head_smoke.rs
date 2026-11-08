//! Runs the supplied-byte ledger-head contract, not ledger state or authority.
#[path = "../tests/support/ledger_heads.rs"]
mod fixtures;
use fixtures::Result;
use sha2::{Digest, Sha256};
use writfold_wire::Limits;
use writfold_wire::engineering::ledger::{decode_head, encode_head};

fn main() -> Result {
    let limits = Limits::new(65536, 8, 32, 128)?;
    let (positive, negative) = fixtures::fixtures()?;
    for vector in &positive {
        assert_eq!(
            Sha256::digest(&vector.bytes).as_slice(),
            vector.checksum,
            "{}",
            vector.name
        );
        assert_eq!(
            decode_head(&vector.bytes, limits)?,
            vector.head,
            "{}",
            vector.name
        );
        assert_eq!(
            encode_head(&vector.head, limits)?.as_bytes(),
            vector.bytes,
            "{}",
            vector.name
        );
    }
    for vector in &negative {
        assert!(
            decode_head(&vector.bytes, limits).is_err(),
            "{}",
            vector.name
        );
    }
    println!(
        "ledger_head_smoke=ok goldens={} refusals={} scope=structural-only authority=none",
        positive.len(),
        negative.len()
    );
    Ok(())
}
