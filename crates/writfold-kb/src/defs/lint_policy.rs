//! The policy warnings `kb lint` gives (policy spec 2.3): a checklist that
//! restricts `not_applicable` while its gate still accepts a waiver, and a
//! delivery mode that lists `references.denied` or a mechanical gate while
//! taking a waiver of it. Each is then one waive from being moot. A
//! warning, not an error: a team may want exactly that.
use super::lint::{File, Kind};
use super::{Sink, Slot, toml};
use writfold_core::diag::{Report, Severity};

/// The warning for one file, if it applies.
fn waivable(f: &File) -> Option<Report> {
    let d = crate::gate::def_check::check(&f.text, &f.stem, f.origin, &f.dir, &f.shown).ok()?;
    let spec = d.checklist.as_ref()?;
    spec.lists.na.as_ref()?;
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

/// The warning for a delivery mode that lists `references.denied` or a
/// mechanical gate and still takes a waiver of it (no `waivable`, or one
/// that names it): the deny list is then one `deliver waive` from moot.
fn mode_waivable(f: &File) -> Option<Report> {
    if f.stem == "guard" || f.stem.starts_with("deny-") {
        return None;
    }
    let root = f.dir.parent().and_then(std::path::Path::parent)?;
    let mode = crate::deliver::mode::mode(root, &f.stem).ok()?;
    let deny = crate::deliver::config::Deny {
        audience: "internal".into(),
        path: None,
        rules: Vec::new(),
    };
    let ob = crate::deliver::config::obligations(&mode, &deny)
        .into_iter()
        .find(|o| {
            o.accept().waived
                && (o.name == crate::deliver::config::REFERENCES
                    || o.gate.as_ref().is_some_and(|g| g.class == "mechanical"))
        })?;
    let mut sink = Sink::new(&f.shown, &f.text);
    let doc = toml::read(&f.text, &mut Sink::new("", &f.text));
    let key = match ob.gate {
        Some(_) => "gates",
        None => "checks",
    };
    let span = doc.get(key).and_then(|e| match &e.value {
        Slot::Value(v) => Some(v.span),
        Slot::Broken => None,
    });
    let why = format!(
        "mode {} lists {} and takes a waiver of it: deliver waive sets it aside",
        mode.name, ob.name
    );
    let w = sink.push(Severity::Warning, "policy.waivable", span, why);
    w.label = "listed here".into();
    w.help = Some(match ob.gate {
        Some(g) => format!("add accept = [\"satisfied\"] to .writfold/gates/{}.toml to refuse waivers, or keep it if a waiver is wanted", g.id),
        None => "add waivable = [] (or the names a waiver may set aside) to refuse it, or keep it if a waiver is wanted".into(),
    });
    Some(sink.report())
}

/// Every policy warning of the definition files.
pub fn check(all: &[File]) -> Report {
    let mut report = Report::default();
    for f in all.iter() {
        let found = match f.kind {
            Kind::Gate => waivable(f),
            Kind::Mode => mode_waivable(f),
            _ => None,
        };
        if let Some(found) = found {
            report.extend(found);
        }
    }
    report
}
