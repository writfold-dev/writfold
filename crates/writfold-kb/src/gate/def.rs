//! Gate definitions: `.writfold/gates/<id>.toml`, in the KB root (shared by
//! every project) or in the subject repository (versioned with the code it
//! checks). The schema is `writfold-gate/1`; unknown keys are refused, so a
//! misspelt field never silently changes what a gate means.
use super::toml::{self, Value};
use crate::error::{KbError, Result};
use crate::write::is_slug;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "writfold-gate/1";
/// Where definitions live, under the KB root or the subject repository.
pub const DIR: &str = ".writfold/gates";
/// The one placeholder an argument may hold: a fresh local directory for
/// the command's own output files.
pub const ARTIFACTS: &str = "{artifacts}";
const KEYS: [&str; 9] = [
    "schema",
    "id",
    "version",
    "class",
    "argv",
    "workdir",
    "timeout_seconds",
    "scope",
    "description",
];
const MAX_TIMEOUT: i64 = 86_400;
/// The class of a gate that is a statement, not a check.
pub const ATTESTED: &str = "attested";

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

/// A relative path that stays inside its root.
fn contained(path: &str) -> bool {
    !path.starts_with('/')
        && !path.contains('\\')
        && path.split('/').all(|part| part != ".." && !part.is_empty())
}

fn placeholders_ok(arg: &str) -> bool {
    let rest = arg.replace(ARTIFACTS, "");
    !rest.split('{').skip(1).any(|after| {
        after.split_once('}').is_some_and(|(name, _)| {
            !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
    })
}

/// The command fields: `argv`, `workdir` and `timeout_seconds`, required
/// for a check and refused for an attested gate (which runs nothing; it is
/// met only by `gate attest`).
fn command(map: &BTreeMap<String, Value>, class: &str) -> Result<(Vec<String>, String, u64)> {
    if class == ATTESTED {
        if let Some(key) = ["argv", "workdir", "timeout_seconds"]
            .into_iter()
            .find(|k| map.contains_key(*k))
        {
            return Err(bad(format!("an attested gate runs nothing: remove {key}")));
        }
        return Ok((Vec::new(), ".".into(), 0));
    }
    let argv = list(map, "argv")?;
    if argv
        .iter()
        .any(|a| a.is_empty() || a.contains('\0') || !placeholders_ok(a))
    {
        return Err(bad(format!(
            "argv entries must be non-empty, and {ARTIFACTS} is the only placeholder"
        )));
    }
    let workdir = text(map, "workdir")?.unwrap_or_else(|| ".".into());
    if workdir != "." && !contained(&workdir) {
        return Err(bad(
            "workdir must be a relative path inside the repository".into()
        ));
    }
    let timeout_seconds = u64::try_from(int(map, "timeout_seconds", 1, MAX_TIMEOUT)?)
        .map_err(|_| bad("timeout_seconds".into()))?;
    Ok((argv, workdir, timeout_seconds))
}

/// Parse and check one definition file's text.
pub fn parse(text_in: &str, stem: &str, origin: Origin) -> Result<Definition> {
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
    if !matches!(class.as_str(), "mechanical" | "observed" | ATTESTED) {
        return Err(bad("class must be mechanical, observed or attested".into()));
    }
    let (argv, workdir, timeout_seconds) = command(&map, &class)?;
    let scope = list(&map, "scope")?;
    for pattern in &scope {
        let bare = pattern.strip_prefix('!').unwrap_or(pattern);
        if bare.is_empty() || bare.starts_with('/') || bare.contains('\\') {
            return Err(bad(format!(
                "scope pattern {pattern:?} is not a relative glob"
            )));
        }
    }
    let description = text(&map, "description")?;
    if description
        .as_deref()
        .is_some_and(|d| d.contains(['\n', '\r']))
    {
        return Err(bad("description must be one line".into()));
    }
    let version = int(&map, "version", 1, i64::from(u32::MAX))?;
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
        digest: String::new(),
    };
    definition.digest = digest(&definition);
    Ok(definition)
}

/// The digest of what the definition means: every field but the
/// description, in a fixed form.
pub fn digest(d: &Definition) -> String {
    let json = |items: &[String]| serde_json::to_string(items).unwrap_or_default();
    let canonical = format!(
        "{SCHEMA}\nid={}\nversion={}\nclass={}\nargv={}\nworkdir={}\ntimeout_seconds={}\nscope={}\n",
        d.id,
        d.version,
        d.class,
        json(&d.argv),
        d.workdir,
        d.timeout_seconds,
        json(&d.scope)
    );
    format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(canonical.as_bytes()))
    )
}

