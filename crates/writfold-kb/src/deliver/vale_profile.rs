//! `prose.vale` profiles (#132, RFC 2026-11-25-vale-for-documents): the
//! pin `.writfold/deliveries/vale-<name>.toml` and the mode keys that name
//! it.
//!
//! ```toml
//! schema = "writfold-vale-profile/1"
//! name = "house"
//! version = "3.28.1"
//! canary = "This draft is TODO."
//! ```
//!
//! `version` is the exact Vale release the profile was written for (3.20.0
//! or later, for `--counts`); `canary` is one line that must draw at least
//! one alert, so a config that matches nothing is never a pass. The user's
//! own `.vale.ini` and styles live unchanged in `.writfold/vale/<name>/`
//! ([`super::vale_tree`]), and every file there is bound into the
//! obligation digest. Reading is tested; the decision is
//! [`writfold_core::vale`].
use super::config_check::named;
use crate::defs::fields::Field;
use crate::defs::{Check, Spanned, Text};
use crate::gate::def_check::schema;
use writfold_core::diag::Report;
use writfold_core::vale::BlockFrom;

pub const SCHEMA: &str = "writfold-vale-profile/1";
/// The oldest Vale with `--counts`.
const OLDEST: (u64, u64, u64) = (3, 20, 0);
/// `vale_block`'s values, strictest last.
pub const BLOCKS: [&str; 4] = ["never", "error", "warning", "suggestion"];

/// One profile with its bound tree.
#[derive(Clone, Debug)]
pub struct Profile {
    pub name: String,
    /// The KB path of `vale-<name>.toml`.
    pub path: String,
    pub version: String,
    pub canary: String,
    pub timeout_seconds: u64,
    /// Every file under `.writfold/vale/<name>/`: path relative to it and
    /// sha256, sorted by path.
    pub inventory: Vec<(String, String)>,
}

/// A mode's Vale settings: the profile and the lowest severity that blocks.
#[derive(Clone, Debug)]
pub struct Vale {
    pub profile: Profile,
    pub block: BlockFrom,
}

impl Profile {
    /// The tree's KB path, `.writfold/vale/<name>`.
    pub fn tree(&self) -> String {
        format!("{}/{}", super::vale_tree::DIR, self.name)
    }
}

impl Vale {
    /// What the check means, for the obligation digest. The timeout is
    /// left out: it changes when a run gives up, not what it says.
    pub fn material(&self) -> String {
        let p = &self.profile;
        let files: Vec<String> = (p.inventory.iter())
            .map(|(path, sha)| format!("{sha}\t{path}"))
            .collect();
        format!(
            "profile={}\nversion={}\nblock={}\ncanary={}\n{}",
            p.name,
            p.version,
            block_name(self.block),
            p.canary,
            files.join("\n")
        )
    }
}

pub fn block_name(b: BlockFrom) -> &'static str {
    match b {
        BlockFrom::Never => "never",
        BlockFrom::Error => "error",
        BlockFrom::Warning => "warning",
        BlockFrom::Suggestion => "suggestion",
    }
}

fn block_from(name: &str) -> BlockFrom {
    match name {
        "never" => BlockFrom::Never,
        "warning" => BlockFrom::Warning,
        "suggestion" => BlockFrom::Suggestion,
        _ => BlockFrom::Error,
    }
}

/// `X.Y.Z` as numbers.
pub fn version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text
        .split('.')
        .map(|p| match p.bytes().all(|b| b.is_ascii_digit()) {
            true => p.parse::<u64>().ok(),
            false => None,
        });
    let v = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(v)
}

