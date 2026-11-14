//! Validate the complete recorded template closure before expansion.
use super::super::event::Splice;
use super::super::template::{self, Template};
use super::Snapshot;

/// A snapshot's template, checked against its digest and name.
fn snapshot_of(snapshot: Snapshot<'_>, name: &str, digest: &str) -> Result<Template, String> {
    match snapshot(digest) {
        Some(bytes) if template::digest(&bytes) == digest => {
            template::parse(&bytes, Some(name)).map_err(|why| format!("its snapshot: {why}"))
        }
        _ => Err(format!(
            "the template snapshot of {name} is missing or altered"
        )),
    }
}

/// A splice's template, with every template it includes added to
/// `library`; every `use=` in the closure must be recorded in it.
pub(super) fn load(
    s: &Splice,
    snapshot: Snapshot<'_>,
    library: &mut Vec<Template>,
) -> Result<Template, String> {
    let top = snapshot_of(snapshot, &s.template, &s.digest)?;
    let mut closure = vec![top.clone()];
    for include in &s.includes {
        closure.push(snapshot_of(snapshot, &include.name, &include.digest)?);
    }
    for t in &closure {
        for name in t.steps.iter().filter_map(|step| step.include.as_deref()) {
            if !s.includes.iter().any(|i| i.name == name) {
                return Err(format!(
                    "{} includes {name}, which its closure does not record",
                    t.name
                ));
            }
        }
    }
    for t in closure.into_iter().skip(1) {
        if !library.iter().any(|l| l.digest == t.digest) {
            library.push(t);
        }
    }
    Ok(top)
}
