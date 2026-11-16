//! The `deliver:<mode>` word of a plan step: its form, read when a
//! template is parsed, and the mode it names, checked when a template is
//! spliced (`plan set`, `plan use`, `use=` closures, required templates).
//! A gate ID is a slug and cannot hold `:`, so the word never reads as a
//! gate, and a template written for an older binary keeps its meaning.
use super::config::{self, MODES, is_name};
use super::step::PREFIX;
use crate::error::{KbError, Result};
use std::path::Path;

/// Whether `mode` may be named by a step: a mode name, as `deliver
/// prepare --mode` takes it.
pub fn is_mode(mode: &str) -> bool {
    is_name(mode) && mode != "guard" && !mode.starts_with("deny-")
}

/// Check one step word that starts `deliver:`, against the words the step
/// already owes; the refusal names the step `id`.
pub fn check(id: &str, word: &str, owed: &[String]) -> std::result::Result<(), String> {
    let mode = word.strip_prefix(PREFIX).unwrap_or_default();
    if !is_mode(mode) {
        return Err(format!(
            "step {id}: {word} does not name a delivery mode ([a-z0-9.-], not guard or deny-*)"
        ));
    }
    if owed.iter().any(|w| w == word) {
        return Err(format!("step {id} names {word} twice"));
    }
    Ok(())
}

/// Refuse a delivery that no mode of the KB root can meet: no such mode,
/// a mode file that does not load, or a mode that declares nothing.
pub fn spliced(root: &Path, template: &str, mode: &str) -> Result<()> {
    let file = format!("{}/{mode}.toml", config::DIR);
    let exists = MODES.contains(&mode) || root.join(&file).is_file();
    if !exists {
        return Err(KbError::Refused(format!(
            "plan {template} names {PREFIX}{mode}, which is not a delivery mode: add {file} to the KB root, or use a built-in mode: {}",
            MODES.join(", ")
        )));
    }
    let m = config::mode(root, mode).map_err(|e| {
        let why = match e {
            KbError::Definition(report) => return KbError::Definition(report),
            KbError::Refused(why) | KbError::Usage(why) => why,
            other => other.to_string(),
        };
        KbError::Refused(format!("{PREFIX}{mode}: {why}"))
    })?;
    if m.checks.is_empty() && m.gates.is_empty() && m.attest.is_empty() {
        return Err(KbError::Refused(format!(
            "plan {template} names {PREFIX}{mode}, whose mode declares nothing to check; a delivery is not met by an empty set: add checks, gates or attest to {file}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::plan::template::step;

    #[test]
    fn step_words_name_a_mode_once() -> Result<(), String> {
        let s = step("d: deliver:pr-comment unit use=r")?;
        assert_eq!(s.gates, ["deliver:pr-comment", "unit"]);
        assert_eq!(s.include.as_deref(), Some("r"));
        let s = step("d: unit deliver:note deliver:pr-comment")?;
        assert_eq!(s.gates, ["unit", "deliver:note", "deliver:pr-comment"]);
        for (bad, says) in [
            ("d: deliver:", "does not name a delivery mode"),
            ("d: deliver:Bad", "does not name a delivery mode"),
            ("d: deliver:guard", "does not name a delivery mode"),
            ("d: deliver:deny-external", "does not name a delivery mode"),
            ("d: deliver:a:b", "does not name a delivery mode"),
            ("d: deliver:note deliver:note", "names deliver:note twice"),
        ] {
            let why = step(bad).err().unwrap_or_default();
            assert!(why.contains(says), "{bad}: {why}");
        }
        Ok(())
    }
}
