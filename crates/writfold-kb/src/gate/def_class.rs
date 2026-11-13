//! The per-class rules of a gate definition ([`super::def`]): which
//! command fields a class takes, a run-record gate's `require`, a
//! checklist gate's subject, and the placeholders a command may use.
use super::def::{
    ASSESSED, ATTESTED, CHECKLIST, Judges, MAX_TIMEOUT, RUN_RECORD, bad, int, list, text,
};
use super::places;
use super::toml::Value;
use crate::error::Result;
use std::collections::BTreeMap;

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
/// for a check and refused for an attested or checklist gate (which runs
/// nothing; it is met only by `gate attest` or `gate cover`).
pub(super) fn command(
    map: &BTreeMap<String, Value>,
    class: &str,
    judges: Judges,
) -> Result<(Vec<String>, String, u64)> {
    if [ATTESTED, ASSESSED, RUN_RECORD, CHECKLIST].contains(&class) {
        let runs = match class {
            ATTESTED => "an attested gate runs nothing",
            CHECKLIST => "a checklist gate runs nothing (gate cover records its items)",
            ASSESSED => "an assessed gate runs only the pinned weft binary",
            _ => "a run-record gate runs no command",
        };
        if let Some(key) = ["argv", "workdir", "timeout_seconds"]
            .into_iter()
            .filter(|k| class == ATTESTED || class == CHECKLIST || *k != "timeout_seconds")
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

/// `require`: the built-in requirements of a run-record gate, which judges
/// the run's own records (so it takes no `scope` or `subject`); refused on
/// every other class.
pub(super) fn require(map: &BTreeMap<String, Value>, class: &str) -> Result<Vec<String>> {
    if class != RUN_RECORD {
        return match map.contains_key("require") {
            true => Err(bad(format!("require is for class = \"{RUN_RECORD}\""))),
            false => Ok(Vec::new()),
        };
    }
    if let Some(key) = ["scope", "subject"]
        .into_iter()
        .find(|k| map.contains_key(*k))
    {
        return Err(bad(format!(
            "a run-record gate judges the run's own records: remove {key}"
        )));
    }
    let require = list(map, "require")?;
    crate::plan::records::known(&require).map_err(bad)?;
    Ok(require)
}

/// A checklist gate judges a repository's content in its scope: it takes
/// no `subject` (a candidate has no cover rows).
pub(super) fn checklist_subject(map: &BTreeMap<String, Value>, class: &str) -> Result<()> {
    match class == CHECKLIST && map.contains_key("subject") {
        true => Err(bad(
            "a checklist gate judges the repository content in its scope: remove subject".into(),
        )),
        false => Ok(()),
    }
}
