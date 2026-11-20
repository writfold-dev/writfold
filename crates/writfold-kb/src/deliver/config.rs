//! Delivery configuration under the KB root's `.writfold/deliveries/`:
//! a mode file per mode ([`super::mode`]; built-in defaults otherwise), a deny
//! list per audience, and the guard hook's list of publishing commands.
//! Every file is the gate definitions' TOML subset ([`crate::defs::toml`]);
//! unknown keys are refused, so a misspelt field never silently changes a
//! check.
use crate::error::{KbError, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Where delivery configuration lives, under the KB root.
pub const DIR: &str = ".writfold/deliveries";
pub use super::mode::{MODES, Mode, mode};
pub const AUDIENCES: [&str; 2] = ["internal", "external"];
pub const PARAGRAPHS: &str = "paragraphs.bounded";
pub const REFERENCES: &str = "references.denied";
pub const CLAIMS: &str = "claims.cited-fresh";
/// `length.bounded` (`max_chars`, `max_lines`): never on by default.
pub const LENGTH: &str = "length.bounded";
/// The built-in checks, in the order they run and are shown.
pub const BUILTIN: [&str; 4] = [PARAGRAPHS, REFERENCES, CLAIMS, LENGTH];
/// The checks a built-in mode runs when no file names it.
pub const DEFAULTS: [&str; 3] = [PARAGRAPHS, REFERENCES, CLAIMS];

/// A delivery file's path and text; `None` when it does not exist.
pub(super) fn open(root: &Path, name: &str) -> Result<Option<(String, String)>> {
    let path = format!("{DIR}/{name}");
    let bytes = match std::fs::read(root.join(&path)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some((path, text))),
        Err(_) => {
            let p = crate::defs::Problem::whole(
                &path,
                &path,
                "file.unreadable",
                "is not UTF-8 text".into(),
            );
            Err(KbError::Definition(Box::new(p.report)))
        }
    }
}

pub(super) fn refused(report: writfold_core::diag::Report) -> KbError {
    KbError::Definition(Box::new(report))
}

/// An obligation name: lower-case letters, digits, `.` and `-`.
pub fn is_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        && !name.starts_with(['.', '-'])
}

/// One deny rule: a name and an ASCII-mode, case-insensitive pattern.
#[derive(Clone, Debug)]
pub struct Rule {
    pub name: String,
    pub pattern: String,
    pub regex: regex::bytes::Regex,
}

/// The deny list of one audience; `path` is `None` when it has none.
#[derive(Clone, Debug)]
pub struct Deny {
    pub audience: String,
    pub path: Option<String>,
    pub rules: Vec<Rule>,
}

/// `.writfold/deliveries/deny-<audience>.toml`: `rules = ["name: pattern",
/// …]`. A pattern is a regular expression in ASCII mode (`\b`, `\w` and
/// case folding are ASCII), matched case-insensitively, line by line.
pub fn deny(root: &Path, audience: &str) -> Result<Deny> {
    let Some((path, text)) = open(root, &format!("deny-{audience}.toml"))? else {
        return Ok(Deny {
            audience: audience.to_owned(),
            path: None,
            rules: Vec::new(),
        });
    };
    super::config_check::deny(audience, &path, &text).map_err(refused)
}

/// One obligation of a candidate, from its mode.
#[derive(Clone, Debug)]
pub struct Obligation {
    pub name: String,
    /// `mechanical`, `observed` (depends on the cited repositories) or
    /// `attested`.
    pub class: &'static str,
    /// The digest of what the obligation means; a receipt or statement
    /// about another digest is stale.
    pub digest: String,
    /// The candidate gate behind it, when it is one of the mode's gates.
    pub gate: Option<crate::gate::def::Definition>,
    /// Whether the mode takes a waiver of it (`waivable`); a mode gate's
    /// own `accept` decides for it instead.
    pub waivable: bool,
    /// The mode's `waivable` key and where it is, for a refusal.
    pub waivable_at: String,
}

impl Obligation {
    /// The standings that meet it: a mode gate's `accept`; otherwise every
    /// word, `waived` only when the mode takes a waiver of it.
    pub fn accept(&self) -> writfold_core::policy::Accept {
        match &self.gate {
            Some(g) => g.policy(),
            None => writfold_core::policy::Accept {
                satisfied: true,
                attested: true,
                waived: self.waivable,
            },
        }
    }
}

