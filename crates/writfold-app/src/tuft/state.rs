//! Which state `writfold tuft run` uses, and the state Tuft keeps in a KB.
//!
//! Tuft reads two things from a state: the Git executable it runs, and the
//! directory where `retain` binds evidence (`evidence/<binding>.json`,
//! `.bytes` and `.line`, as `writfold trial retain` writes them). It never
//! reads the trial's operator key, policy, ledger or worktrees.
//!
//! - `--state DIR`, else a non-empty `WRITFOLD_STATE`: that trial state,
//!   exactly as before (it must have been made by `writfold trial init`).
//! - Otherwise, when a KB under Git is found (`--kb DIR`, or as `writfold kb`
//!   finds it): `<KB git dir>/writfold/tuft-state/`, created on first use
//!   with only `evidence/` (both owner-only). Git is the executable the `kb`
//!   verbs use (`$WRITFOLD_GIT`, else `git` on `PATH`), observed per run;
//!   no executable digest, key, policy or ledger is kept there.
//! - Otherwise the default trial state, when `trial init` made it.
//! - Otherwise a refusal naming both ways forward.
//!
//! Bindings keep the trial realm (`trial`), so the same bytes bind to the
//! same digest in either state.
use crate::trial::Result;
use crate::trial::retain::EvidenceStore;
use crate::trial::state_dir::{StateDir, Uses};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use writfold_domain::RealmId;
use writfold_kb::workspace::Workspace;

/// The directory name under `<KB git dir>/writfold/`.
pub const DIR: &str = "tuft-state";

/// Tuft's KB-owned state: an `evidence/` directory, nothing else.
pub struct TuftState {
    root: PathBuf,
    realm: RealmId,
}

impl TuftState {
    /// Where the state of the KB whose Git directory is `git_dir` lives.
    pub fn path(git_dir: &Path) -> PathBuf {
        git_dir.join("writfold").join(DIR)
    }

    /// Create the state (owner-only) unless it exists; never replaces one.
    pub fn ensure(root: &Path) -> Result<()> {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(root.join("evidence"))?;
        Ok(())
    }

    pub fn open(root: &Path) -> Result<Self> {
        if !root.join("evidence").is_dir() {
            let shown = root.display();
            return Err(format!("{shown} is not a Tuft state (no evidence/ directory)").into());
        }
        Ok(Self {
            root: root.to_owned(),
            realm: "trial".parse()?,
        })
    }
}

impl EvidenceStore for TuftState {
    fn realm(&self) -> &RealmId {
        &self.realm
    }
    fn evidence_dir(&self) -> PathBuf {
        self.root.join("evidence")
    }
    /// The bytes are kept in `evidence/<binding>.bytes` only.
    fn keep(&self, _bytes: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// The state a run uses.
#[derive(Debug, PartialEq, Eq)]
pub enum Chosen {
    /// A trial state made by `writfold trial init`.
    Trial(PathBuf),
    /// Tuft's state in a KB's Git directory (created if it was missing).
    Kb(PathBuf),
}

/// Choose from `--state`, the process environment, and `kb`, which finds
/// the KB (called only when neither `--state` nor `WRITFOLD_STATE` is set).
pub fn choose(option: Option<&str>, kb: &dyn Fn() -> Result<Option<Workspace>>) -> Result<Chosen> {
    let env = std::env::var_os("WRITFOLD_STATE").filter(|v| !v.is_empty());
    let home = std::env::var_os("HOME").filter(|v| !v.is_empty());
    choose_from(
        option,
        env.as_deref().map(Path::new),
        home.as_deref().map(Path::new),
        kb,
    )
}

pub(crate) fn choose_from(
    option: Option<&str>,
    env: Option<&Path>,
    home: Option<&Path>,
    kb: &dyn Fn() -> Result<Option<Workspace>>,
) -> Result<Chosen> {
    if option.is_some() || env.is_some() {
        let dir = StateDir::choose(option, env, home)?;
        dir.require_initialized(Uses::AnyState)?;
        return Ok(Chosen::Trial(dir.path));
    }
    if let Some(repo) = kb()?.and_then(|ws| ws.repo) {
        let path = TuftState::path(&repo.git_dir);
        TuftState::ensure(&path)?;
        return Ok(Chosen::Kb(path));
    }
    let default = StateDir::choose(None, None, home).ok();
    if let Some(dir) = default.as_ref()
        && dir.require_initialized(Uses::AnyState).is_ok()
    {
        return Ok(Chosen::Trial(dir.path.clone()));
    }
    Err(refusal(default.as_ref()).into())
}

fn refusal(default: Option<&StateDir>) -> String {
    let mut text =
        "no state for Tuft: no KB under Git was found, and --state and WRITFOLD_STATE are unset"
            .to_owned();
    if let Some(dir) = default {
        let shown = dir.path.display();
        text.push_str(&format!(
            " (the default trial state {shown} does not exist)"
        ));
    }
    text.push_str(
        "\n  run inside a KB, or pass --kb DIR: Tuft then keeps its state in the KB's Git \
         directory (writfold/tuft-state)\
         \n  or pass --state DIR, or set WRITFOLD_STATE, naming a state made by `writfold trial init`",
    );
    text
}
