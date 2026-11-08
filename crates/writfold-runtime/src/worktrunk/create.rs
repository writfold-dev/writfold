use super::command::{Session, line};
use super::{
    Branch, CreateError, CreateRequest, Created, Destinations, Limits, RepositoryIdentity, Result,
    Source, Worktrunk, WorktrunkError,
};
use super::{integrity, profile};
use crate::fs::{ContainedRoot, LockKey, RelativePath};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use writfold_domain::GitObjectId;

struct Before {
    repository: RepositoryIdentity,
    head: Vec<u8>,
    branch: Vec<u8>,
    files: [u8; 32],
    path: PathBuf,
}
impl Worktrunk<'_, '_> {
    /// Caller retains its durable claim before invoking. A refusal before the
    /// Worktrunk invocation is Unissued; every failure from that boundary onward
    /// is Unknown, even a timeout/nonzero exit or failed independent observation.
    /// No rollback, deletion, branch reuse or automatic retry occurs.
    pub fn create(
        &self,
        source: &Source,
        destinations: &Destinations,
        request: CreateRequest<'_>,
        limits: Limits,
        cancelled: &AtomicBool,
    ) -> Result<Created, CreateError> {
        let CreateRequest { leaf, branch, base } = request;
        let session = Session::new(self, limits, cancelled).map_err(CreateError::Unissued)?;
        let source_key = LockKey::new("knowledge")
            .map_err(WorktrunkError::from)
            .map_err(CreateError::Unissued)?;
        let destination_key = LockKey::new("worktrunk")
            .map_err(WorktrunkError::from)
            .map_err(CreateError::Unissued)?;
        let source_guard = source
            .target
            .try_lock_private(&source.locks, &source_key)
            .map_err(WorktrunkError::from)
            .map_err(CreateError::Unissued)?;
        let destination_guard = destinations
            .target
            .try_lock_private(&destinations.locks, &destination_key)
            .map_err(WorktrunkError::from)
            .map_err(CreateError::Unissued)?;
        let before = preflight(&session, source, destinations, leaf, branch, base)
            .map_err(CreateError::Unissued)?;
        source_guard
            .check_current()
            .map_err(WorktrunkError::from)
            .map_err(CreateError::Unissued)?;
        destination_guard
            .check_current()
            .map_err(WorktrunkError::from)
            .map_err(CreateError::Unissued)?;
        session.check().map_err(CreateError::Unissued)?;
        let mut native = session.command(
            self.tool
                .path()
                .map_err(WorktrunkError::from)
                .map_err(CreateError::Unissued)?,
            &source.path,
        );
        native
            .arg("--config")
            .arg(self.controls.join("empty.toml"))
            .args([
                "switch",
                "--create",
                branch.as_str(),
                "--base",
                base.as_str(),
                "--path",
            ])
            .arg(&before.path)
            .args(["--no-hooks", "--no-cd", "--format", "json"]);
        // Owned process cleanup completes before either managed guard can drop.
        let stdout = session
            .run(&mut native)
            .map_err(|reason| CreateError::Unknown {
                reason,
                worktrunk_stdout: None,
            })?;
        let observed = observe(&session, source, destinations, branch, base, &before)
            .and_then(|()| source_guard.check_current().map_err(Into::into))
            .and_then(|()| destination_guard.check_current().map_err(Into::into));
        if let Err(reason) = observed {
            return Err(CreateError::Unknown {
                reason,
                worktrunk_stdout: Some(stdout),
            });
        }
        Ok(Created {
            repository: before.repository,
            common_directory: source.path.join(".git"),
            path: before.path,
            branch: branch.clone(),
            base: base.clone(),
            worktrunk_stdout: stdout,
        })
    }
}

