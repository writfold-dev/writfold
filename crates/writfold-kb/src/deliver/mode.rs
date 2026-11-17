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
use super::config::{BUILTIN, is_name, open, refused};
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
    /// `waivable`: the built-in checks and attest names the mode takes a
    /// waiver of, with the key's line; `None` (absent) takes every waiver.
    pub waivable: Option<(Vec<String>, u32)>,
    /// `rendering`: how the payload is made from the candidate
    /// ([`super::render`]); `verbatim` when absent.
    pub rendering: String,
    /// `audience`: the audience the mode is for, with the key's line;
    /// `None` leaves it to `deliver prepare --audience`.
    pub audience: Option<(String, u32)>,
}

impl Mode {
    /// Whether the mode takes a waiver of obligation `name` (a mode gate's
    /// own `accept` decides for it instead).
    pub fn waivable(&self, name: &str) -> bool {
        self.waivable
            .as_ref()
            .is_none_or(|(names, _)| names.iter().any(|n| n == name))
    }

    /// `waivable = ["claims.cited-fresh"] @.writfold/deliveries/pr-comment.toml:6`.
    pub fn waivable_at(&self) -> String {
        let (names, line) = self.waivable.clone().unwrap_or_default();
        let list = serde_json::to_string(&names).unwrap_or_default();
        format!(
            "waivable = {} @{}:{line}",
            list.replace(',', ", "),
            self.source
        )
    }
}

/// The line of `key = …` in a file's text (1 when it cannot be found).
pub fn key_line(text: &str, key: &str) -> u32 {
    let at = text.lines().position(|l| {
        l.trim_start()
            .strip_prefix(key)
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    });
    at.map_or(1, |n| u32::try_from(n + 1).unwrap_or(1))
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
        waivable: None,
        rendering: super::render::VERBATIM.into(),
        audience: None,
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
            false => Err(super::modes::unknown(root, name)),
        };
    };
    super::mode_check::mode(root, default(name), &path, &text).map_err(refused)
}
