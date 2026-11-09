//! The private state directory of `writfold-trial-worktree/1`.
//!
//! Layout (owner-only): `operator.pem` (Ed25519 operator key), `policy.json`
//! (canonical bootstrap policy trusting that key), `git.sha256` and `git.path`
//! (the Git executable bytes observed at init, required unchanged afterwards),
//! `ledger/` (private writer repository), `objects/` (retained bytes) and
//! `worktrees/<run>/`. This is a cooperating single-user trial profile: it does
//! not defend the directory against the same user or root.
use super::flow::{Blob, Flow};
use super::{BUDGET, Result, budgets, limits};
use crate::ledger::Ledger;
use ed25519_dalek::SigningKey;
use ed25519_dalek::pkcs8::{EncodePrivateKey, spki::der::pem::LineEnding};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::AtomicBool;
use writfold_domain::engineering::objects::{BlobReference, RealmIdentity};
use writfold_domain::{ByteCount, ContentDigest, GitObjectFormat, RealmId};
use writfold_runtime::fs::{ContainedRoot, PinnedFile, RelativePath};
use writfold_runtime::git::{GitExecutable, GitWriter, GitWriterLease};
use writfold_runtime::process::{ProcessError, ProcessSupervisor};
use writfold_store::objects::ObjectStore;
use writfold_wire::policy::AuthorityPolicy;
use writfold_wire::signatures::ApprovalSigningKey;
use writfold_wire::{RawJson, canonical};

pub const REALM: &str = "trial";
pub static CANCELLED: AtomicBool = AtomicBool::new(false);
static SUPERVISOR: LazyLock<std::result::Result<ProcessSupervisor, ProcessError>> =
    LazyLock::new(ProcessSupervisor::enable);

pub struct Trial {
    pub root: PathBuf,
    pub realm: RealmId,
    pub policy: AuthorityPolicy,
    pub writer: GitWriter,
    pub objects: ObjectStore,
    git: PinnedFile,
    git_digest: ContentDigest,
}

fn private_dir(path: &Path) -> Result<()> {
    fs::DirBuilder::new().mode(0o700).create(path)?;
    Ok(())
}

fn private_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn find_git() -> Result<PathBuf> {
    let path = std::env::var_os("PATH").ok_or("PATH is not set; cannot find git")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("git"))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| "git was not found on PATH".into())
        .and_then(|found| Ok(fs::canonicalize(found)?))
}

fn pin(path: &Path) -> Result<PinnedFile> {
    let dir = path.parent().ok_or("git path has no parent")?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("git path name")?;
    Ok(ContainedRoot::open(dir)?.pin(&RelativePath::new(name)?)?)
}

fn supervisor() -> Result<&'static ProcessSupervisor> {
    SUPERVISOR
        .as_ref()
        .map_err(|e| format!("process supervision unavailable: {e}").into())
}

fn policy_bytes(public_key: &[u8; 32]) -> Result<Vec<u8>> {
    let hex: String = public_key.iter().map(|b| format!("{b:02x}")).collect();
    let json = format!(
        r#"{{"kb_id":"trial","policy_version":"trial-1","schema_version":2,"trusted_keys":{{"operator":{{"public_key":"{hex}","target_uses":["grant-revocation","operator-grant","prepared-edit-approval"]}}}}}}"#
    );
    Ok(canonical(
        &RawJson::parse(json.as_bytes(), limits()?)?,
        BUDGET,
    )?)
}

impl Trial {
    /// Create a new state directory; an existing path refuses.
    pub fn init(root: &Path) -> Result<()> {
        private_dir(root)?;
        let mut seed = [0u8; 32];
        fs::File::open("/dev/urandom")?.read_exact(&mut seed)?;
        let key = SigningKey::from_bytes(&seed);
        let pem = key
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| format!("key encoding: {e}"))?;
        private_file(&root.join("operator.pem"), pem.as_bytes())?;
        private_file(
            &root.join("policy.json"),
            &policy_bytes(key.verifying_key().as_bytes())?,
        )?;
        let git = find_git()?;
        let digest = GitExecutable::observe(&pin(&git)?, count(64 << 20)?)?;
        private_file(&root.join("git.path"), git.as_os_str().as_encoded_bytes())?;
        private_file(&root.join("git.sha256"), digest.as_str().as_bytes())?;
        let file = pin(&git)?;
        let executable = GitExecutable::admit(&file, &digest, count(64 << 20)?, supervisor()?)?;
        GitWriter::initialize(
            root.join("ledger"),
            &executable,
            count(1 << 20)?,
            budgets()?,
            &CANCELLED,
        )?;
        let identity = RealmIdentity::new(REALM.parse()?, "trial".parse()?, "local".parse()?);
        ObjectStore::initialize(root.join("objects"), identity, limits()?)?;
        private_dir(&root.join("worktrees"))?;
        Ok(())
    }

    pub fn open(root: &Path) -> Result<Self> {
        let git = PathBuf::from(std::str::from_utf8(&fs::read(root.join("git.path"))?)?);
        let git_digest: ContentDigest =
            std::str::from_utf8(&fs::read(root.join("git.sha256"))?)?.parse()?;
        let policy =
            AuthorityPolicy::new_bootstrap(&fs::read(root.join("policy.json"))?, limits()?)?;
        let writer = GitWriter::open(
            ContainedRoot::open(root.join("ledger"))?,
            GitObjectFormat::Sha256,
            count(1 << 20)?,
        )?;
        let realm: RealmId = REALM.parse()?;
        let objects = ObjectStore::open(root.join("objects"), &realm, limits()?)?;
        Ok(Self {
            root: root.to_owned(),
            realm,
            policy,
            writer,
            objects,
            git: pin(&git)?,
            git_digest,
        })
    }

    pub fn executable(&self) -> Result<GitExecutable<'_>> {
        Ok(GitExecutable::admit(
            &self.git,
            &self.git_digest,
            count(64 << 20)?,
            supervisor()?,
        )?)
    }

    pub fn signing_key(&self) -> Result<ApprovalSigningKey> {
        Ok(ApprovalSigningKey::from_pkcs8_pem(&fs::read_to_string(
            self.root.join("operator.pem"),
        )?)?)
    }

    pub fn worktree(&self, run: &str) -> PathBuf {
        self.root.join("worktrees").join(run)
    }

    pub fn retain(&self, bytes: &[u8]) -> Result<Blob> {
        let reference = self.objects.put(bytes, count(BUDGET as u64)?)?;
        Ok(Blob {
            digest: *reference.digest(),
            bytes: reference.bytes().get(),
        })
    }

    pub fn fetch(&self, blob: &Blob) -> Result<Vec<u8>> {
        let reference = BlobReference::new(self.realm.clone(), blob.digest, count(blob.bytes)?);
        Ok(self.objects.get(&reference, count(BUDGET as u64)?)?)
    }

    pub fn ledger<'a>(
        &'a self,
        lease: &'a GitWriterLease<'a>,
        executable: &'a GitExecutable<'a>,
    ) -> Result<Ledger<'a, Flow>> {
        Ok(Ledger {
            lease,
            executable,
            objects: &self.objects,
            validator: &Flow,
            limits: limits()?,
            budgets: budgets()?,
            max_records: 16,
            cancelled: &CANCELLED,
        })
    }
}

pub fn count(value: u64) -> Result<ByteCount> {
    Ok(ByteCount::new(value)?)
}
