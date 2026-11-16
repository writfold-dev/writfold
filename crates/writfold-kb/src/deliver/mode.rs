//! Delivery modes. Any `.writfold/deliveries/<name>.toml` in the KB root
//! with `schema = "writfold-delivery-mode/1"` defines the mode `<name>`;
//! `pr-comment` and `slack-summary` are built in and used when no file
//! names them. A mode lists its obligations and nothing more:
//!
//! - `checks`: built-in checks (all three by default for a built-in mode,
//!   none for a user mode);
//! - `gates`: candidate gates from the KB root's `.writfold/gates/`
//!   (`subject = "candidate"`): scripts run against the candidate through
//!   `{candidate}`, and attested gates whose instructions are shown;
//! - `attest`: plain attested names, as before.
//!
//! Everything is checked when the mode is loaded: unknown keys, unknown or
//! malformed gates, a repository gate, a mode named where a gate belongs
//! and a name listed twice are refused with the file and the reason. A
//! mode cannot name another mode and a gate cannot name a gate, so no
//! cycle can form. Loading a mode never runs anything.
use super::config::{BUILTIN, DIR, is_name, open, refused};
use crate::error::{KbError, Result};
use crate::gate::def::Definition;
use std::path::Path;

/// The modes that exist without a file.
pub const MODES: [&str; 2] = ["pr-comment", "slack-summary"];
/// One mode: which checks apply, the paragraph bound, its gates and its
/// attested obligations (statements a person or agent must make).
#[derive(Clone, Debug)]
pub struct Mode {
    pub name: String,
    pub checks: Vec<String>,
    pub paragraph_words: usize,
    /// Candidate gates, in the order listed.
    pub gates: Vec<Definition>,
    pub attest: Vec<String>,
    /// One line the mode file says about itself (shown with its
    /// candidates), such as what its checks cannot see.
    pub description: Option<String>,
    /// `built-in`, or the mode file's KB path.
    pub source: String,
}

/// The default of a mode: for a built-in mode every check, 150 words per
/// paragraph for a PR comment and 60 for a Slack summary; for a user mode
/// no check and 150 words.
pub(super) fn default(name: &str) -> Mode {
    let builtin = MODES.contains(&name);
    Mode {
        name: name.to_owned(),
        checks: match builtin {
            true => BUILTIN.iter().map(|c| (*c).to_owned()).collect(),
            false => Vec::new(),
        },
        paragraph_words: if name == "slack-summary" { 60 } else { 150 },
        gates: Vec::new(),
        attest: Vec::new(),
        description: None,
        source: "built-in".into(),
    }
}

/// The mode `name`: `.writfold/deliveries/<name>.toml` when present, else
/// the built-in default; any other name is refused, naming the file to add.
pub fn mode(root: &Path, name: &str) -> Result<Mode> {
    if !is_name(name) || name == "guard" || name.starts_with("deny-") {
        return Err(KbError::Usage(format!(
            "--mode {name:?} is not a mode name ([a-z0-9.-], not guard or deny-*)"
        )));
    }
    let file = format!("{name}.toml");
    let Some((path, text)) = open(root, &file)? else {
        return match MODES.contains(&name) {
            true => Ok(default(name)),
            false => Err(KbError::Refused(format!(
                "no delivery mode {name}: add {DIR}/{file} (schema = \"writfold-delivery-mode/1\", mode = \"{name}\"), or use a built-in mode: {}",
                MODES.join(", ")
            ))),
        };
    };
    super::mode_check::mode(root, default(name), &path, &text).map_err(refused)
}