fn sha(text: &str) -> String {
    format!(
        "sha256:{}",
        crate::index::hex(&Sha256::digest(text.as_bytes()))
    )
}

/// The obligations of a mode for an audience, built-in checks first.
pub fn obligations(mode: &Mode, deny: &Deny) -> Vec<Obligation> {
    let base = "writfold-delivery-check/1\n";
    // A rendering other than verbatim joins every digest, so a receipt or
    // statement about the other payload stops standing.
    let rendered = match mode.rendering.as_str() {
        super::render::VERBATIM => String::new(),
        r => format!("\nrendering={r}"),
    };
    let mut out = Vec::new();
    for check in &mode.checks {
        let (class, material) = match check.as_str() {
            PARAGRAPHS => ("mechanical", format!("words={}", mode.paragraph_words)),
            REFERENCES => {
                let rules: Vec<String> = deny
                    .rules
                    .iter()
                    .map(|r| format!("{}\t{}", r.name, r.pattern))
                    .collect();
                let list = match deny.path {
                    Some(_) => rules.join("\n"),
                    None => "(no deny list)".into(),
                };
                ("mechanical", format!("audience={}\n{list}", deny.audience))
            }
            _ => ("observed", String::new()),
        };
        out.push(Obligation {
            name: check.clone(),
            class,
            digest: sha(&format!("{base}{check}\n{material}{rendered}")),
            gate: None,
            waivable: mode.waivable(check),
            waivable_at: mode.waivable_at(),
        });
    }
    // A gate's obligation digest is its definition digest, which covers
    // its command, its instructions' bytes and the scripts it ships.
    for gate in &mode.gates {
        out.push(Obligation {
            name: gate.id.clone(),
            class: match gate.class.as_str() {
                "mechanical" => "mechanical",
                "observed" => "observed",
                _ => "attested",
            },
            digest: sha(&format!(
                "{base}gate\n{}\n{}{rendered}",
                gate.id, gate.digest
            )),
            gate: Some(gate.clone()),
            waivable: true,
            waivable_at: String::new(),
        });
    }
    for name in &mode.attest {
        out.push(Obligation {
            name: name.clone(),
            class: "attested",
            digest: sha(&format!("{base}attested\n{name}{rendered}")),
            gate: None,
            waivable: mode.waivable(name),
            waivable_at: mode.waivable_at(),
        });
    }
    out
}

/// The guard hook's configuration: `.writfold/deliveries/guard.toml`.
#[derive(Clone, Debug)]
pub struct Guard {
    pub path: String,
    /// Publishing commands, as words (`gh pr comment`).
    pub commands: Vec<Vec<String>>,
    /// Tool names the harness calls directly (an MCP tool that posts).
    pub tools: Vec<String>,
    /// Posting commands and tools with their mode and body ([`super::route`]).
    pub routes: Vec<super::route::Route>,
}

/// The guard configuration, or `None` when there is none (the hook then
/// allows everything).
pub fn guard(root: &Path) -> Result<Option<Guard>> {
    let Some((path, text)) = open(root, "guard.toml")? else {
        return Ok(None);
    };
    super::config_check::guard(&path, &text, Some(root))
        .map(Some)
        .map_err(refused)
}

/// Check one delivery file's text (`kb lint`): `dir` is the KB root's
/// `.writfold/deliveries/`, `stem` the file name without `.toml`.
pub fn check_file(
    dir: &Path,
    stem: &str,
    shown: &str,
    text: &str,
) -> std::result::Result<(), writfold_core::diag::Report> {
    let root = dir.parent().and_then(Path::parent).unwrap_or(dir);
    match (stem, stem.strip_prefix("deny-")) {
        ("guard", _) => super::config_check::guard(shown, text, Some(root)).map(|_| ()),
        (_, Some(audience)) => super::config_check::deny(audience, shown, text).map(|_| ()),
        _ => {
            let defaults = super::mode::default(stem);
            super::mode_check::mode(root, defaults, shown, text).map(|_| ())
        }
    }
}