fn preflight(
    session: &Session<'_, '_, '_>,
    source: &Source,
    destinations: &Destinations,
    leaf: &RelativePath,
    branch: &Branch,
    base: &GitObjectId,
) -> Result<Before> {
    let name = leaf
        .as_path()
        .to_str()
        .ok_or(WorktrunkError::Profile("destination name"))?;
    super::check_name(name)?;
    for (left, right) in [
        (&source.path, &destinations.path),
        (&source.path, &session.adapter.controls),
        (&destinations.path, &session.adapter.controls),
    ] {
        if left.starts_with(right) || right.starts_with(left) {
            return Err(WorktrunkError::Profile(
                "disjoint source/destination/control roots",
            ));
        }
    }
    if base.format() != source.format {
        return Err(WorktrunkError::Profile("base object format"));
    }
    destinations.root.check_private()?;
    destinations.locks.check_private()?;
    profile::absent(&destinations.root, name)?;
    let files = profile::check(source, session.limits, session.deadline, session.cancelled)?;
    let repository = profile::identity(&source.metadata)?;
    check_repository(session, &source.path, source, repository)?;
    let reference = format!("refs/heads/{}", branch.as_str());
    if !session
        .git(
            &source.path,
            &["for-each-ref", "--format=%(refname)", &reference],
        )?
        .is_empty()
    {
        return Err(WorktrunkError::Conflict);
    }
    integrity::verify_base(session, &source.path, base)?;
    let head = session.git(&source.path, &["rev-parse", "--verify", "HEAD"])?;
    let branch = session.git(&source.path, &["symbolic-ref", "HEAD"])?;
    session.check()?;
    Ok(Before {
        repository,
        head,
        branch,
        files,
        path: destinations.path.join(leaf.as_path()),
    })
}

fn observe(
    session: &Session<'_, '_, '_>,
    source: &Source,
    destinations: &Destinations,
    branch: &Branch,
    base: &GitObjectId,
    before: &Before,
) -> Result<()> {
    session.check()?;
    destinations.root.check_private()?;
    let target = ContainedRoot::open(&before.path)?;
    profile::check_output(&target, session.limits, session.deadline, session.cancelled)?;
    // Linked-worktree admission is confined to this write boundary. It never
    // weakens or reuses GitRepository's immutable-reader admission.
    target.pin(&profile::relative(".git")?)?.open_read()?;
    check_repository(session, &before.path, source, before.repository)?;
    let gitdir = session.git(&before.path, &["rev-parse", "--absolute-git-dir"])?;
    let gitdir = Path::new(line(&gitdir)?);
    let worktrees = source.path.join(".git/worktrees");
    if gitdir.parent() != Some(worktrees.as_path()) {
        return Err(WorktrunkError::Observation("linked metadata containment"));
    }
    ContainedRoot::open(gitdir)?.check_current()?;
    let actual = session.git(&before.path, &["rev-parse", "--verify", "HEAD"])?;
    let actual_branch = session.git(&before.path, &["symbolic-ref", "HEAD"])?;
    if line(&actual)? != base.as_str()
        || line(&actual_branch)? != format!("refs/heads/{}", branch.as_str())
    {
        return Err(WorktrunkError::Observation("exact base/branch"));
    }
    if session.git(&source.path, &["rev-parse", "--verify", "HEAD"])? != before.head
        || session.git(&source.path, &["symbolic-ref", "HEAD"])? != before.branch
        || profile::check(source, session.limits, session.deadline, session.cancelled)?
            != before.files
    {
        return Err(WorktrunkError::Observation("original source preservation"));
    }
    target.check_current()?;
    session.check()
}

fn check_repository(
    session: &Session<'_, '_, '_>,
    path: &Path,
    source: &Source,
    expected: RepositoryIdentity,
) -> Result<()> {
    let common = session.git(
        path,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let common = Path::new(line(&common)?);
    let top = session.git(path, &["rev-parse", "--show-toplevel"])?;
    let format = session.git(path, &["rev-parse", "--show-object-format"])?;
    let expected_format = match source.format {
        writfold_domain::GitObjectFormat::Sha1 => "sha1",
        writfold_domain::GitObjectFormat::Sha256 => "sha256",
    };
    if common != source.path.join(".git")
        || Path::new(line(&top)?) != path
        || line(&format)? != expected_format
        || profile::identity(&ContainedRoot::open(common)?)? != expected
    {
        return Err(WorktrunkError::Observation(
            "repository/common-directory/path/format",
        ));
    }
    Ok(())
}
