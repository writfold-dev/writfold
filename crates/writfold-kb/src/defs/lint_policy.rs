//! The policy warning `kb lint` gives (policy spec 2.3): a checklist that
//! restricts `not_applicable` while its gate still accepts a waiver. The
//! restriction is then one `gate waive` from being moot. A warning, not an
//! error: a team may want exactly that.
use super::lint::{File, Kind};
use super::{Sink, Slot, toml};
use writfold_core::diag::{Report, Severity};

/// The warning for one file, if it applies.
fn waivable(f: &File) -> Option<Report> {
    let d = crate::gate::def_check::check(&f.text, &f.stem, f.origin, &f.dir, &f.shown).ok()?;
    let spec = d.checklist.as_ref()?;
    spec.not_applicable.as_ref()?;
    if !d.policy().waived {
        return None;
    }
    let mut sink = Sink::new(&f.shown, &f.text);
    let doc = toml::read(&f.text, &mut Sink::new("", &f.text));
    let span = doc.get("not_applicable").and_then(|e| match &e.value {
        Slot::Value(v) => Some(v.span),
        Slot::Broken => None,
    });
    let why = format!(
        "checklist {} restricts not_applicable but accepts a waiver: gate waive {} sets the whole checklist aside",
        d.id, d.id
    );
    let w = sink.push(Severity::Warning, "policy.waivable", span, why);
    w.label = "restricted here".into();
    w.help = Some(
        "add accept = [\"attested\"] to refuse waivers, or keep it if a waiver is wanted".into(),
    );
    Some(sink.report())
}

/// Every policy warning of the definition files.
pub fn check(all: &[File]) -> Report {
    let mut report = Report::default();
    for f in all.iter().filter(|f| f.kind == Kind::Gate) {
        if let Some(found) = waivable(f) {
            report.extend(found);
        }
    }
    report
}
