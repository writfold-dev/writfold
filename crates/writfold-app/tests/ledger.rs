//! Durable ledger: records retained as Git commits, heads selected by
//! expected-old CAS, every load replayed through the kernel validator.
#[path = "ledger/support.rs"]
mod support;
use std::error::Error;
use support::*;
use writfold_app::ledger::{Ledger, LedgerError};
use writfold_domain::StreamId;
use writfold_domain::engineering::{BlobDigest, EngineeringDigest};
use writfold_kernel::ledger::{Record, Refusal};
use writfold_wire::engineering::ledger_record as codec;

#[test]
fn appends_reload_and_refuse_through_the_kernel() -> Result {
    let f = Fixture::new()?;
    let executable = executable(&f.git)?;
    let lease = f.writer.try_lease()?;
    let material = *f.objects.put(b"evidence", count(64)?)?.digest();
    let ledger = Ledger {
        lease: &lease,
        executable: &executable,
        objects: &f.objects,
        validator: &Capacity,
        limits: limits()?,
        budgets: budgets()?,
        max_records: 16,
        cancelled: &ACTIVE,
    };
    let s = stream()?;
    let h1 = ledger.append(
        &s,
        None,
        record("r1", 1, None, Units::Reserve(2), vec![material])?,
    )?;
    let h2 = ledger.append(
        &s,
        Some(&h1),
        record("r2", 2, Some("r1"), Units::Release(1), vec![])?,
    )?;
    let loaded = ledger.load::<Units, NoPremises>(&s)?;
    assert_eq!(
        loaded
            .records
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["r1", "r2"]
    );
    let refused = |expected, r| match ledger.append(&s, expected, r) {
        Err(LedgerError::Refused(refusal)) => Ok(refusal),
        other => Err(format!("expected refusal, got {other:?}")),
    };
    // Stale expected head, overbooking, and unretained material all refuse.
    assert_eq!(
        refused(
            Some(&h1),
            record("r3", 3, Some("r2"), Units::Reserve(1), vec![])?
        )?,
        Refusal::StaleHead
    );
    assert_eq!(
        refused(
            Some(&h2),
            record("r3", 3, Some("r2"), Units::Reserve(3), vec![])?
        )?,
        Refusal::InvalidRecord
    );
    let missing = BlobDigest::from_digest([9; 32]);
    assert_eq!(
        refused(
            Some(&h2),
            record("r3", 3, Some("r2"), Units::Reserve(1), vec![missing])?
        )?,
        Refusal::MissingMaterial
    );
    assert_eq!(
        refused(
            Some(&h2),
            record("r1", 3, Some("r2"), Units::Reserve(1), vec![])?
        )?,
        Refusal::DuplicateRecord
    );
    // Refusals selected nothing.
    assert_eq!(ledger.load::<Units, NoPremises>(&s)?.records.len(), 2);
    ledger.append(
        &s,
        Some(&h2),
        record("r3", 3, Some("r2"), Units::Reserve(2), vec![])?,
    )?;
    Ok(())
}

#[test]
fn histories_written_around_the_kernel_refuse_on_load() -> Result {
    let f = Fixture::new()?;
    let executable = executable(&f.git)?;
    let lease = f.writer.try_lease()?;
    let ledger = Ledger {
        lease: &lease,
        executable: &executable,
        objects: &f.objects,
        validator: &Capacity,
        limits: limits()?,
        budgets: budgets()?,
        max_records: 2,
        cancelled: &ACTIVE,
    };
    let s = stream()?;
    let raw = |expected, r: R| {
        lease
            .ledger_select(
                &executable,
                &s,
                expected,
                &codec::encode(&r, limits()?)?,
                budgets()?,
                &ACTIVE,
            )
            .map_err(Box::<dyn Error>::from)
    };
    // An overbooked second record, selected directly in Git.
    let c1 = raw(None, record("r1", 1, None, Units::Reserve(3), vec![])?)?;
    let c2 = raw(
        Some(&c1),
        record("r2", 2, Some("r1"), Units::Reserve(1), vec![])?,
    )?;
    assert!(matches!(
        ledger.load::<Units, NoPremises>(&s),
        Err(LedgerError::InvalidHistory)
    ));
    // A sequence gap.
    let other: StreamId = "run.2".parse()?;
    let gap = Record {
        stream: other.clone(),
        ..record("g1", 2, None, Units::Reserve(1), vec![])?
    };
    lease.ledger_select(
        &executable,
        &other,
        None,
        &codec::encode(&gap, limits()?)?,
        budgets()?,
        &ACTIVE,
    )?;
    assert!(matches!(
        ledger.load::<Units, NoPremises>(&other),
        Err(LedgerError::InvalidHistory)
    ));
    // A history longer than the load bound.
    raw(
        Some(&c2),
        record("r3", 3, Some("r2"), Units::Release(1), vec![])?,
    )?;
    assert!(matches!(
        ledger.load::<Units, NoPremises>(&s),
        Err(LedgerError::TooLong(2))
    ));
    Ok(())
}

#[test]
fn record_codec_is_canonical() -> Result {
    let r = record(
        "r1",
        1,
        None,
        Units::Reserve(2),
        vec![BlobDigest::from_digest([1; 32])],
    )?;
    let bytes = codec::encode(&r, limits()?)?;
    assert_eq!(codec::decode::<Units, NoPremises>(&bytes, limits()?)?, r);
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(codec::decode::<Units, NoPremises>(&trailing, limits()?).is_err());
    assert!(codec::decode::<Units, NoPremises>(&bytes[..bytes.len() - 1], limits()?).is_err());
    // A different namespace with the same structure refuses.
    let mut renamed = bytes.clone();
    let at = renamed
        .windows(22)
        .position(|w| w == b"writfold-ledger-record")
        .ok_or("namespace")?;
    renamed[at + 21] = b'x';
    assert!(codec::decode::<Units, NoPremises>(&renamed, limits()?).is_err());
    Ok(())
}

#[test]
fn unretained_material_refuses_on_load() -> Result {
    let f = Fixture::new()?;
    let executable = executable(&f.git)?;
    let lease = f.writer.try_lease()?;
    let ledger = Ledger {
        lease: &lease,
        executable: &executable,
        objects: &f.objects,
        validator: &Capacity,
        limits: limits()?,
        budgets: budgets()?,
        max_records: 2,
        cancelled: &ACTIVE,
    };
    // Material that is not retained.
    let third: StreamId = "run.3".parse()?;
    let unretained = Record {
        stream: third.clone(),
        ..record(
            "u1",
            1,
            None,
            Units::Reserve(1),
            vec![BlobDigest::from_digest([7; 32])],
        )?
    };
    lease.ledger_select(
        &executable,
        &third,
        None,
        &codec::encode(&unretained, limits()?)?,
        budgets()?,
        &ACTIVE,
    )?;
    assert!(matches!(
        ledger.load::<Units, NoPremises>(&third),
        Err(LedgerError::UnretainedMaterial)
    ));
    Ok(())
}
