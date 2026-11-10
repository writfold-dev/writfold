//! One disposable publication world: a Dolt writer, the Git release lease, a
//! fixture-owned genesis release and the approval signer.
use super::realm::{Realm, admitted_dolt};
use super::signer::Signer;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use writfold_app::publication::{Candidate, Manifest, Releases};
use writfold_domain::dolt::DoltReadPin;
use writfold_domain::{ByteCount, DoltToolGeneration, GitObjectId, PlanDigest, RowLimit};
use writfold_runtime::dolt::{DoltDatabase, DoltExecutable};
use writfold_runtime::fs::{ContainedRoot, PinnedFile, RelativePath};
use writfold_runtime::git::{
    GitBudgets, GitExecutable, GitManifestEntry, GitPrepareInput, GitWriter,
};
use writfold_runtime::process::ProcessSupervisor;
use writfold_store::backend::dolt::{DoltBackend, WriterId};
use writfold_store::backend::{ExactRead, RecordChange, RecordKey, RecordKind, StorageBackend};
use writfold_store::dolt::ReadBudgets;
use writfold_wire::Limits;

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
pub static STOP: AtomicBool = AtomicBool::new(false);
/// The maintained qualification profile digest of `/usr/bin/git`.
const GIT_DIGEST: &str = "eb54b535467e0975c1284d4c4d08eea779aeb248a5d34b02e2ee171382e22201";

pub struct World<'w, 'b, 'r, 'f> {
    pub backend: &'w mut DoltBackend<'b>,
    pub releases: &'w Releases<'r, 'f>,
    pub signer: &'w Signer,
    pub realm: &'w Realm,
    pub genesis: GitObjectId,
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

/// One realm: a Dolt writer, the Git release lease and a fixture-owned
/// genesis release naming the provisioned `AFTER` pin.
pub fn with_world(test: impl FnOnce(&mut World<'_, '_, '_, '_>) -> Result) -> Result {
    let (dolt_path, digest) = admitted_dolt()?;
    let realm = Realm::provision(&dolt_path, &digest)?;
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
    let genesis = genesis(&releases, &git, &realm.after)?;
    let signer = Signer::load(realm.target())?;
    test(&mut World {
        backend: &mut backend,
        releases: &releases,
        signer: &signer,
        realm: &realm,
        genesis,
    })
}

/// The fixture owner's first release. Bootstrap publication is not in scope.
fn genesis(
    releases: &Releases<'_, '_>,
    git: &GitExecutable<'_>,
    pin: &DoltReadPin,
) -> Result<GitObjectId> {
    let candidate = Candidate::new(Vec::new())?;
    let native = releases.native();
    let blob = native.write_blob(git, candidate.bytes(), git_budgets()?, &STOP)?;
    let operation: PlanDigest = "e".repeat(64).parse()?;
    let manifest = Manifest {
        operation: operation.clone(),
        candidate: candidate.digest().clone(),
        blob: blob.clone(),
        pin: pin.clone(),
    }
    .encode()?;
    let entries = [GitManifestEntry {
        path: RelativePath::new("candidate")?,
        blob,
    }];
    let input = GitPrepareInput::new(&operation, None, &manifest, &entries);
    let prepared = native.prepare(git, input, git_budgets()?, &STOP)?;
    native.publish(git, &prepared, git_budgets()?, &STOP)?;
    Ok(prepared.commit().clone())
}

pub fn candidate(key: &str, content: &str) -> Result<Candidate> {
    Ok(Candidate::new(vec![RecordChange {
        key: RecordKey::new(key.to_owned())?,
        kind: RecordKind::new("document".to_owned())?,
        content: Some(content.as_bytes().to_vec()),
    }])?)
}

pub fn reads(world: &World<'_, '_, '_, '_>, pin: &DoltReadPin, key: &str) -> Result<Vec<u8>> {
    let read = world.backend.read_exact(
        pin,
        &ExactRead::new(RecordKey::new(key.to_owned())?, None),
        &STOP,
    )?;
    Ok(read.content().to_vec())
}
