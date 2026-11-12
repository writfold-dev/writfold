//! The keys of an `assessed` gate definition:
//!
//! ```toml
//! weft = "{gate}/step.weft"          # or "{run}/<path>.weft", submitted in the run
//! checks = ["rollback_ready"]        # the Weft checks that must be met
//! weft_sha256 = "sha256:<64 hex>"    # the pinned Weft binary
//! timeout_seconds = 60
//! ```
//!
//! The binary's path is never part of a definition (see [`super::weft`]):
//! the definition pins its digest only.
use super::super::places;
use super::super::toml::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The keys only an assessed gate may use.
pub const KEYS: [&str; 3] = ["weft", "checks", "weft_sha256"];
/// The placeholder for the run's own directory in `weft`.
pub const RUN: &str = "{run}";

/// Where the Weft source is read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Source {
    /// Shipped beside the definition: `{gate}/<path>`.
    Gate(String),
    /// Submitted by the agent in the run: `{run}/<path>`.
    Run(String),
}

impl Source {
    /// The file, given the definition's directory and the run's directory.
    pub fn file(&self, gate: &Path, run: &Path) -> Result<PathBuf, String> {
        match self {
            Source::Gate(rel) => places::resolve(gate, rel),
            Source::Run(rel) => places::resolve(run, rel)
                .map_err(|why| format!("{why} (submit the Weft file at {RUN}/{rel})")),
        }
    }
}

/// An assessed gate's own fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spec {
    /// As written in the definition.
    pub weft: String,
    pub source: Source,
    pub checks: Vec<String>,
    /// `sha256:<hex>` of the Weft binary.
    pub pin: String,
}

impl Spec {
    /// The lines the definition digest adds for these fields.
    pub fn canonical(&self) -> String {
        format!(
            "weft={}\nchecks={}\nweft_sha256={}\n",
            self.weft,
            serde_json::to_string(&self.checks).unwrap_or_default(),
            self.pin
        )
    }
}

fn identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn source(weft: &str, dir: &Path) -> Result<Source, String> {
    let (placeholder, rel) = weft
        .split_once('/')
        .ok_or_else(|| format!("weft {weft:?} must be {{gate}}/<path> or {RUN}/<path>"))?;
    if !rel.ends_with(".weft") || !places::contained(rel) || rel.contains('{') {
        return Err(format!(
            "weft {weft:?} must name a .weft file inside its directory"
        ));
    }
    match placeholder {
        places::GATE => {
            places::resolve(dir, rel).map_err(|why| format!("weft: {why}"))?;
            Ok(Source::Gate(rel.to_owned()))
        }
        RUN => Ok(Source::Run(rel.to_owned())),
        _ => Err(format!("weft {weft:?} must start with {{gate}}/ or {RUN}/")),
    }
}

fn pin(text: &str) -> Result<String, String> {
    let hex = text.strip_prefix("sha256:").unwrap_or(text);
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("weft_sha256 must be sha256:<64 hex digits>".into());
    }
    Ok(format!("sha256:{}", hex.to_ascii_lowercase()))
}

/// The assessed fields of a definition of class `class`, read from `map`;
/// `None` for any other class, which may not use them. `dir` holds the
/// definition.
pub fn parse(
    map: &BTreeMap<String, Value>,
    class: &str,
    dir: &Path,
) -> Result<Option<Spec>, String> {
    if class != super::super::def::ASSESSED {
        return match KEYS.iter().find(|k| map.contains_key(**k)) {
            Some(key) => Err(format!("{key} is for gates of class assessed")),
            None => Ok(None),
        };
    }
    let text = |key: &str| match map.get(key) {
        Some(Value::Str(s)) => Ok(s.clone()),
        _ => Err(format!("an assessed gate needs {key} as a string")),
    };
    let weft = text("weft")?;
    let checks = match map.get("checks") {
        Some(Value::List(items)) if !items.is_empty() => items.clone(),
        _ => return Err("checks must be a non-empty array of Weft check names".into()),
    };
    if let Some(bad) = checks.iter().find(|c| !identifier(c)) {
        return Err(format!("check {bad:?} is not a Weft identifier"));
    }
    if (1..checks.len()).any(|i| checks[..i].contains(&checks[i])) {
        return Err("a check is listed twice".into());
    }
    Ok(Some(Spec {
        source: source(&weft, dir)?,
        weft,
        checks,
        pin: pin(&text("weft_sha256")?)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::toml;

    fn spec(text: &str, dir: &Path) -> Result<Option<Spec>, String> {
        parse(&toml::parse(text)?, "assessed", dir)
    }

    #[test]
    fn sources_checks_and_pins_are_checked() -> Result<(), String> {
        let dir = std::env::temp_dir().join(format!("wf-assessed-spec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("step.weft"), "review x {}\n").map_err(|e| e.to_string())?;
        let pin = "a".repeat(64);
        let ok = spec(
            &format!("weft = \"{{gate}}/step.weft\"\nchecks = [\"a\", \"b_2\"]\nweft_sha256 = \"{pin}\"\n"),
            &dir,
        )?
        .ok_or("no spec")?;
        assert_eq!(ok.source, Source::Gate("step.weft".into()));
        assert_eq!(ok.pin, format!("sha256:{pin}"));
        let run = spec(
            &format!("weft = \"{{run}}/plan/step.weft\"\nchecks = [\"a\"]\nweft_sha256 = \"sha256:{pin}\"\n"),
            &dir,
        )?
        .ok_or("no spec")?;
        assert_eq!(run.source, Source::Run("plan/step.weft".into()));
        for (weft, checks, sha) in [
            ("{gate}/missing.weft", "[\"a\"]", pin.as_str()),
            ("{gate}/../step.weft", "[\"a\"]", pin.as_str()),
            ("{run}/step.txt", "[\"a\"]", pin.as_str()),
            ("/abs/step.weft", "[\"a\"]", pin.as_str()),
            ("{gate}/step.weft", "[\"a\", \"a\"]", pin.as_str()),
            ("{gate}/step.weft", "[\"1a\"]", pin.as_str()),
            ("{gate}/step.weft", "[\"a\"]", "abc"),
        ] {
            let text = format!("weft = \"{weft}\"\nchecks = {checks}\nweft_sha256 = \"{sha}\"\n");
            assert!(spec(&text, &dir).is_err(), "{text}");
        }
        let plain = toml::parse("weft = \"{gate}/step.weft\"\n")?;
        assert!(parse(&plain, "mechanical", &dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}
