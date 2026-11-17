//! Gate definitions: `.writfold/gates/<id>.toml`, in the KB root (shared by
//! every project) or in the subject repository (versioned with the code it
//! checks). The schema is `writfold-gate/1`; unknown keys are refused, so a
//! misspelt field never silently changes what a gate means. A definition
//! may name an `instructions` file and ship scripts beside itself
//! ([`super::places`]); their bytes are part of its digest. Reading a
//! definition never runs anything. The checks are in [`super::def_check`]:
//! every fault of a file is reported, each at its line and column.
use crate::error::{KbError, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "writfold-gate/1";
/// Where definitions live, under the KB root or the subject repository.
pub const DIR: &str = ".writfold/gates";
use super::places;
pub use places::ARTIFACTS;
pub(super) const MAX_TIMEOUT: i64 = 86_400;
/// The class of a gate that is a statement, not a check.
pub const ATTESTED: &str = "attested";
/// The class of a gate met by a Weft assessment ([`super::assessed`]).
pub const ASSESSED: &str = "assessed";
/// The class of a check over the run's own records ([`crate::plan::records`]).
pub const RUN_RECORD: &str = "run-record";
/// The class of a gate met by covering its items ([`super::checklist`]).
pub const CHECKLIST: &str = "checklist";
/// Every class, in the order the guide lists them.
pub const CLASSES: [&str; 6] = [
    "mechanical",
    "observed",
    ATTESTED,
    ASSESSED,
    RUN_RECORD,
    CHECKLIST,
];

/// Which root a definition was read from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Origin {
    Kb,
    Repo,
}

impl Origin {
    pub fn name(self) -> &'static str {
        match self {
            Origin::Kb => "kb",
            Origin::Repo => "repo",
        }
    }
}

/// What a gate judges: the subject repository (`gate run`, `gate status`)
/// or a delivery candidate (a delivery mode's `gates`, `deliver check`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Judges {
    Repository,
    Candidate,
}

/// The Markdown file a gate names for the agent to read: the guideline it
/// checks, or, on an attested gate, the thing being attested to.
#[derive(Clone, Debug)]
pub struct Instructions {
    /// As written, relative to the definition's directory.
    pub path: String,
    /// `sha256:<hex>` of its bytes; part of the definition digest.
    pub digest: String,
}

/// One parsed, checked definition.
#[derive(Clone, Debug)]
pub struct Definition {
    pub id: String,
    pub version: i64,
    /// `mechanical`, `observed` or `attested` (no command).
    pub class: String,
    pub argv: Vec<String>,
    pub workdir: String,
    pub timeout_seconds: u64,
    pub scope: Vec<String>,
    pub description: Option<String>,
    pub origin: Origin,
    pub judges: Judges,
    pub instructions: Option<Instructions>,
    /// A run-record gate's built-in requirements (`research.backfilled`,
    /// `run.reflected`); empty for every other class.
    pub require: Vec<String>,
    /// Files named through `{gate}/…` in `argv`, with their digests.
    pub files: Vec<(String, String)>,
    /// An assessed gate's Weft source, checks and binary pin.
    pub assessed: Option<super::assessed::Spec>,
    /// A checklist gate's items.
    pub checklist: Option<super::checklist::Spec>,
    /// `accept`, when written: the standings that meet the gate, in the
    /// order satisfied, attested, waived ([`super::def_accept`]).
    pub accept: Option<Vec<String>>,
    /// The directory holding the definition: what `{gate}` expands to.
    pub dir: PathBuf,
    /// `.writfold/gates/<id>.toml`, relative to its root.
    pub path: String,
    /// `sha256:<hex>` of the canonical form (comments and layout excluded).
    pub digest: String,
}

impl Definition {
    pub fn patterns(&self) -> Vec<&str> {
        self.scope.iter().map(String::as_str).collect()
    }

    /// The standings that meet this gate, for [`writfold_core::policy::met`].
    pub fn policy(&self) -> writfold_core::policy::Accept {
        super::def_accept::policy(self.accept.as_deref())
    }

    /// An attested gate runs no command; only `gate attest` meets it.
    pub fn attested(&self) -> bool {
        self.class == ATTESTED
    }

    /// A checklist gate runs no command: `gate cover` records its items.
    pub fn is_checklist(&self) -> bool {
        self.class == CHECKLIST
    }

    /// A run-record gate runs no command: `gate run` evaluates its
    /// requirements over the run's own records.
    pub fn run_record(&self) -> bool {
        self.class == RUN_RECORD
    }

    /// Whether it judges delivery candidates rather than a repository.
    pub fn on_candidate(&self) -> bool {
        self.judges == Judges::Candidate
    }

    /// The instructions file relative to its root (`.writfold/gates/…`).
    pub fn instructions_path(&self) -> Option<String> {
        self.instructions
            .as_ref()
            .map(|i| format!("{DIR}/{}", i.path))
    }
}

/// Parse and check one definition file's text; `dir` is the directory
/// holding it, where `instructions` and `{gate}/…` files are read. Every
/// fault is reported ([`KbError::Definition`]).
pub fn parse(text_in: &str, stem: &str, origin: Origin, dir: &Path) -> Result<Definition> {
    let file = format!("{DIR}/{stem}.toml");
    super::def_check::check(text_in, stem, origin, dir, &file)
        .map_err(|report| KbError::Definition(Box::new(report)))
}

/// The digest of what the definition means: every field but the
/// description, in a fixed form, with the bytes of its instructions and
/// of every file its `argv` names through `{gate}/…`. Lines for those
/// (and for a candidate subject) are added only when present, so a
/// definition without them keeps the digest it always had.
pub fn digest(d: &Definition) -> String {
    let json = |items: &[String]| serde_json::to_string(items).unwrap_or_default();
    let mut canonical = format!(
        "{SCHEMA}\nid={}\nversion={}\nclass={}\nargv={}\nworkdir={}\ntimeout_seconds={}\nscope={}\n",
        d.id,
        d.version,
        d.class,
        json(&d.argv),
        d.workdir,
        d.timeout_seconds,
        json(&d.scope)
    );
    if d.on_candidate() {
        canonical.push_str("subject=candidate\n");
    }
    if d.run_record() {
        canonical.push_str(&format!("require={}\n", json(&d.require)));
    }
    if let Some(i) = &d.instructions {
        canonical.push_str(&format!("instructions={} {}\n", i.path, i.digest));
    }
    for (path, file) in &d.files {
        canonical.push_str(&format!("file={path} {file}\n"));
    }
    if let Some(spec) = &d.assessed {
        canonical.push_str(&spec.canonical());
    }
    if let Some(spec) = &d.checklist {
        canonical.push_str(&spec.canonical());
    }
    // Only when written, so a definition without it keeps its digest.
    if let Some(accept) = &d.accept {
        canonical.push_str(&format!("accept={}\n", json(accept)));
    }
    format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(canonical.as_bytes()))
    )
}

pub use super::discover::{Unusable, all, find, find_repository};