/// `vale-<name>.toml`, without its tree (the tree is read by
/// [`super::vale_tree::inventory`]).
pub fn check(name: &str, path: &str, text: &str) -> Result<Profile, Report> {
    let mut c = Check::read(path, text, "a Vale profile");
    let keys = ["schema", "name", "version", "canary", "timeout_seconds"];
    c.unknown_keys(&keys, &keys, "not a Vale profile key");
    schema(&mut c, SCHEMA);
    let ok = named(&mut c, "name", name, &format!("vale-{name}.toml"));
    let pin = c.need_text("version", "version = \"3.28.1\"");
    let pin = pin.filter(|v| match version(v.str()) {
        Some(v) if v >= OLDEST => true,
        _ => {
            let why = format!(
                "version {:?} is not an exact Vale release, 3.20.0 or later",
                v.str()
            );
            c.error("value.form", Some(v.span), why).label("X.Y.Z");
            false
        }
    });
    let canary = c.need_text("canary", "canary = \"<one line that draws an alert>\"");
    let canary = canary.filter(|v| one_line(&mut c, v));
    let timeout = c.int("timeout_seconds", (1, 3600), 60).or(Spanned {
        value: 60,
        span: Default::default(),
    });
    let built = (|| {
        Some(Profile {
            name: name.to_owned(),
            path: path.to_owned(),
            version: pin?.value.text,
            canary: canary?.value.text,
            timeout_seconds: u64::try_from(timeout?.value).ok()?,
            inventory: Vec::new(),
        })
    })();
    c.finish(built.filter(|_| ok))
}

fn one_line(c: &mut Check<'_>, v: &Spanned<Text>) -> bool {
    if !v.str().trim().is_empty() && !v.str().contains(['\n', '\r']) {
        return true;
    }
    let why = "canary must be one non-empty line".to_owned();
    c.error("value.form", Some(v.span), why).label("one line");
    false
}

/// The mode keys `vale` and `vale_block`. `vale` is required when `checks`
/// lists `prose.vale` and refused otherwise; so is `vale_block`, which is
/// `error` when absent. `Some(None)` when absent and not needed, `None` on
/// a fault.
pub(super) fn key(c: &mut Check<'_>, root: &std::path::Path, listed: bool) -> Option<Option<Vale>> {
    let block = super::mode_keys::closed(c, "vale_block", &BLOCKS);
    let name = match (c.text("vale"), listed) {
        (Field::Bad, _) => return None,
        (Field::Absent, false) => {
            return match block? {
                None => Some(None),
                Some(_) => unchecked(c, "vale_block"),
            };
        }
        (Field::Absent, true) => {
            let why = "prose.vale needs vale = \"<profile>\", naming .writfold/deliveries/vale-<profile>.toml";
            c.error("key.missing", None, why.into()).label("no profile");
            return None;
        }
        (Field::Got(_), false) => return unchecked(c, "vale"),
        (Field::Got(v), true) => v,
    };
    match load(root, name.str()) {
        Ok(profile) => Some(Some(Vale {
            profile,
            block: block?.as_deref().map_or(BlockFrom::Error, block_from),
        })),
        Err(why) => {
            c.error("ref.undefined", Some(name.span), why)
                .label("no usable profile");
            None
        }
    }
}

fn unchecked(c: &mut Check<'_>, key: &str) -> Option<Option<Vale>> {
    let why = format!("`{key}` is set, but checks does not list prose.vale");
    let span = c.key_span(key);
    c.error("value.conflict", span, why).label("not checked");
    None
}

/// The profile `name` and its tree, or why it cannot be used.
pub fn load(root: &std::path::Path, name: &str) -> Result<Profile, String> {
    if !super::config::is_name(name) {
        return Err(format!("profile {name:?} must be [a-z0-9.-]"));
    }
    let file = format!("vale-{name}.toml");
    let (path, text) = match super::config::open(root, &file) {
        Ok(Some(found)) => found,
        Ok(None) => {
            return Err(format!(
                "no Vale profile: add {}/{file}",
                super::config::DIR
            ));
        }
        Err(e) => return Err(format!("{file} cannot be read: {e}")),
    };
    let mut profile = check(name, &path, &text).map_err(|r| {
        let first = r.diagnostics.first().map(|d| d.message.clone());
        format!("{file} has errors: {}", first.unwrap_or_default())
    })?;
    profile.inventory = super::vale_tree::inventory(root, name)?;
    Ok(profile)
}
