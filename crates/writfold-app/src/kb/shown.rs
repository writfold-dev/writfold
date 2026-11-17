//! Diagnostics on stderr: a refusal over files a person wrote is drawn in
//! full ([`writfold_kb::diag::render`]) and still ends with the one
//! `writfold kb: refused: …` line a script reads last; a status verb's
//! diagnostics are drawn capped above its rows.
use super::Style;
use writfold_kb::diag::{Report, render, render_capped};
use writfold_kb::emit::Output;
use writfold_kb::error::KbError;

/// The last line of a refusal over `report`:
/// `writfold kb: refused: <file>: N errors`.
pub fn refused_line(report: &Report) -> String {
    let file = match (report.files(), report.diagnostics.first()) {
        (1, Some(d)) => d.file.clone(),
        (n, _) => format!("{n} files"),
    };
    let count = writfold_kb::diag::render::counted(report);
    format!("writfold kb: refused: {file}: {count}\n")
}

/// What stderr shows for an error.
pub fn error(error: &KbError, style: &Style) -> String {
    match error {
        KbError::Definition(report) => {
            format!("{}\n{}", render(report, style), refused_line(report))
        }
        other => format!("writfold kb: {other}\n"),
    }
}

/// What stdout shows: the diagnostics that are the answer, then the text.
pub fn stdout(output: &Output, style: &Style) -> String {
    match &output.shown {
        Some(report) if !report.diagnostics.is_empty() => {
            format!("{}\n{}", render(report, style), output.text)
        }
        _ => output.text.clone(),
    }
}

/// What stderr shows beside a verb's rows: its diagnostics, at most three.
pub fn stderr(output: &Output, style: &Style) -> String {
    let diagnostics = match &output.report {
        Some(report) if !report.diagnostics.is_empty() => render_capped(report, style, 3, 3),
        _ => String::new(),
    };
    format!(
        "{diagnostics}{}",
        output.drawn.as_deref().unwrap_or_default()
    )
}
