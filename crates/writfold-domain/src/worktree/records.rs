use super::*;
use crate::{ActorId, RealmId, RunId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scope {
    realm: RealmId,
    run: RunId,
    repo: RepositoryId,
    worktree: WorktreeId,
    binding: BindingRevision,
}
impl Scope {
    pub fn new(
        realm: RealmId,
        run: RunId,
        repo: RepositoryId,
        worktree: WorktreeId,
        binding: BindingRevision,
    ) -> Result<Self> {
        for value in [realm.as_str(), run.as_str()] {
            label(value)?;
        }
        Ok(Self {
            realm,
            run,
            repo,
            worktree,
            binding,
        })
    }
    pub fn realm(&self) -> &RealmId {
        &self.realm
    }
    pub fn run(&self) -> &RunId {
        &self.run
    }
    pub fn repo(&self) -> &RepositoryId {
        &self.repo
    }
    pub fn worktree(&self) -> &WorktreeId {
        &self.worktree
    }
    pub fn binding(&self) -> &BindingRevision {
        &self.binding
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recipe {
    language_profile: String,
    source: Vec<u8>,
}
impl Recipe {
    pub fn new(language_profile: String, source: Vec<u8>) -> Result<Self> {
        label(&language_profile)?;
        Ok(Self {
            language_profile,
            source,
        })
    }
    pub fn language_profile(&self) -> &str {
        &self.language_profile
    }
    pub fn source(&self) -> &[u8] {
        &self.source
    }
}
/// Exact POSIX timestamp, including pre-epoch seconds; no rounding or normalization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PosixTimestamp {
    seconds: i64,
    nanoseconds: u32,
}
impl PosixTimestamp {
    pub fn new(seconds: i64, nanoseconds: u32) -> Result<Self> {
        if nanoseconds >= 1_000_000_000 {
            return Err(InvalidWorktree);
        }
        Ok(Self {
            seconds,
            nanoseconds,
        })
    }
    pub fn seconds(&self) -> i64 {
        self.seconds
    }
    pub fn nanoseconds(&self) -> u32 {
        self.nanoseconds
    }
}
/// Exact native device/inode observation, not a logical resource identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PosixIdentity {
    device: u64,
    inode: u64,
}
impl PosixIdentity {
    pub fn new(device: u64, inode: u64) -> Self {
        Self { device, inode }
    }
    pub fn device(&self) -> u64 {
        self.device
    }
    pub fn inode(&self) -> u64 {
        self.inode
    }
}
/// Observed metadata, not proof of a race-free host capture. Only this policy exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PosixMetadata {
    identity: PosixIdentity,
    mode: u32,
    uid: u32,
    gid: u32,
    mtime: PosixTimestamp,
    ctime: PosixTimestamp,
}
impl PosixMetadata {
    pub const POLICY: &'static str = "posix-regular-single-link/1";
    pub fn new(
        identity: PosixIdentity,
        mode: u32,
        uid: u32,
        gid: u32,
        links: u64,
        mtime: PosixTimestamp,
        ctime: PosixTimestamp,
    ) -> Result<Self> {
        if mode & 0o170000 != 0o100000 || mode & !0o177777 != 0 || links != 1 {
            return Err(InvalidWorktree);
        }
        Ok(Self {
            identity,
            mode,
            uid,
            gid,
            mtime,
            ctime,
        })
    }
    pub fn device(&self) -> u64 {
        self.identity.device()
    }
    pub fn inode(&self) -> u64 {
        self.identity.inode()
    }
    pub fn mode(&self) -> u32 {
        self.mode
    }
    pub fn uid(&self) -> u32 {
        self.uid
    }
    pub fn gid(&self) -> u32 {
        self.gid
    }
    pub fn mtime(&self) -> PosixTimestamp {
        self.mtime
    }
    pub fn ctime(&self) -> PosixTimestamp {
        self.ctime
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capture {
    scope: Scope,
    path: String,
    content: Vec<u8>,
    metadata: PosixMetadata,
}
impl Capture {
    pub fn new(
        scope: Scope,
        path: String,
        content: Vec<u8>,
        metadata: PosixMetadata,
    ) -> Result<Self> {
        label(&path)?;
        if path.contains('\\')
            || path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
        {
            return Err(InvalidWorktree);
        }
        Ok(Self {
            scope,
            path,
            content,
            metadata,
        })
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn content(&self) -> &[u8] {
        &self.content
    }
    pub fn metadata(&self) -> &PosixMetadata {
        &self.metadata
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    capture: CaptureDigest,
    content: Vec<u8>,
}
impl Candidate {
    pub fn new(capture: CaptureDigest, content: Vec<u8>) -> Self {
        Self { capture, content }
    }
    pub fn capture(&self) -> CaptureDigest {
        self.capture
    }
    pub fn content(&self) -> &[u8] {
        &self.content
    }
}
/// Principal must be authenticated separately by the host; decoding does not do that.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestKey {
    principal: ActorId,
    realm: RealmId,
    run: RunId,
    opaque_key: Vec<u8>,
}
impl RequestKey {
    pub fn new(
        principal: ActorId,
        realm: RealmId,
        run: RunId,
        opaque_key: Vec<u8>,
    ) -> Result<Self> {
        for value in [principal.as_str(), realm.as_str(), run.as_str()] {
            label(value)?;
        }
        if opaque_key.is_empty() || opaque_key.len() > 4096 {
            return Err(InvalidWorktree);
        }
        Ok(Self {
            principal,
            realm,
            run,
            opaque_key,
        })
    }
    pub fn principal(&self) -> &ActorId {
        &self.principal
    }
    pub fn realm(&self) -> &RealmId {
        &self.realm
    }
    pub fn run(&self) -> &RunId {
        &self.run
    }
    pub fn opaque_key(&self) -> &[u8] {
        &self.opaque_key
    }
}
/// Immutable semantic mapping. Persist with insert-or-compare, never last-write-wins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestMapping {
    key: RequestKeyDigest,
    intent: crate::engineering::EffectIntentDigest,
    effect: EffectId,
}
impl RequestMapping {
    pub fn new(
        key: RequestKeyDigest,
        intent: crate::engineering::EffectIntentDigest,
        effect: EffectId,
    ) -> Self {
        Self {
            key,
            intent,
            effect,
        }
    }
    pub fn key(&self) -> RequestKeyDigest {
        self.key
    }
    pub fn intent(&self) -> crate::engineering::EffectIntentDigest {
        self.intent
    }
    pub fn effect(&self) -> &EffectId {
        &self.effect
    }
    pub fn agrees(&self, other: &Self) -> bool {
        self == other
    }
}
