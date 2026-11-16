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
use crate::defs::Check;
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

/// The assessed fields of a definition of class `class`; `Some(None)`
/// for any other class, which may not use them. `dir` holds the
/// definition. Every fault is reported to `c`; `None` when there was one.
pub fn check(c: &mut Check<'_>, class: &str, dir: &Path) -> Option<Option<Spec>> {
    if class != super::super::def::ASSESSED {
        let mut ok = true;
        for key in KEYS {
            let why = format!("`{key}` is for gates of class assessed");
            ok &= !c.not_here(key, why, "only an assessed gate takes it");
        }
        return ok.then_some(None);
    }
    let needs = "an assessed gate";
    let weft = c.need_text_for("weft", needs, "weft = \"{gate}/step.weft\"");
    let source = weft.as_ref().and_then(|w| match source(w.str(), dir) {
        Ok(source) => Some(source),
        Err(why) => {
            c.error("value.form", Some(w.span), why)
                .label("not a Weft source");
            None
        }
    });
    let checks = c.need_list("checks", needs, "checks = [\"ready\"]");
    let checks_ok = checks.as_ref().is_some_and(|checks| {
        let mut ok = true;
        for (n, check) in checks.iter().enumerate() {
            if !identifier(check.str()) {
                let why = format!("check {:?} is not a Weft identifier", check.str());
                c.error("value.form", Some(check.span), why)
                    .label("not an identifier");
                ok = false;
            } else if checks[..n].iter().any(|x| x.str() == check.str()) {
                let why = format!("check {} is listed twice", check.str());
                c.error("value.twice", Some(check.span), why)
                    .label("listed again");
                ok = false;
            }
        }
        ok
    });
    let add = "weft_sha256 = \"sha256:<64 hex digits>\"";
    let sha = c.need_text_for("weft_sha256", needs, add);
    let pin = sha.as_ref().and_then(|s| match pin(s.str()) {
        Ok(pin) => Some(pin),
        Err(why) => {
            c.error("value.form", Some(s.span), why)
                .label("not a SHA-256 digest");
            None
        }
    });
    let checks = checks.filter(|_| checks_ok)?;
    Some(Some(Spec {
        source: source?,
        weft: weft?.value.text,
        checks: checks.into_iter().map(|c| c.value.text).collect(),
        pin: pin?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec_as(text: &str, class: &str, dir: &Path) -> Result<Option<Spec>, String> {
        let mut c = Check::read("f.toml", text, "a gate definition");
        let got = check(&mut c, class, dir);
        c.finish(got).map_err(|r| crate::defs::why(&r))
    }

    fn spec(text: &str, dir: &Path) -> Result<Option<Spec>, String> {
        spec_as(text, "assessed", dir)
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
        assert!(spec_as("weft = \"{gate}/step.weft\"\n", "mechanical", &dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}
