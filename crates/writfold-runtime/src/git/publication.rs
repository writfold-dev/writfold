use super::{
    GitBudgets, GitError, GitExecutable, GitPrepared, GitWriteError, GitWriterLease,
    objects::Session,
    preparation::{commit_bytes, prepared_ref},
    traversal, write_objects,
    writer::relative,
    writer_profile,
};
use std::sync::atomic::AtomicBool;
use writfold_domain::{GitObjectId, PlanDigest};

impl GitWriterLease<'_> {
    /// Historical completed-release authority; HEAD is never consulted.
    pub fn completed(
        &self,
        executable: &GitExecutable<'_>,
        budgets: GitBudgets,
        cancelled: &AtomicBool,
    ) -> Result<Option<GitObjectId>, GitError> {
        let session = self.session(executable, budgets, cancelled)?;
        let object = writer_profile::resolve(session.repository, writer_profile::COMPLETED)?;
        if let Some(object) = &object {
            session.object(object, "commit", budgets.stdout)?;
        }
        self.check()?;
        Ok(object)
    }

    /// Exact operation lookup for reconciliation. Absence does not prove that a
    /// failed attempt had no effects, nor does a different head prove rejection.
    pub fn prepared(
        &self,
        executable: &GitExecutable<'_>,
        operation: &PlanDigest,
        budgets: GitBudgets,
        cancelled: &AtomicBool,
    ) -> Result<Option<GitPrepared>, GitError> {
        let session = self.session(executable, budgets, cancelled)?;
        let Some(commit) = writer_profile::resolve(session.repository, &prepared_ref(operation))?
        else {
            self.check()?;
            return Ok(None);
        };
        let prepared = history(&session, &commit, self.identity()?)?;
        if prepared.operation() != operation {
            return Err(GitError::ObjectMismatch);
        }
        self.check()?;
        Ok(Some(prepared))
    }

    /// Exact native operation/parent/manifest observation. Caller may follow
    /// predecessor links with its own bounded history policy, and use the existing
    /// `repository().read_blob` API for exact manifest and payload bytes.
    pub fn history(
        &self,
        executable: &GitExecutable<'_>,
        commit: &GitObjectId,
        budgets: GitBudgets,
        cancelled: &AtomicBool,
    ) -> Result<GitPrepared, GitError> {
        let session = self.session(executable, budgets, cancelled)?;
        let prepared = history(&session, commit, self.identity()?)?;
        self.check()?;
        Ok(prepared)
    }

    /// Atomic Git expected-old CAS, with permanent prepared root already present.
    /// Success observes the process-crash fsync boundary, not power-loss durability
    /// or app authorization. Any error after possible issuance is Unknown, even a
    /// nonzero native exit; never blindly retry or reinterpret a later head mismatch.
    pub fn publish(
        &self,
        executable: &GitExecutable<'_>,
        prepared: &GitPrepared,
        budgets: GitBudgets,
        cancelled: &AtomicBool,
    ) -> Result<(), GitWriteError> {
        let session = self
            .session(executable, budgets, cancelled)
            .map_err(GitWriteError::Unissued)?;
        self.check_prepared(&session, prepared)
            .map_err(GitWriteError::Unissued)?;
        let actual = writer_profile::resolve(session.repository, writer_profile::COMPLETED)
            .map_err(GitWriteError::Unissued)?;
        if let Some(head) = &actual {
            session
                .object(head, "commit", budgets.stdout)
                .map_err(GitWriteError::Unissued)?;
        }
        if actual.as_ref() != prepared.predecessor() {
            return Err(GitWriteError::Conflict { actual });
        }
        write_objects::update(
            &session,
            writer_profile::COMPLETED,
            prepared.commit(),
            prepared.predecessor(),
        )
        .map_err(GitWriteError::Unknown)?;
        self.check().map_err(GitWriteError::Unknown)?;
        let observed = writer_profile::resolve(session.repository, writer_profile::COMPLETED)
            .map_err(GitWriteError::Unknown)?;
        if observed.as_ref() != Some(prepared.commit()) {
            return Err(GitWriteError::Unknown(GitError::ObjectMismatch));
        }
        session.check_stop().map_err(GitWriteError::Unknown)?;
        Ok(())
    }

    fn check_prepared(
        &self,
        session: &Session<'_, '_>,
        prepared: &GitPrepared,
    ) -> Result<(), GitError> {
        if self.identity()? != prepared.repository {
            return Err(GitError::UnsupportedProfile("foreign prepared repository"));
        }
        if writer_profile::resolve(session.repository, &prepared_ref(prepared.operation()))?
            .as_ref()
            != Some(prepared.commit())
        {
            return Err(GitError::UnsupportedProfile(
                "missing or changed prepared retention root",
            ));
        }
        if &history(session, prepared.commit(), self.identity()?)? != prepared {
            return Err(GitError::ObjectMismatch);
        }
        Ok(())
    }
}

fn history(
    session: &Session<'_, '_>,
    commit: &GitObjectId,
    repository: (u64, u64),
) -> Result<GitPrepared, GitError> {
    let bytes = session.object(commit, "commit", session.budgets.stdout)?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| GitError::Malformed("prepared commit encoding"))?;
    let mut lines = text.lines();
    let tree = parse_id(session, lines.next(), "tree ")?;
    let next = lines.next();
    let predecessor = match next {
        Some(line) if line.starts_with("parent ") => {
            Some(parse_id(session, Some(line), "parent ")?)
        }
        _ => None,
    };
    let operation = text
        .rsplit_once("\nwritfold-operation ")
        .and_then(|(_, value)| value.strip_suffix('\n'))
        .ok_or(GitError::Malformed("prepared operation identity"))?
        .parse::<PlanDigest>()
        .map_err(|_| GitError::Malformed("prepared operation identity"))?;
    if commit_bytes(&tree, predecessor.as_ref(), &operation).as_bytes() != bytes {
        return Err(GitError::Malformed("noncanonical prepared commit"));
    }
    let manifest = traversal::lookup(session, commit, &relative("manifest")?)?;
    session.object(&manifest, "blob", session.budgets.blob)?;
    Ok(GitPrepared {
        operation,
        commit: commit.clone(),
        manifest,
        predecessor,
        repository,
    })
}

fn parse_id(
    session: &Session<'_, '_>,
    line: Option<&str>,
    prefix: &str,
) -> Result<GitObjectId, GitError> {
    let text = line
        .and_then(|line| line.strip_prefix(prefix))
        .ok_or(GitError::Malformed("prepared header"))?;
    GitObjectId::new(session.repository.format(), text.to_owned())
        .map_err(|_| GitError::Malformed("prepared header identity"))
}
