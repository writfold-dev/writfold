//! `writfold trial prepare`: worktree, pinned replay, retention, `Prepared`.
use super::flow::{Event, Premises};
use super::state::{CANCELLED, Trial};
use super::{BUDGET, Result, budgets, limits, scope, stream};
use std::fs;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};
use writfold_domain::LedgerSequence;
use writfold_domain::engineering::ledger::ReducerProfile;
use writfold_domain::worktree::{
    Artifact, Candidate, Capture, PosixIdentity, PosixMetadata, PosixTimestamp, PreparedEdit,
    Recipe, RecipeSpan, Scope, Splice,
};
use writfold_kernel::ledger::Record;
use writfold_runtime::git::{trial_worktree_add, trial_worktree_remove};
use writfold_sources::splicery::{ReplayedEdit, replay};
use writfold_wire::worktree::{capture_digest, prepared_edit_digest, validate_prepared};

/// What `prepare` produced, for display.
#[derive(Debug)]
pub struct Prepared {
    pub run: String,
    pub worktree: std::path::PathBuf,
    pub path: String,
    pub base: String,
    pub prepared: String,
    pub edits: Vec<(u64, u64, usize)>,
}

fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(BUDGET as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > BUDGET {
        return Err(format!("{} is larger than {BUDGET} bytes", path.display()).into());
    }
    Ok(bytes)
}

fn run_id() -> Result<String> {
    let mut entropy = [0u8; 3];
    fs::File::open("/dev/urandom")?.read_exact(&mut entropy)?;
    let hex: String = entropy.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("run.{}.{hex}", super::now()?))
}

fn capture(scope: Scope, path: &str, source: Vec<u8>, file: &Path) -> Result<Capture> {
    let m = fs::symlink_metadata(file)?;
    if !m.file_type().is_file() {
        return Err(format!("{path} is not a regular file").into());
    }
    let metadata = PosixMetadata::new(
        PosixIdentity::new(m.dev(), m.ino()),
        m.mode(),
        m.uid(),
        m.gid(),
        m.nlink(),
        PosixTimestamp::new(m.mtime(), u32::try_from(m.mtime_nsec())?)?,
        PosixTimestamp::new(m.ctime(), u32::try_from(m.ctime_nsec())?)?,
    )?;
    Ok(Capture::new(scope, path.to_owned(), source, metadata)?)
}

/// The exact `writfold-worktree/1` prepared edit for this replay, validated.
fn prepared_edit(
    replayed: &ReplayedEdit,
    recipe: &Recipe,
    capture: &Capture,
    scope: &Scope,
) -> Result<(PreparedEdit, Candidate)> {
    let limits = limits()?;
    let capture_id = capture_digest(capture, limits)?;
    let candidate = Candidate::new(capture_id, replayed.candidate().to_vec());
    let identity = replayed.identity();
    let artifact_hex = identity.source_artifact_sha256();
    let mut sha = [0u8; 32];
    for (i, byte) in sha.iter_mut().enumerate() {
        *byte = u8::from_str_radix(
            artifact_hex
                .get(2 * i..2 * i + 2)
                .ok_or("artifact digest")?,
            16,
        )?;
    }
    let artifact = Artifact::new(identity.profile().to_owned(), sha)?;
    let mut splices = Vec::new();
    for edit in replayed.edits() {
        let origin = RecipeSpan::new(edit.origin.start as u64, edit.origin.end as u64)?;
        splices.push(Splice::new(
            edit.target.lo() as u64,
            edit.target.hi() as u64,
            edit.replacement.clone(),
            origin,
        )?);
    }
    splices.sort_by_key(|s| (s.start(), s.end()));
    let prepared = PreparedEdit::new(
        writfold_wire::worktree::recipe_digest(recipe, limits)?,
        capture_id,
        writfold_wire::worktree::candidate_digest(&candidate, limits)?,
        artifact.clone(),
        artifact,
        splices,
        Vec::new(),
    )?;
    validate_prepared(&prepared, recipe, capture, &candidate, scope, limits)?;
    Ok((prepared, candidate))
}