/// The roots definitions are read from: the KB root, then the subject
/// repository's top when it is another directory.
fn roots(kb: &Path, repo: Option<&Path>) -> Vec<(Origin, PathBuf)> {
    let mut out = vec![(Origin::Kb, kb.join(DIR))];
    if let Some(top) = repo {
        let dir = top.join(DIR);
        let same = match (
            std::fs::canonicalize(&out[0].1),
            std::fs::canonicalize(&dir),
        ) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        };
        if !same {
            out.push((Origin::Repo, dir));
        }
    }
    out
}

/// Every definition, and every file that does not parse (path, why).
pub fn all(kb: &Path, repo: Option<&Path>) -> (Vec<Definition>, Vec<(Origin, String, String)>) {
    let mut found: Vec<Definition> = Vec::new();
    let mut problems = Vec::new();
    for (origin, dir) in roots(kb, repo) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".toml") && !n.starts_with('.'))
            .collect();
        names.sort();
        for name in names {
            let stem = name.trim_end_matches(".toml");
            let path = format!("{DIR}/{name}");
            let parsed = std::fs::read_to_string(dir.join(&name))
                .map_err(KbError::from)
                .and_then(|text| parse(&text, stem, origin));
            match parsed {
                Ok(d) if found.iter().any(|f| f.id == d.id) => problems.push((
                    origin,
                    path,
                    format!("gate {} is defined in both the KB and the repository", d.id),
                )),
                Ok(d) => found.push(d),
                Err(error) => problems.push((origin, path, error.to_string())),
            }
        }
    }
    (found, problems)
}

/// The definition of `id`, refusing a missing, malformed or doubly defined
/// gate.
pub fn find(kb: &Path, repo: Option<&Path>, id: &str) -> Result<Definition> {
    let (found, problems) = all(kb, repo);
    let name = format!("{DIR}/{id}.toml");
    if let Some((_, path, why)) = problems.iter().find(|(_, path, _)| *path == name) {
        return Err(KbError::Refused(format!("{path}: {why}")));
    }
    found.into_iter().find(|d| d.id == id).ok_or_else(|| {
        KbError::Refused(format!(
            "no gate {id}: define {name} in the KB root or the repository"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::{Origin, parse};

    const UNIT: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"scripts/gate\", \"unit\", \"--receipt\", \"{artifacts}/unit.json\"]\ntimeout_seconds = 60\nscope = [\"**\", \"!docs/**\"]\n";

    #[test]
    fn a_definition_parses_and_its_digest_ignores_layout() -> Result<(), String> {
        let d = parse(UNIT, "unit", Origin::Repo).map_err(|e| e.to_string())?;
        assert_eq!((d.workdir.as_str(), d.timeout_seconds), (".", 60));
        let commented = UNIT.replace("version = 1\n", "# why\nversion   = 1 # first\n");
        let again = parse(&commented, "unit", Origin::Kb).map_err(|e| e.to_string())?;
        assert_eq!(d.digest, again.digest);
        let changed =
            parse(&UNIT.replace("60", "61"), "unit", Origin::Kb).map_err(|e| e.to_string())?;
        assert_ne!(d.digest, changed.digest);
        for (from, to) in [
            ("id = \"unit\"", "id = \"other\""),
            ("class = \"mechanical\"", "class = \"assessed\""),
            ("\"unit\", \"--receipt\"", "\"{receipt}\", \"--receipt\""),
            ("timeout_seconds = 60", "timeout_seconds = 0"),
            ("scope = [\"**\", \"!docs/**\"]", "scope = []"),
            ("scope = [\"**\", \"!docs/**\"]", "scope = [\"/abs\"]"),
            ("version = 1", "version = 1\nshell = \"sh -c\""),
            ("version = 1", "version = 1\nworkdir = \"../up\""),
            ("writfold-gate/1", "writfold-gate/2"),
            ("class = \"mechanical\"", "class = \"attested\""),
        ] {
            assert!(
                parse(&UNIT.replace(from, to), "unit", Origin::Kb).is_err(),
                "{to}"
            );
        }
        let attested = "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\n";
        let d = parse(attested, "review", Origin::Kb).map_err(|e| e.to_string())?;
        assert!(d.attested() && d.argv.is_empty());
        Ok(())
    }
}
