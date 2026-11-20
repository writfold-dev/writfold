//! The `kb lint` warning for looks that never age (gate-evidence spec,
//! stage 4): a `no-data` or `not-found` observation whose `--source` kind
//! has no declaration in `.writfold/sources/` keeps its standing for ever,
//! because only a declared kind's `default_ttl` ages it. One warning per
//! such kind, on its newest look, with the count. It reads the runs'
//! observation files and the declarations' names, nothing else. A warning,
//! not an error: an undeclared kind is valid, and its looks behave as
//! before v0.6.0-dev.9.
use super::Sink;
use std::collections::BTreeMap;
use std::path::Path;
use writfold_core::diag::{Report, Severity};
use writfold_core::observe::Status;

/// Every negative look of an undeclared kind, as one warning per kind.
pub fn check(kb: &Path) -> Report {
    let mut by_kind: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let Ok(runs) = std::fs::read_dir(kb.join("runs")) else {
        return Report::default();
    };
    let mut dirs: Vec<String> = runs
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    dirs.sort();
    for run in dirs {
        for f in crate::observe::filed(kb, &format!("runs/{run}")) {
            let Ok(o) = &f.parsed else { continue };
            let negative = matches!(o.status, Status::NoData | Status::NotFound);
            let declared = kb
                .join(".writfold/sources")
                .join(format!("{}.toml", o.source))
                .is_file();
            if negative && !declared {
                let entry = by_kind.entry(o.source.clone()).or_default();
                entry.0 += 1;
                entry.1 = newest(&entry.1, &f.path);
            }
        }
    }
    let mut report = Report::default();
    for (kind, (count, path)) in by_kind {
        let text = std::fs::read_to_string(kb.join(&path)).unwrap_or_default();
        let mut sink = Sink::new(&path, &text);
        let w = sink.push(
            Severity::Warning,
            "policy.never-ages",
            None,
            format!(
                "{count} no-data or not-found look(s) of kind {kind} never age: the kind is not declared"
            ),
        );
        w.help = Some(format!(
            "declare it in .writfold/sources/{kind}.toml with a default_ttl, so a look that found nothing turns stale"
        ));
        report.extend(sink.report());
    }
    report
}

/// The later of two KB-relative observation paths (names sort by time).
fn newest(a: &str, b: &str) -> String {
    let name = |p: &str| p.rsplit('/').next().unwrap_or_default().to_owned();
    match name(b) >= name(a) {
        true => b.to_owned(),
        false => a.to_owned(),
    }
}
