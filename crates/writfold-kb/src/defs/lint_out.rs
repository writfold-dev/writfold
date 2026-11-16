//! What `kb lint` prints: the rendering (drawn by the application on
//! stdout) and the trailer, or the JSON envelope with one row per file.
use super::lint::File;
use crate::emit::Output;
use serde_json::json;
use writfold_core::diag::Report;

pub fn output(all: &[File], report: Report, json: bool, deny: bool) -> Output {
    let invalid: Vec<&str> = all
        .iter()
        .filter(|f| {
            report
                .diagnostics
                .iter()
                .any(|d| d.file == f.shown && d.is_error())
        })
        .map(|f| f.shown.as_str())
        .collect();
    let (errors, warnings) = (report.errors(), report.warnings());
    let exit = match errors > 0 || (deny && warnings > 0) {
        true => 2,
        false => 0,
    };
    let counts = (all.len(), all.len() - invalid.len(), invalid.len());
    let trailer = format!(
        "# coverage=complete files={} clean={} invalid={} errors={errors} warnings={warnings}",
        counts.0, counts.1, counts.2
    );
    if !json {
        return Output {
            text: format!("{trailer}\n"),
            exit,
            report: None,
            shown: Some(report),
        };
    }
    let rows: Vec<serde_json::Value> = all
        .iter()
        .map(|f| {
            let status = if invalid.contains(&f.shown.as_str()) { "invalid" } else { "clean" };
            json!({"kind": "definition", "id": f.shown, "status": status, "text": "", "path": f.shown,
                   "line": null, "flags": [], "more": [], "section": null})
        })
        .collect();
    let body = json!({"writfold_kb": 1, "verb": "lint", "rows": rows, "notes": [],
        "trailer": {"coverage": "complete", "files": counts.0, "clean": counts.1, "invalid": counts.2,
                    "errors": errors, "warnings": warnings},
        "diagnostics": super::json(&report)});
    Output {
        text: format!("{body}\n"),
        exit,
        report: None,
        shown: None,
    }
}
