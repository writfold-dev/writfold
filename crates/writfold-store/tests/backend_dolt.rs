//! B0 end to end: the Dolt adapter passes the storage conformance suite against
//! a real disposable Dolt database, read through the existing DINT01 reader.
//!
//! The admitted executable (path and SHA-256 from
//! `qualification/dolt-integration-profile.json`) must be present, as it is in
//! the qualified tool image used by `scripts/gate unit`. A different or missing
//! Dolt fails this test; it is never skipped.
mod dolt_realm;

use dolt_realm::{Realm, admitted_candidate};
use std::path::Path;
use std::time::Duration;
use writfold_domain::{ByteCount, DoltToolGeneration, RowLimit};
use writfold_runtime::dolt::{DoltDatabase, DoltExecutable};
use writfold_runtime::fs::{ContainedRoot, RelativePath};
use writfold_runtime::process::ProcessSupervisor;
use writfold_store::backend::conformance::{self, CHECKS};
use writfold_store::backend::dolt::DoltBackend;
use writfold_store::backend::{BackendKind, Capability, StorageBackend};
use writfold_store::dolt::ReadBudgets;
use writfold_wire::Limits;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn budgets() -> Result<ReadBudgets> {
    let stdout = 4 * 1024 * 1024;
    Ok(ReadBudgets::new(
        ByteCount::new(stdout as u64)?,
        ByteCount::new(65_536)?,
        RowLimit::new(64)?,
        Limits::new(stdout, 4, 65, 1024)?,
        Duration::from_secs(60),
    ))
}

#[test]
fn dolt_backend_passes_the_storage_conformance_suite() -> Result {
    let (executable_path, digest) = admitted_candidate()?;
    let realm = Realm::provision(&executable_path, &digest)?;
    let generation: DoltToolGeneration = digest.parse()?;
    let supervisor = ProcessSupervisor::enable()?;
    let parent = executable_path.parent().ok_or("executable has no parent")?;
    let name = executable_path
        .file_name()
        .ok_or("executable has no name")?;
    let file = ContainedRoot::open(parent)?.pin(&RelativePath::new(name)?)?;
    let executable = DoltExecutable::admit(
        &file,
        &generation,
        ByteCount::new(512 * 1024 * 1024)?,
        &supervisor,
    )?;
    let database = DoltDatabase::open(
        ContainedRoot::open(realm.path("database"))?,
        ContainedRoot::open(realm.path("reader-home"))?,
        ByteCount::new(1024 * 1024)?,
    )?;
    let mut backend = DoltBackend::new(&database, &executable, budgets()?);
    assert_eq!(backend.kind(), BackendKind::Dolt);
    assert!(!backend.capabilities().contains(&Capability::Write));
    assert!(!backend.capabilities().contains(&Capability::Fuzzy));
    let passed = conformance::run(&mut backend, realm.fixture())?;
    assert_eq!(passed, CHECKS);
    assert_eq!(realm.head()?, realm.head_at_provision());
    assert!(Path::new(&realm.path("database/coordination/database.lock")).is_file());
    Ok(())
}
