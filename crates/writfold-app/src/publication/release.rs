//! The Git side of a publication: the exclusive release lease, the canonical
//! release manifest and exact historical interpretation of a release.
use super::candidate::digest;
use super::{PublicationError, ReleasePin};
use std::sync::atomic::AtomicBool;
use writfold_domain::{ContentDigest, GitObjectId, PlanDigest};
use writfold_runtime::fs::RelativePath;
use writfold_runtime::git::{GitBudgets, GitExecutable, GitPrepared, GitWriter, GitWriterLease};

const VERSION: &str = "writfold-dolt-release/1";
/// Release history walked by `observe` before it refuses as too deep.
const HISTORY: usize = 1024;
pub(super) const CANDIDATE_PATH: &str = "candidate";

/// Canonical release manifest: what one completed release names exactly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manifest<P> {
    pub operation: PlanDigest,
    pub candidate: ContentDigest,
    /// The retained Git blob holding the candidate's canonical bytes.
    pub blob: GitObjectId,
    pub pin: P,
}

impl<P: ReleasePin> Manifest<P> {
    pub fn encode(&self) -> Result<Vec<u8>, PublicationError> {
        let pin = String::from_utf8(self.pin.encode()?)
            .map_err(|_| PublicationError::Manifest("pin encoding"))?;
        if pin.contains('\n') {
            return Err(PublicationError::Manifest("pin encoding"));
        }
        Ok(format!(
            "{VERSION}\noperation {}\ncandidate {}\nblob {}\npin {pin}\n",
            self.operation.as_str(),
            self.candidate.as_str(),
            self.blob.as_str()
        )
        .into_bytes())
    }

    /// Strict: exactly the canonical encoding of the decoded fields.
    pub fn decode(bytes: &[u8]) -> Result<Self, PublicationError> {
        let text =
            std::str::from_utf8(bytes).map_err(|_| PublicationError::Manifest("encoding"))?;
        let mut lines = text.strip_suffix('\n').unwrap_or_default().split('\n');
        if lines.next() != Some(VERSION) {
            return Err(PublicationError::Manifest("version"));
        }
        let mut field = |name: &str| {
            lines
                .next()
                .and_then(|line| line.strip_prefix(name))
                .and_then(|rest| rest.strip_prefix(' '))
                .ok_or(PublicationError::Manifest("field order"))
        };
        let operation = field("operation")?.parse();
        let candidate = field("candidate")?.parse();
        let blob = field("blob")?.parse();
        let pin = P::decode(field("pin")?.as_bytes())?;
        let manifest = Self {
            operation: operation.map_err(|_| PublicationError::Manifest("operation"))?,
            candidate: candidate.map_err(|_| PublicationError::Manifest("candidate"))?,
            blob: blob.map_err(|_| PublicationError::Manifest("blob"))?,
            pin,
        };
        if manifest.encode()? != bytes {
            return Err(PublicationError::Manifest("noncanonical"));
        }
        Ok(manifest)
    }
}

/// One completed release, interpreted exactly from retained Git objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Released<P> {
    pub release: GitObjectId,
    pub predecessor: Option<GitObjectId>,
    pub manifest: Manifest<P>,
}

/// The realm's exclusive Git release lease, held for the publisher's lifetime.
#[derive(Debug)]
pub struct Releases<'a, 'f> {
    writer: &'a GitWriter,
    lease: GitWriterLease<'a>,
    executable: &'a GitExecutable<'f>,
    budgets: GitBudgets,
}

impl<'a, 'f> Releases<'a, 'f> {
    /// Takes the writer's exclusive cooperating-writer lease without waiting.
    pub fn lease(
        writer: &'a GitWriter,
        executable: &'a GitExecutable<'f>,
        budgets: GitBudgets,
    ) -> Result<Self, PublicationError> {
        Ok(Self {
            writer,
            lease: writer.try_lease()?,
            executable,
            budgets,
        })
    }

    /// The native lease, for owners that build a genesis release.
    pub fn native(&self) -> &GitWriterLease<'a> {
        &self.lease
    }
    pub(super) fn executable(&self) -> &GitExecutable<'f> {
        self.executable
    }
    pub(super) fn budgets(&self) -> GitBudgets {
        self.budgets
    }

    /// The completed ref (the visibility point), never HEAD.
    pub fn completed(
        &self,
        cancelled: &AtomicBool,
    ) -> Result<Option<GitObjectId>, PublicationError> {
        Ok(self
            .lease
            .completed(self.executable, self.budgets, cancelled)?)
    }

    pub(super) fn prepared(
        &self,
        operation: &PlanDigest,
        cancelled: &AtomicBool,
    ) -> Result<Option<GitPrepared>, PublicationError> {
        Ok(self
            .lease
            .prepared(self.executable, operation, self.budgets, cancelled)?)
    }

    /// Exact interpretation of `release`: its canonical manifest, which must
    /// name the commit's operation, and the retained candidate blob, whose
    /// bytes must hash to the manifest's candidate digest.
    pub fn interpret<P: ReleasePin>(
        &self,
        release: &GitObjectId,
        cancelled: &AtomicBool,
    ) -> Result<Released<P>, PublicationError> {
        let prepared = self
            .lease
            .history(self.executable, release, self.budgets, cancelled)?;
        let manifest = self.blob(release, "manifest", prepared.manifest(), cancelled)?;
        let manifest = Manifest::<P>::decode(&manifest)?;
        if &manifest.operation != prepared.operation() {
            return Err(PublicationError::Manifest(
                "operation differs from its commit",
            ));
        }
        let candidate = self.blob(release, CANDIDATE_PATH, &manifest.blob, cancelled)?;
        if digest(&candidate)? != manifest.candidate {
            return Err(PublicationError::Manifest(
                "candidate blob differs from its digest",
            ));
        }
        Ok(Released {
            release: release.clone(),
            predecessor: prepared.predecessor().cloned(),
            manifest,
        })
    }

    /// Whether `prepared` is `completed` or on its predecessor chain. The walk
    /// stops at the prepared commit's own predecessor: a chain through the
    /// prepared commit passes it before reaching its parent.
    pub(super) fn includes(
        &self,
        completed: Option<&GitObjectId>,
        prepared: &GitPrepared,
        cancelled: &AtomicBool,
    ) -> Result<bool, PublicationError> {
        let mut cursor = completed.cloned();
        for _ in 0..HISTORY {
            let Some(current) = cursor else {
                return Ok(false);
            };
            if &current == prepared.commit() {
                return Ok(true);
            }
            if Some(&current) == prepared.predecessor() {
                return Ok(false);
            }
            let history = self
                .lease
                .history(self.executable, &current, self.budgets, cancelled)?;
            cursor = history.predecessor().cloned();
        }
        Err(PublicationError::Manifest(
            "release history deeper than the bound",
        ))
    }

    fn blob(
        &self,
        commit: &GitObjectId,
        path: &str,
        blob: &GitObjectId,
        cancelled: &AtomicBool,
    ) -> Result<Vec<u8>, PublicationError> {
        let path = RelativePath::new(path).map_err(|_| PublicationError::Manifest("path"))?;
        let read = self.writer.repository().read_blob(
            self.executable,
            commit,
            &path,
            blob,
            self.budgets,
            cancelled,
        )?;
        Ok(read.bytes().to_vec())
    }
}
