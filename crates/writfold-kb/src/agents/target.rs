//! What `agent prepare --out` and `--for` name (#167), classified for
//! [`writfold_core::invoke::prepare_check`]: whether `--out` resolves
//! outside the KB, and whether `--for GATE.ITEM` names an item the
//! checklist has now (a generated checklist's units as made from its
//! scope's manifest now). Tested IO; the decision is the kernel's.
use crate::gate::checklist::generated;
use crate::gate::status::{self, Filings};
use crate::plan::Loaded;
use crate::session::Session;
use std::path::{Component, Path, PathBuf};
use writfold_core::invoke::ForItem;

/// `path` with `..` and symbolic links resolved as far as it exists, the
/// rest appended as written.
fn resolved(path: &Path) -> PathBuf {
    let mut missing: Vec<&std::ffi::OsStr> = Vec::new();
    let mut at = path;
    loop {
        if let Ok(real) = std::fs::canonicalize(at) {
            let mut out = real;
            for part in missing.iter().rev() {
                match Path::new(part).components().next() {
                    Some(Component::ParentDir) => {
                        out.pop();
                    }
                    Some(Component::CurDir) => {}
                    _ => out.push(part),
                }
            }
            return out;
        }
        match (at.parent(), at.file_name()) {
            (Some(up), Some(name)) => {
                missing.push(name);
                at = up;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// `path` with `.` and `..` taken lexically.
fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// Whether `--out` (relative to `cwd`) resolves outside the KB root.
pub fn out_outside(kb: &Path, cwd: &Path, out: &Path) -> bool {
    let root = std::fs::canonicalize(kb).unwrap_or_else(|_| kb.to_path_buf());
    let joined = cwd.join(out);
    let inside = |p: &Path| p.starts_with(&root);
    !(inside(&resolved(&joined))
        || inside(&resolved(&lexical(&joined)))
        || inside(&lexical(&joined)))
}

/// What `--for` names, and why when it is not an item the checklist has
/// now.
pub fn for_item(session: &Session<'_>, loaded: &Loaded, text: Option<&str>) -> (ForItem, String) {
    let Some(text) = text else {
        return (ForItem::Absent, String::new());
    };
    match known(session, loaded, text) {
        Ok(()) => (ForItem::Known, String::new()),
        Err(why) => (ForItem::Unknown, why),
    }
}

fn known(session: &Session<'_>, loaded: &Loaded, text: &str) -> Result<(), String> {
    let (gate, item) = super::invocation::for_item(text).ok_or(
        "write GATE.ITEM, a checklist gate and one of its items; next: writfold kb gate status",
    )?;
    let next = format!("next: writfold kb gate status {gate}");
    let subject = (loaded.subject.as_ref()).ok_or(format!("no subject repository; {next}"))?;
    let d = crate::gate::def::find_repository(&session.ws.root, Some(&subject.top), &gate)
        .map_err(|e| format!("{e}; next: writfold kb gate list"))?;
    if d.checklist.is_none() {
        return Err(format!(
            "{gate} is a {} gate, not a checklist; {next}",
            d.class
        ));
    }
    let mut filings = Filings::for_run(session, &loaded.rec);
    filings.owed.gates = None;
    let (plain, _) = status::judge_plain(session, subject, &filings).map_err(|e| e.to_string())?;
    let (d, universe) = generated::expand(&session.ws.root, &plain, d);
    let has = d
        .checklist
        .as_ref()
        .is_some_and(|s| s.item(&item).is_some());
    match (has, universe.and_then(|u| u.why)) {
        (true, _) => Ok(()),
        (false, Some(why)) => Err(format!("{gate} has no units now: {why}; {next}")),
        (false, None) => Err(format!("{gate} has no such item now; {next}")),
    }
}
