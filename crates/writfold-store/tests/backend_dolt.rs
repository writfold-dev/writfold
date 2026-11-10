//! B0 end to end: the Dolt adapter passes the storage conformance suite against
//! a real disposable Dolt database. Reads go through the existing DINT01 reader;
//! writes go through the native conditional writer with real signed approvals.
//!
//! The admitted executable (path and SHA-256 from
//! `qualification/dolt-integration-profile.json`) must be present, as it is in
//! the qualified tool image used by `scripts/gate unit`. A different or missing
//! Dolt fails these tests; they are never skipped.
mod dolt_realm;

use dolt_realm::approval::Signer;
use dolt_realm::{Realm, admitted_candidate};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use writfold_domain::dolt::DoltReadPin;
use writfold_domain::{ByteCount, DoltToolGeneration, RowLimit};
use writfold_runtime::dolt::{DoltDatabase, DoltExecutable};
use writfold_runtime::fs::{ContainedRoot, RelativePath};
use writfold_runtime::process::ProcessSupervisor;
use writfold_store::backend::conformance::{self, CHECKS, WRITE_CHECKS, WriterFixture};
use writfold_store::backend::dolt::{DoltBackend, WriterId};
use writfold_store::backend::{
    BackendKind, Capability, HeadName, RecordChange, RecordKey, RecordKind, Refusal,
    StorageBackend, WriteRequest,
};
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

/// Provision a disposable realm and admit the exact executable and database.
fn with_native(test: impl FnOnce(&Realm, &DoltDatabase, &DoltExecutable<'_>) -> Result) -> Result {
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
    test(&realm, &database, &executable)
}

#[test]
fn dolt_backend_passes_the_storage_conformance_suite() -> Result {
    with_native(|realm, database, executable| {
        let mut backend = DoltBackend::new(database, executable, budgets()?);
        assert_eq!(backend.kind(), BackendKind::Dolt);
        assert!(!backend.capabilities().contains(&Capability::Write));
        assert!(!backend.capabilities().contains(&Capability::Fuzzy));
        let passed = conformance::run(&mut backend, realm.fixture())?;
        assert_eq!(passed, CHECKS);
        assert_eq!(realm.head()?, realm.head_at_provision());
        assert!(Path::new(&realm.path("database/coordination/database.lock")).is_file());
        Ok(())
    })
}

#[test]
fn dolt_writer_passes_the_writer_conformance_suite() -> Result {
    with_native(|realm, database, executable| {
        let budgets = budgets()?;
        let slot = database.writer_slot()?;
        let target = realm.target();
        let signer = Signer::load(target.clone())?;
        let open = || {
            let owner = WriterId::new("conformance-writer".to_owned())?;
            DoltBackend::writer(&slot, executable, budgets, owner, target.clone())
        };
        let authorize =
            |request: &WriteRequest<DoltReadPin>| signer.authority(&request.expected_old);
        let writes = WriterFixture {
            open: &open,
            authorize: &authorize,
            foreign: realm.foreign()?,
        };
        let report = conformance::run_writes(realm.fixture(), &writes)?;
        eprintln!("dolt writer conformance: {report:?}");
        assert_eq!(report.passed, WRITE_CHECKS);
        assert!(report.kill.unknown_applied + report.kill.unknown_absent > 0);
        // Native evidence: no uncommitted residue, the head is retained, and
        // the writer lease's holder is recorded.
        assert!(realm.clean()?);
        let head = realm.head()?;
        assert!(realm.branches()?.contains(&format!("wf-retained/{head}")));
        let claim = std::fs::read_to_string(realm.path("database/coordination/writer.claim"))?;
        assert!(claim.contains("owner=conformance-writer\nrealm=test-realm\n"));
        Ok(())
    })
}

#[test]
fn dolt_writer_refuses_bootstrap_approvals_and_a_mismatched_producer() -> Result {
    with_native(|realm, database, executable| {
        let signer = Signer::load(realm.target())?;
        assert!(matches!(
            signer.bootstrap_admission()?,
            Err(Refusal::Unauthorized(_))
        ));
        let slot = database.writer_slot()?;
        let other = realm.target();
        let wrong = writfold_wire::dolt_operation::DoltTarget::new(
            other.realm().clone(),
            other.kb().clone(),
            other.database().clone(),
            other.epoch().clone(),
            other.profile().clone(),
            "e".repeat(64).parse()?,
        );
        let owner = WriterId::new("w".to_owned())?;
        let refused = DoltBackend::writer(&slot, executable, budgets()?, owner, wrong);
        assert!(matches!(refused, Err(Refusal::Pin(_))));
        Ok(())
    })
}

/// A writer configured for another database identity is signed and owns its
/// pins, but the in-transaction realm-row guard refuses the commit atomically.
#[test]
fn dolt_writer_refuses_a_target_that_is_not_the_databases_realm_row() -> Result {
    with_native(|realm, database, executable| {
        let actual = realm.target();
        let target = writfold_wire::dolt_operation::DoltTarget::new(
            actual.realm().clone(),
            actual.kb().clone(),
            "other_database".parse()?,
            actual.epoch().clone(),
            actual.profile().clone(),
            actual.producer().clone(),
        );
        let after = &realm.fixture().after;
        let expected_old = DoltReadPin::new(
            after.realm().clone(),
            after.kb().clone(),
            target.database().clone(),
            after.commit().clone(),
            after.epoch().clone(),
            after.profile().clone(),
            after.producer().clone(),
        );
        let approval = Signer::load(target.clone())?.approve(&expected_old)?;
        let slot = database.writer_slot()?;
        let owner = WriterId::new("w".to_owned())?;
        let mut writer = DoltBackend::writer(&slot, executable, budgets()?, owner, target)?;
        let request = WriteRequest {
            head: HeadName::new("main".to_owned())?,
            expected_old,
            changes: vec![RecordChange {
                key: RecordKey::new("doc:x".to_owned())?,
                kind: RecordKind::new("document".to_owned())?,
                content: Some(b"wrong realm".to_vec()),
            }],
        };
        let refused = writer.admit_write(&request, Some(&approval), &AtomicBool::new(false));
        assert!(matches!(refused, Err(Refusal::Backend(_))), "{refused:?}");
        assert_eq!(realm.head()?, realm.head_at_provision());
        assert!(realm.clean()?);
        Ok(())
    })
}
