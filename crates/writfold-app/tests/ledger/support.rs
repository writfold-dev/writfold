//! Shared fixture: a capacity-3 reservation validator over a real writer
//! repository and object store.
use std::error::Error;
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use writfold_domain::engineering::BlobDigest;
use writfold_domain::engineering::ledger::ReducerProfile;
use writfold_domain::engineering::objects::RealmIdentity;
use writfold_domain::{ByteCount, ContentDigest, LedgerSequence, StreamId};
use writfold_kernel::ledger::{Record, Validator};
use writfold_runtime::fs::{ContainedRoot, PinnedFile, RelativePath};
use writfold_runtime::git::{GitBudgets, GitExecutable, GitWriter};
use writfold_runtime::process::{ProcessError, ProcessSupervisor};
use writfold_store::objects::ObjectStore;
use writfold_wire::Limits;
use writfold_wire::engineering::ledger_record::{self as codec, Part};

pub type Result<T = (), E = Box<dyn Error>> = std::result::Result<T, E>;
pub static ACTIVE: AtomicBool = AtomicBool::new(false);
pub static NEXT: AtomicU64 = AtomicU64::new(0);
pub static SUPERVISOR: LazyLock<std::result::Result<ProcessSupervisor, ProcessError>> =
    LazyLock::new(ProcessSupervisor::enable);
pub const CAPACITY: u64 = 3;

/// Reserve or release units of one capacity-3 resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Units {
    Reserve(u64),
    Release(u64),
}
impl Part for Units {
    fn encode(&self, _: Limits) -> codec::Result<Vec<u8>> {
        let (tag, n) = match self {
            Units::Reserve(n) => (0u8, n),
            Units::Release(n) => (1u8, n),
        };
        Ok([&[tag][..], &n.to_be_bytes()].concat())
    }
    fn decode(bytes: &[u8], _: Limits) -> codec::Result<Self> {
        let (tag, n) = bytes.split_first().ok_or(codec::Error::Field("units"))?;
        let n = u64::from_be_bytes(n.try_into().map_err(|_| codec::Error::Field("units"))?);
        match tag {
            0 => Ok(Units::Reserve(n)),
            1 => Ok(Units::Release(n)),
            _ => Err(codec::Error::Field("units")),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoPremises;
impl Part for NoPremises {
    fn encode(&self, _: Limits) -> codec::Result<Vec<u8>> {
        Ok(Vec::new())
    }
    fn decode(bytes: &[u8], _: Limits) -> codec::Result<Self> {
        bytes
            .is_empty()
            .then_some(NoPremises)
            .ok_or(codec::Error::Field("premises"))
    }
}
pub struct Capacity;
impl Validator<Units, NoPremises> for Capacity {
    fn validate(
        &self,
        prefix: &[Record<Units, NoPremises>],
        record: &Record<Units, NoPremises>,
    ) -> bool {
        let used = prefix.iter().fold(0u64, |t, r| match r.payload {
            Units::Reserve(n) => t.saturating_add(n),
            Units::Release(n) => t.saturating_sub(n),
        });
        match record.payload {
            Units::Reserve(n) => 0 < n && used.saturating_add(n) <= CAPACITY,
            Units::Release(n) => 0 < n && n <= used,
        }
    }
}
pub type R = Record<Units, NoPremises>;

pub struct Fixture {
    parent: PathBuf,
    pub git: PinnedFile,
    pub writer: GitWriter,
    pub objects: ObjectStore,
}
impl Fixture {
    pub fn new() -> Result<Self> {
        let parent = std::env::temp_dir().join(format!(
            ".writfold-ledger-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&parent)?;
        let git = ContainedRoot::open("/usr/bin")?.pin(&RelativePath::new("git")?)?;
        let writer = {
            let executable = executable(&git)?;
            GitWriter::initialize(
                parent.join("ledger"),
                &executable,
                count(1 << 20)?,
                budgets()?,
                &ACTIVE,
            )?
        };
        let identity =
            RealmIdentity::new("realm-a".parse()?, "kb-a".parse()?, "instance-a".parse()?);
        let objects = ObjectStore::initialize(parent.join("objects"), identity, limits()?)?;
        Ok(Self {
            parent,
            git,
            writer,
            objects,
        })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.parent);
    }
}
pub fn count(n: u64) -> Result<ByteCount> {
    Ok(ByteCount::new(n)?)
}
pub fn budgets() -> Result<GitBudgets> {
    Ok(GitBudgets::new(
        count(1 << 20)?,
        count(1 << 16)?,
        count(1 << 20)?,
        Duration::from_secs(20),
    ))
}
pub fn limits() -> Result<Limits> {
    Ok(Limits::new(1 << 20, 16, 1024, 4096)?)
}
pub fn executable(file: &PinnedFile) -> Result<GitExecutable<'_>> {
    let digest: ContentDigest =
        "eb54b535467e0975c1284d4c4d08eea779aeb248a5d34b02e2ee171382e22201".parse()?;
    let supervisor = SUPERVISOR
        .as_ref()
        .map_err(|e| format!("supervisor: {e}"))?;
    Ok(GitExecutable::admit(
        file,
        &digest,
        count(16 << 20)?,
        supervisor,
    )?)
}
pub fn stream() -> Result<StreamId> {
    Ok("run.1".parse()?)
}
pub fn record(
    id: &str,
    seq: u64,
    pred: Option<&str>,
    units: Units,
    closure: Vec<BlobDigest>,
) -> Result<R> {
    Ok(Record {
        id: id.parse()?,
        stream: stream()?,
        sequence: LedgerSequence::new(seq)?,
        predecessor: pred.map(str::parse).transpose()?,
        profile: ReducerProfile::EngineeringV1,
        payload: units,
        historical: NoPremises,
        closure,
    })
}
