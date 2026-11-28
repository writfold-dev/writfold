//! `kb lint` of `.writfold/guards.toml` (#184): the file's own faults,
//! then each `required` rule: one not defined in the KB root's
//! `.writfold/guards/` is `ref.undefined` (the site would leave every
//! event undecided), and one in audit mode is warned
//! (`policy.required-audit`: it never decides a call).
use super::lint::{File, Kind};
use super::{Check, Problem};
use crate::gate::def::Origin;
use crate::guard::posture::{self, FILE};
use crate::guard::rules;
use std::path::Path;
use writfold_core::diag::{Report, Severity};
use writfold_core::guard::Mode;

fn report(kb: &Path, text: &str) -> Report {
    if let Err(report) = posture::check(text, FILE) {
        return report;
    }
    let mut c = Check::read(FILE, text, "the guard site file");
    let required = c
        .list("required")
        .got()
        .map(|l| l.value)
        .unwrap_or_default();
    let (found, _) = rules::read(rules::root_dir(kb).as_slice());
    for id in required {
        match found.iter().find(|r| r.id == id.str()) {
            None => {
                let why = format!(
                    "required rule {} is not defined in {}",
                    id.str(),
                    rules::DIR
                );
                c.error("ref.undefined", Some(id.span), why).help(
                    "define it, or take it out of `required`: until then every event is undecided"
                        .to_owned(),
                );
            }
            Some(rule) if rule.mode == Mode::Audit => {
                let why = format!(
                    "required rule {} is in audit mode: it never decides a call",
                    id.str()
                );
                c.sink.push(
                    Severity::Warning,
                    "policy.required-audit",
                    Some(id.span),
                    why,
                );
            }
            Some(_) => {}
        }
    }
    c.sink.report()
}

/// The KB root's site file, when it exists.
pub fn file(kb: &Path) -> Option<File> {
    let path = kb.join(FILE);
    if !path.is_file() {
        return None;
    }
    let (text, report) = match super::read_text(&path, FILE) {
        Ok(text) => {
            let report = report(kb, &text);
            (text, report)
        }
        Err(Problem { report, .. }) => (String::new(), report),
    };
    Some(File {
        kind: Kind::Site,
        origin: Origin::Kb,
        dir: kb.join(".writfold"),
        stem: "guards".into(),
        shown: FILE.into(),
        text,
        report,
    })
}
