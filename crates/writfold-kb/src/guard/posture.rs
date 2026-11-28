//! `.writfold/guards.toml` (`writfold-guard-site/1`, #184): the posture an
//! undecided event gets, and the rules that must load.
//!
//! ```toml
//! schema = "writfold-guard-site/1"
//! posture = "deny"          # open (default) | ask | deny
//! required = ["commit"]     # rule ids that must load
//! ```
//!
//! The effective posture is the stricter of this file's and the
//! registration's (`--posture`, [`writfold_core::site::stricter`]): a KB
//! that cannot be found cannot say what its posture is.
use crate::defs::{Check, Field};
use crate::gate::def_check::schema;
use std::path::Path;
use writfold_core::diag::Report;
use writfold_core::site::Posture;

pub const SCHEMA: &str = "writfold-guard-site/1";
pub const FILE: &str = ".writfold/guards.toml";
const KEYS: [&str; 3] = ["schema", "posture", "required"];
const POSTURES: [&str; 3] = ["open", "ask", "deny"];

/// The site file's declarations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Site {
    pub posture: Posture,
    pub required: Vec<String>,
}

/// Parse and check the site file.
pub fn check(text: &str, shown: &str) -> Result<Site, Report> {
    let mut c = Check::read(shown, text, "the guard site file");
    c.unknown_keys(&KEYS, &KEYS, "not a guard site key");
    schema(&mut c, SCHEMA);
    let posture = match c.text("posture") {
        Field::Absent => Some(Posture::Open),
        Field::Bad => None,
        Field::Got(t) => match Posture::parse(t.str()) {
            Some(p) => Some(p),
            None => {
                let why = format!("unknown posture {:?}", t.str());
                c.unknown_value(t.span, why, "not a posture", t.str(), &POSTURES, '"');
                None
            }
        },
    };
    let mut required: Vec<String> = Vec::new();
    let mut ok = true;
    let listed = c
        .list("required")
        .got()
        .map(|l| l.value)
        .unwrap_or_default();
    for id in listed {
        if required.iter().any(|r| r == id.str()) {
            let why = format!("`required` names {} twice", id.str());
            c.error("value.twice", Some(id.span), why)
                .label("named again");
            ok = false;
        } else {
            required.push(id.str().to_owned());
        }
    }
    let site = posture
        .filter(|_| ok)
        .map(|posture| Site { posture, required });
    c.finish(site)
}

/// The KB's site file: `Ok(None)` when there is none, `Err` with every
/// fault when it does not read or parse.
pub fn read(kb: &Path) -> Result<Option<Site>, String> {
    let path = kb.join(FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{FILE}: cannot read: {e}")),
    };
    check(&text, FILE)
        .map(Some)
        .map_err(|r| crate::defs::why(&r))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_site_file_reads_and_its_faults_are_named() {
        let ok = check(
            "schema = \"writfold-guard-site/1\"\nposture = \"deny\"\nrequired = [\"commit\"]\n",
            FILE,
        );
        assert_eq!(
            ok.ok(),
            Some(Site {
                posture: Posture::Deny,
                required: vec!["commit".into()]
            })
        );
        let bare = check("schema = \"writfold-guard-site/1\"\n", FILE);
        assert_eq!(bare.ok().map(|s| s.posture), Some(Posture::Open));
        let bad = check(
            "schema = \"writfold-guard-site/1\"\nposture = \"closed\"\nrequired = [\"a\", \"a\"]\n",
            FILE,
        );
        let why = bad.err().map(|r| crate::defs::why(&r)).unwrap_or_default();
        assert!(why.contains("unknown posture \"closed\""), "{why}");
    }
}
