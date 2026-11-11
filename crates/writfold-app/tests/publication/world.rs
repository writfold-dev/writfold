//! One disposable publication world: a Dolt writer, the Git release lease and
//! the approval signer, over a fresh realm (`Fresh`) or over a realm whose
//! first release a real bootstrap publication created (`World`).
use super::realm::{Realm, admitted_dolt};
use super::signer::Signer;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use writfold_app::publication::{Candidate, Publication, Releases, publish};
use writfold_domain::dolt::DoltReadPin;
use writfold_domain::{ByteCount, DoltToolGeneration, GitObjectId, RowLimit};
use writfold_runtime::dolt::{DoltDatabase, DoltExecutable};
use writfold_runtime::fs::{ContainedRoot, PinnedFile, RelativePath};
use writfold_runtime::git::{GitBudgets, GitExecutable, GitWriter};
use writfold_runtime::process::ProcessSupervisor;
use writfold_store::backend::dolt::{DoltBackend, WriterId};
use writfold_store::backend::{ExactRead, RecordChange, RecordKey, RecordKind, StorageBackend};
use writfold_store::dolt::ReadBudgets;
use writfold_wire::Limits;

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
pub static STOP: AtomicBool = AtomicBool::new(false);
/// The maintained qualification profile digest of `/usr/bin/git`.
const GIT_DIGEST: &str = "eb54b535467e0975c1284d4c4d08eea779aeb248a5d34b02e2ee171382e22201";

/// A realm with no release: the Git completed ref is absent.
pub struct Fresh<'w, 'b, 'r, 'f> {
    pub backend: &'w mut DoltBackend<'b>,
    pub releases: &'w Releases<'r, 'f>,
    pub signer: &'w Signer,
    pub realm: &'w Realm,
}

/// A realm whose first release `genesis` (naming `base`) was bootstrapped.
pub struct World<'w, 'b, 'r, 'f> {
    pub backend: &'w mut DoltBackend<'b>,
    pub releases: &'w Releases<'r, 'f>,
    pub signer: &'w Signer,
    pub realm: &'w Realm,
    pub genesis: GitObjectId,
    pub base: DoltReadPin,
}

fn count(value: u64) -> Result<ByteCount> {
    Ok(ByteCount::new(value)?)
}

fn pinned(path: &Path) -> Result<PinnedFile> {
    let parent = path.parent().ok_or("executable has no parent")?;
    let name = path.file_name().ok_or("executable has no name")?;
    Ok(ContainedRoot::open(parent)?.pin(&RelativePath::new(name)?)?)
}

fn budgets() -> Result<ReadBudgets> {
    let stdout = 4 * 1024 * 1024;
    Ok(ReadBudgets::new(
        count(stdout as u64)?,
        count(65_536)?,
        RowLimit::new(64)?,
        Limits::new(stdout, 4, 65, 1024)?,
        Duration::from_secs(60),
    ))
}

fn git_budgets() -> Result<GitBudgets> {
    Ok(GitBudgets::new(
        count(1 << 20)?,
        count(1 << 16)?,
        count(1 << 20)?,
        Duration::from_secs(20),
    ))
}

/// A new realm provisioned through the admitted Dolt at the `AFTER` state.
pub fn provision() -> Result<Realm> {
    let (dolt_path, digest) = admitted_dolt()?;
    Realm::provision(&dolt_path, &digest)
}

/// Open `realm` with a Dolt writer and a new, empty Git release repository.
pub fn with_realm(
    realm: &Realm,
    test: impl FnOnce(&mut Fresh<'_, '_, '_, '_>) -> Result,
) -> Result {
    let (dolt_path, digest) = admitted_dolt()?;
    let supervisor = ProcessSupervisor::enable()?;
    let dolt_file = pinned(&dolt_path)?;
    let generation: DoltToolGeneration = digest.parse()?;
    let dolt = DoltExecutable::admit(&dolt_file, &generation, count(512 << 20)?, &supervisor)?;
    let database = DoltDatabase::open(
        ContainedRoot::open(realm.path("database"))?,
        ContainedRoot::open(realm.path("reader-home"))?,
        count(1 << 20)?,
    )?;
    let slot = database.writer_slot()?;
    let owner = WriterId::new("publisher".to_owned())?;
    let mut backend = DoltBackend::writer(&slot, &dolt, budgets()?, owner, realm.target())?;
    let git_file = pinned(Path::new("/usr/bin/git"))?;
    let git = GitExecutable::admit(
        &git_file,
        &GIT_DIGEST.parse()?,
        count(16 << 20)?,
        &supervisor,
    )?;
    let writer = GitWriter::initialize(
        realm.path("releases"),
        &git,
        count(1 << 20)?,
        git_budgets()?,
        &STOP,
    )?;
    let releases = Releases::lease(&writer, &git, git_budgets()?)?;
    let signer = Signer::load(realm.target())?;
    test(&mut Fresh {
        backend: &mut backend,
        releases: &releases,
        signer: &signer,
        realm,
    })
}

/// A new realm whose first release a signed bootstrap publication created.
pub fn with_world(test: impl FnOnce(&mut World<'_, '_, '_, '_>) -> Result) -> Result {
    let realm = provision()?;
    with_realm(&realm, |fresh| {
        let (genesis, base) = bootstrap(fresh)?;
        test(&mut World {
            backend: &mut *fresh.backend,
            releases: fresh.releases,
            signer: fresh.signer,
            realm: fresh.realm,
            genesis,
            base,
        })
    })
}

/// The realm's first release, through bootstrap publication.
pub fn bootstrap(fresh: &mut Fresh<'_, '_, '_, '_>) -> Result<(GitObjectId, DoltReadPin)> {
    let approval = fresh.signer.bootstrap(0)?;
    let first = candidate("doc:genesis", "genesis")?;
    match publish(fresh.backend, fresh.releases, &approval, &first, &STOP)? {
        Publication::Published { release, pin } => Ok((release, pin)),
        other => Err(format!("bootstrap did not publish: {other:?}").into()),
    }
}

pub fn candidate(key: &str, content: &str) -> Result<Candidate> {
    Ok(Candidate::new(vec![RecordChange {
        key: RecordKey::new(key.to_owned())?,
        kind: RecordKind::new("document".to_owned())?,
        content: Some(content.as_bytes().to_vec()),
    }])?)
}

pub fn reads(backend: &DoltBackend<'_>, pin: &DoltReadPin, key: &str) -> Result<Vec<u8>> {
    let read = backend.read_exact(
        pin,
        &ExactRead::new(RecordKey::new(key.to_owned())?, None),
        &STOP,
    )?;
    Ok(read.content().to_vec())
}
