//! Gate definitions: `.writfold/gates/<id>.toml`, in the KB root (shared by
//! every project) or in the subject repository (versioned with the code it
//! checks). The schema is `writfold-gate/1`; unknown keys are refused, so a
//! misspelt field never silently changes what a gate means. A definition
//! may name an `instructions` file and ship scripts beside itself
//! ([`super::places`]); their bytes are part of its digest. Reading a
//! definition never runs anything.
use super::places;
use super::toml::{self, Value};
use crate::error::{KbError, Result};
use crate::write::is_slug;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "writfold-gate/1";
/// Where definitions live, under the KB root or the subject repository.
pub const DIR: &str = ".writfold/gates";
pub use places::ARTIFACTS;
const KEYS: [&str; 14] = [
    "schema",
    "id",
    "version",
    "class",
    "argv",
    "workdir",
    "timeout_seconds",
    "scope",
    "description",
    "instructions",
    "subject",
    "weft",
    "checks",
    "weft_sha256",
];
const MAX_TIMEOUT: i64 = 86_400;
/// The class of a gate that is a statement, not a check.
pub const ATTESTED: &str = "attested";
/// The class of a gate met by a Weft assessment ([`super::assessed`]).
pub const ASSESSED: &str = "assessed";

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
    /// Files named through `{gate}/…` in `argv`, with their digests.
    pub files: Vec<(String, String)>,
    /// An assessed gate's Weft source, checks and binary pin.
    pub assessed: Option<super::assessed::Spec>,
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

    /// An attested gate runs no command; only `gate attest` meets it.
    pub fn attested(&self) -> bool {
        self.class == ATTESTED
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

fn text(map: &BTreeMap<String, Value>, key: &str) -> Result<Option<String>> {
    match map.get(key) {
        None => Ok(None),
        Some(Value::Str(s)) => Ok(Some(s.clone())),
        Some(_) => Err(bad(format!("{key} must be a string"))),
    }
}

fn bad(why: String) -> KbError {
    KbError::Refused(format!("gate definition: {why}"))
}

fn need(map: &BTreeMap<String, Value>, key: &str) -> Result<String> {
    text(map, key)?.ok_or_else(|| bad(format!("{key} is required")))
}

fn list(map: &BTreeMap<String, Value>, key: &str) -> Result<Vec<String>> {
    match map.get(key) {
        Some(Value::List(items)) if !items.is_empty() => Ok(items.clone()),
        _ => Err(bad(format!("{key} must be a non-empty array of strings"))),
    }
}

fn int(map: &BTreeMap<String, Value>, key: &str, low: i64, high: i64) -> Result<i64> {
    match map.get(key) {
        Some(Value::Int(n)) if (low..=high).contains(n) => Ok(*n),
        _ => Err(bad(format!(
            "{key} must be an integer from {low} to {high}"
        ))),
    }
}

/// Check each argument's placeholders against what the gate judges: a
/// repository gate may not use `{candidate}`; a candidate gate must use it
/// and may not use `{artifacts}` (its receipt keeps no local files).
fn placeholders(argv: &[String], judges: Judges) -> Result<()> {
    let mut all = Vec::new();
    for arg in argv {
        if arg.is_empty() || arg.contains('\0') {
            return Err(bad("argv entries must be non-empty".into()));
        }
        all.extend(places::used(arg).map_err(bad)?);
    }
    let has = |p: &str| all.contains(&p);
    match judges {
        Judges::Repository if has(places::CANDIDATE) => Err(bad(format!(
            "{} is for gates with subject = \"candidate\"",
            places::CANDIDATE
        ))),
        Judges::Candidate if !has(places::CANDIDATE) => Err(bad(format!(
            "a candidate gate must pass {} to its command",
            places::CANDIDATE
        ))),
        Judges::Candidate if has(places::ARTIFACTS) => Err(bad(format!(
            "{} is for repository gates",
            places::ARTIFACTS
        ))),
        _ => Ok(()),
    }
}

/// The command fields: `argv`, `workdir` and `timeout_seconds`, required
/// for a check and refused for an attested gate (which runs nothing; it is
/// met only by `gate attest`).
fn command(
    map: &BTreeMap<String, Value>,
    class: &str,
    judges: Judges,
) -> Result<(Vec<String>, String, u64)> {
    if class == ATTESTED || class == ASSESSED {
        let runs = match class == ATTESTED {
            true => "an attested gate runs nothing",
            false => "an assessed gate runs only the pinned weft binary",
        };
        if let Some(key) = ["argv", "workdir", "timeout_seconds"]
            .into_iter()
            .filter(|k| class == ATTESTED || *k != "timeout_seconds")
            .find(|k| map.contains_key(*k))
        {
            return Err(bad(format!("{runs}: remove {key}")));
        }
        let timeout = match class == ASSESSED {
            true => u64::try_from(int(map, "timeout_seconds", 1, MAX_TIMEOUT)?)
                .map_err(|_| bad("timeout_seconds".into()))?,
            false => 0,
        };
        return Ok((Vec::new(), ".".into(), timeout));
    }
    let argv = list(map, "argv")?;
    placeholders(&argv, judges)?;
    let workdir = text(map, "workdir")?.unwrap_or_else(|| ".".into());
    if workdir != "." && !places::contained(&workdir) {
        return Err(bad(
            "workdir must be a relative path inside the repository".into()
        ));
    }
    let timeout_seconds = u64::try_from(int(map, "timeout_seconds", 1, MAX_TIMEOUT)?)
        .map_err(|_| bad("timeout_seconds".into()))?;
    Ok((argv, workdir, timeout_seconds))
}

/// `subject`: `repository` (the default) or `candidate`. A candidate gate
/// lives in the KB root, where delivery modes look for it.
fn judges(map: &BTreeMap<String, Value>, origin: Origin) -> Result<Judges> {
    match text(map, "subject")?.as_deref() {
        None | Some("repository") => Ok(Judges::Repository),
        Some("candidate") if origin == Origin::Kb => Ok(Judges::Candidate),
        Some("candidate") => Err(bad(
            "a candidate gate belongs in the KB root's .writfold/gates/, where delivery modes read it".into(),
        )),
        Some(_) => Err(bad("subject must be repository or candidate".into())),
    }
}

/// `scope`: required for a repository gate, refused for a candidate gate
/// (whose subject is the candidate's bytes).
fn scope(map: &BTreeMap<String, Value>, judges: Judges, class: &str) -> Result<Vec<String>> {
    if class == ASSESSED {
        return match map.contains_key("scope") || judges == Judges::Candidate {
            true => Err(bad(
                "an assessed gate judges its Weft source and cited evidence: remove scope and subject".into(),
            )),
            false => Ok(Vec::new()),
        };
    }
    if judges == Judges::Candidate {
        if map.contains_key("scope") {
            return Err(bad(
                "a candidate gate judges the candidate's bytes: remove scope".into(),
            ));
        }
        return Ok(Vec::new());
    }
    let scope = list(map, "scope")?;
    for pattern in &scope {
        let bare = pattern.strip_prefix('!').unwrap_or(pattern);
        if bare.is_empty() || bare.starts_with('/') || bare.contains('\\') {
            return Err(bad(format!(
                "scope pattern {pattern:?} is not a relative glob"
            )));
        }
    }
    Ok(scope)
}

/// `instructions`: a Markdown file inside the definition's directory.
fn instructions(map: &BTreeMap<String, Value>, dir: &Path) -> Result<Option<Instructions>> {
    let Some(path) = text(map, "instructions")? else {
        return Ok(None);
    };
    if !path.ends_with(".md") {
        return Err(bad(format!("instructions {path:?} must name a .md file")));
    }
    let real = places::resolve(dir, &path).map_err(|why| bad(format!("instructions: {why}")))?;
    let digest = places::digest_file(&real).map_err(bad)?;
    Ok(Some(Instructions { path, digest }))
}

/// Parse and check one definition file's text; `dir` is the directory
/// holding it, where `instructions` and `{gate}/…` files are read.
pub fn parse(text_in: &str, stem: &str, origin: Origin, dir: &Path) -> Result<Definition> {
    let map = toml::parse(text_in).map_err(bad)?;
    if let Some(key) = map.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(bad(format!("unknown key {key}")));
    }
    if need(&map, "schema")? != SCHEMA {
        return Err(bad(format!("schema must be {SCHEMA}")));
    }
    let id = need(&map, "id")?;
    if !is_slug(&id) || id != stem {
        return Err(bad(format!(
            "id {id:?} must be a slug equal to the file name {stem}"
        )));
    }
    let class = need(&map, "class")?;
    if !matches!(
        class.as_str(),
        "mechanical" | "observed" | ATTESTED | ASSESSED
    ) {
        return Err(bad(
            "class must be mechanical, observed, attested or assessed".into(),
        ));
    }
    let judges = judges(&map, origin)?;
    let (argv, workdir, timeout_seconds) = command(&map, &class, judges)?;
    let scope = scope(&map, judges, &class)?;
    let assessed = super::assessed::spec::parse(&map, &class, dir).map_err(bad)?;
    let description = text(&map, "description")?;
    if description
        .as_deref()
        .is_some_and(|d| d.contains(['\n', '\r']))
    {
        return Err(bad("description must be one line".into()));
    }
    let version = int(&map, "version", 1, i64::from(u32::MAX))?;
    let files = places::gate_files(&argv, dir).map_err(bad)?;
    let mut definition = Definition {
        path: format!("{DIR}/{id}.toml"),
        id,
        version,
        class,
        argv,
        workdir,
        timeout_seconds,
        scope,
        description,
        origin,
        judges,
        instructions: instructions(&map, dir)?,
        files,
        assessed,
        dir: dir.to_path_buf(),
        digest: String::new(),
    };
    definition.digest = digest(&definition);
    Ok(definition)
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
    if let Some(i) = &d.instructions {
        canonical.push_str(&format!("instructions={} {}\n", i.path, i.digest));
    }
    for (path, file) in &d.files {
        canonical.push_str(&format!("file={path} {file}\n"));
    }
    if let Some(spec) = &d.assessed {
        canonical.push_str(&spec.canonical());
    }
    format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(canonical.as_bytes()))
    )
}

pub use super::discover::{all, find, find_repository};
