//! `deliver modes [MODE]`: what each delivery mode asks for, before a draft
//! is written. One row per mode (built-in or file): its audience, its
//! rendering and paragraph bound, each obligation with its class and
//! whether the mode takes a waiver of it, `read:` lines for instructions,
//! the deny rule names of its audience, and the guard routes that post it.
//! `kb policy` shows the same rows after the gates. Read-only.
use super::config::{self, DIR, MODES};
use crate::emit::{Answer, More, Row};
use crate::error::{KbError, Result};
use std::path::Path;
use writfold_core::diag::suggest::{did_you_mean, suggest};

/// Every mode name: the built-in ones, then each mode file's.
pub fn names(root: &Path) -> Vec<String> {
    let mut out: Vec<String> = MODES.iter().map(|m| (*m).to_owned()).collect();
    let mut files: Vec<String> = std::fs::read_dir(root.join(DIR))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter_map(|n| n.strip_suffix(".toml").map(str::to_owned))
        .filter(|n| n != "guard" && !n.starts_with("deny-") && config::is_name(n))
        .filter(|n| !MODES.contains(&n.as_str()))
        .collect();
    files.sort();
    out.extend(files);
    out
}

/// The refusal for a mode that is not there, with a suggestion.
pub fn unknown(root: &Path, name: &str) -> KbError {
    let all = names(root);
    let refs: Vec<&str> = all.iter().map(String::as_str).collect();
    let help = did_you_mean(&suggest(name, &refs), '"')
        .map(|h| format!("; help: {h}"))
        .unwrap_or_default();
    KbError::Refused(format!(
        "no delivery mode {name}: add {DIR}/{name}.toml (schema = \"writfold-delivery-mode/1\", mode = \"{name}\"); the modes are: {}{help}",
        all.join(", ")
    ))
}

fn line(label: &'static str, text: String) -> More {
    More {
        label,
        text,
        ..More::default()
    }
}

/// One `check:` line per obligation (with `read:` for instructions);
/// returns the names the mode takes a waiver of.
fn obligation_lines(root: &Path, mode: &config::Mode, row: &mut Row) -> Vec<String> {
    let audience = mode.audience.as_ref().map(|(a, _)| a.as_str());
    let deny = config::deny(root, audience.unwrap_or("internal"));
    let obligations = match &deny {
        Ok(d) => config::obligations(mode, d),
        Err(_) => config::obligations(
            mode,
            &config::Deny {
                audience: audience.unwrap_or("internal").to_owned(),
                path: None,
                rules: Vec::new(),
            },
        ),
    };
    let mut waivable = Vec::new();
    for ob in &obligations {
        let taken = ob.accept().waived;
        if taken {
            waivable.push(ob.name.clone());
        }
        let waiver = match taken {
            true => "a waiver is taken",
            false => "no waiver",
        };
        row.more.push(line(
            "check",
            format!("{} {} ({waiver})", ob.name, ob.class),
        ));
        if let Some(read) = ob
            .gate
            .as_ref()
            .and_then(|g| crate::gate::status::read_line(g, ""))
        {
            row.more.push(read);
        }
    }
    if obligations.is_empty() {
        row.more.push(line("why", super::step::empty(&mode.name)));
    }
    waivable
}

/// The deny rule names of the mode's audience (both, when it has none).
fn deny_lines(root: &Path, audience: Option<&str>, row: &mut Row) {
    let audiences: Vec<&str> = match audience {
        Some(a) => vec![a],
        None => config::AUDIENCES.to_vec(),
    };
    for a in audiences {
        let text = match config::deny(root, a) {
            Ok(d) if d.path.is_some() => {
                let rules: Vec<&str> = d.rules.iter().map(|r| r.name.as_str()).collect();
                format!("{a}: {} rules ({})", rules.len(), rules.join(", "))
            }
            Ok(_) => format!("{a}: no deny list ({DIR}/deny-{a}.toml is the team's policy)"),
            Err(e) => format!("{a}: {e}"),
        };
        row.more.push(line("deny", text));
    }
}

/// One mode's row.
pub fn row(root: &Path, name: &str) -> Row {
    let mut row = Row {
        kind: "mode",
        id: name.to_owned(),
        section: Some("modes"),
        ..Row::default()
    };
    let mode = match config::mode(root, name) {
        Ok(m) => m,
        Err(e) => {
            row.status = Some("invalid".into());
            row.flags.push("invalid".into());
            row.text = e.to_string();
            return row;
        }
    };
    let audience = mode.audience.as_ref().map(|(a, _)| a.as_str());
    row.text = format!(
        "audience={} rendering={} paragraph_words={} source={}",
        audience.unwrap_or("any"),
        mode.rendering,
        mode.paragraph_words,
        mode.source
    );
    if mode.source != "built-in" {
        row.path = Some(mode.source.clone());
        row.line = Some(1);
    }
    let waivable = obligation_lines(root, &mode, &mut row);
    deny_lines(root, audience, &mut row);
    if let Ok(Some(g)) = config::guard(root) {
        for r in g.routes.iter().filter(|r| r.mode.as_deref() == Some(name)) {
            row.more.push(line("route", r.name()));
        }
    }
    row.extra.push(("audience", serde_json::json!(audience)));
    row.extra
        .push(("rendering", serde_json::json!(mode.rendering)));
    row.extra
        .push(("paragraph_words", serde_json::json!(mode.paragraph_words)));
    row.extra.push(("waivable", serde_json::json!(waivable)));
    row
}

/// `deliver modes [MODE]`.
pub fn answer(root: &Path, named: Option<&str>) -> Result<Answer> {
    let all = names(root);
    let shown: Vec<String> = match named {
        Some(n) if all.iter().any(|m| m == n) => vec![n.to_owned()],
        Some(n) => return Err(unknown(root, n)),
        None => all,
    };
    let mut answer = Answer::default();
    answer.rows = shown.iter().map(|m| row(root, m)).collect();
    answer.counts = vec![("modes", answer.rows.len())];
    answer.notes.push(
        "a mode lists what a candidate owes before it is posted; Writfold sends nothing".into(),
    );
    Ok(answer)
}