type PreparedParts = (Event, String, Vec<(u64, u64, usize)>);

/// Replay and retain; refusals here undo the worktree in `prepare`.
fn prepared_event(
    trial: &Trial,
    run: &str,
    path: &str,
    worktree: &Path,
    base: &str,
    recipe_bytes: &[u8],
    report_bytes: &[u8],
) -> Result<PreparedParts> {
    let file = worktree.join(path);
    let source = read_bounded(&file)?;
    let replayed =
        replay(source.clone(), path, recipe_bytes).map_err(|e| format!("recipe refused: {e}"))?;
    let scope = scope(run, base)?;
    let recipe_record = Recipe::new(
        replayed.identity().profile().to_owned(),
        recipe_bytes.to_vec(),
    )?;
    let capture = capture(scope.clone(), path, source.clone(), &file)?;
    let (prepared, candidate) = prepared_edit(&replayed, &recipe_record, &capture, &scope)?;
    let digest = prepared_edit_digest(&prepared, limits()?)?;
    let event = Event::Prepared {
        path: path.to_owned(),
        base: base.to_owned(),
        source: trial.retain(&source)?,
        recipe: trial.retain(recipe_bytes)?,
        candidate: trial.retain(candidate.content())?,
        report: trial.retain(report_bytes)?,
        prepared: *writfold_domain::worktree::WorktreeDigest::as_digest(&digest),
    };
    let edits = replayed
        .edits()
        .iter()
        .map(|e| {
            (
                e.target.lo() as u64,
                e.target.hi() as u64,
                e.replacement.len(),
            )
        })
        .collect();
    Ok((event, digest.to_string(), edits))
}

fn relative(path: &str) -> Result<()> {
    let p = Path::new(path);
    if path.is_empty() || !p.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(format!("{path}: use a plain relative path inside the repository").into());
    }
    Ok(())
}

pub fn prepare(
    state: &Path,
    repo: &Path,
    path: &str,
    recipe: &Path,
    report: Option<&Path>,
) -> Result<Prepared> {
    relative(path)?;
    let trial = Trial::open(state)?;
    let git_dir = repo.join(".git");
    if !git_dir.is_dir() {
        return Err(format!(
            "{} is not a Git repository with a .git directory",
            repo.display()
        )
        .into());
    }
    let recipe_bytes = read_bounded(recipe)?;
    let report_bytes = report.map(read_bounded).transpose()?.unwrap_or_default();
    let run = run_id()?;
    let worktree = trial.worktree(&run);
    let executable = trial.executable()?;
    let base = trial_worktree_add(
        &executable,
        &fs::canonicalize(&git_dir)?,
        &worktree,
        &[path],
        budgets()?,
        &CANCELLED,
    )?;
    let git_dir = fs::canonicalize(&git_dir)?;
    let prepared = match prepared_event(
        &trial,
        &run,
        path,
        &worktree,
        base.as_str(),
        &recipe_bytes,
        &report_bytes,
    ) {
        Ok(prepared) => prepared,
        Err(error) => {
            trial_worktree_remove(&executable, &git_dir, &worktree, budgets()?, &CANCELLED)?;
            return Err(error);
        }
    };
    let (event, digest, edits) = prepared;
    let lease = trial.writer.try_lease()?;
    let ledger = trial.ledger(&lease, &executable)?;
    let record = Record {
        id: format!("{run}.prepared").parse()?,
        stream: stream(&run)?,
        sequence: LedgerSequence::new(1)?,
        predecessor: None,
        profile: ReducerProfile::EngineeringV1,
        closure: event.blobs(),
        payload: event,
        historical: Premises,
    };
    ledger.append(&stream(&run)?, None, record)?;
    Ok(Prepared {
        run,
        worktree,
        path: path.to_owned(),
        base: base.as_str().to_owned(),
        prepared: digest,
        edits,
    })
}
