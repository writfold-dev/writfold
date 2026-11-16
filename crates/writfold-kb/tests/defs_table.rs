//! The table test over every kind of definition file: each kind's valid
//! exemplars (the guide's fenced blocks) with each key given the wrong
//! type, removed and misspelt. A diagnostic must name the key that is
//! wrong, at its line, and never blame a key the file does not have.
use std::error::Error;
use std::path::{Path, PathBuf};
use writfold_core::diag::{Diagnostic, Report};
use writfold_kb::gate::def::Origin;
use writfold_kb::gate::def_check;

#[path = "guide_toml/blocks.rs"]
mod blocks;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// A gate exemplar and the directory its files are in.
struct Exemplar {
    at: String,
    text: String,
    id: String,
    dir: PathBuf,
}

fn exemplars(tag: &str) -> TestResult<Vec<Exemplar>> {
    let guide = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide");
    let mut out = Vec::new();
    for page in ["gates", "run-graph"] {
        let text = std::fs::read_to_string(guide.join(format!("{page}.md")))?;
        for block in blocks::blocks(page, &text) {
            if block.planned || !block.text.contains("\"writfold-gate/1\"") {
                continue;
            }
            let id = value(&block.text, "id").ok_or("no id")?;
            let dir = std::env::temp_dir().join(format!(
                "wf-defs-table-{tag}-{}-{}-{id}",
                std::process::id(),
                out.len()
            ));
            std::fs::create_dir_all(&dir)?;
            if let Some(name) = value(&block.text, "instructions") {
                std::fs::write(dir.join(name), "read me\n")?;
            }
            for script in block
                .text
                .split('"')
                .filter_map(|w| w.strip_prefix("{gate}/"))
            {
                std::fs::write(dir.join(script), "exit 0\n")?;
            }
            out.push(Exemplar {
                at: block.at,
                text: block.text,
                id,
                dir,
            });
        }
    }
    Ok(out)
}

/// The string value of `key` on its line.
fn value(text: &str, key: &str) -> Option<String> {
    let line = text.lines().find(|l| l.starts_with(&format!("{key} =")))?;
    line.split('"').nth(1).map(str::to_owned)
}

/// Each `key = …` line: (line number, key, the value's first character).
fn keys(text: &str) -> Vec<(u32, String, char)> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if let Some((key, rest)) = line.split_once(" = ") {
            let first = rest.trim_start().chars().next().unwrap_or(' ');
            out.push((u32::try_from(n + 1).unwrap_or(0), key.to_owned(), first));
        }
    }
    out
}

fn check(e: &Exemplar, text: &str) -> Report {
    let file = format!(".writfold/gates/{}.toml", e.id);
    match def_check::check(text, &e.id, Origin::Kb, &e.dir, &file) {
        Ok(_) => Report::default(),
        Err(report) => report,
    }
}

fn replace_line(text: &str, line: u32, with: Option<&str>) -> String {
    let mut out = String::new();
    for (n, l) in text.lines().enumerate() {
        match (u32::try_from(n + 1).ok() == Some(line), with) {
            (true, Some(w)) => {
                out.push_str(w);
                out.push('\n');
            }
            (true, None) => {}
            (false, _) => {
                out.push_str(l);
                out.push('\n');
            }
        }
    }
    out
}

/// The keys named in backticks in a message.
fn named(d: &Diagnostic) -> Vec<&str> {
    d.message.split('`').skip(1).step_by(2).collect()
}

/// No diagnostic but `key.missing` names a key absent from `text`.
fn blames_only_present(text: &str, report: &Report) {
    let present: Vec<String> = keys(text).into_iter().map(|(_, k, _)| k).collect();
    for d in &report.diagnostics {
        for key in named(d) {
            let is_key = key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit());
            if is_key && d.code != "key.missing" {
                assert!(
                    present.iter().any(|p| p == key),
                    "blames absent `{key}`: {d:?}\n{text}"
                );
            }
        }
    }
}

fn required(text: &str) -> Vec<&'static str> {
    let class = value(text, "class").unwrap_or_default();
    let candidate = value(text, "subject").as_deref() == Some("candidate");
    let mut out = vec!["schema", "id", "version", "class"];
    out.extend(match class.as_str() {
        "mechanical" | "observed" if candidate => vec!["argv", "timeout_seconds"],
        "mechanical" | "observed" => vec!["argv", "timeout_seconds", "scope"],
        "attested" if candidate => vec![],
        "attested" => vec!["scope"],
        "assessed" => vec!["weft", "checks", "weft_sha256", "timeout_seconds"],
        "run-record" => vec!["require"],
        _ => vec!["scope", "items"],
    });
    out
}

#[test]
fn every_key_wrong_removed_or_misspelt_is_named_at_its_line() -> TestResult {
    let all = exemplars("each")?;
    assert!(all.len() >= 8, "the guide's gate blocks");
    for e in &all {
        let first = check(e, &e.text);
        assert!(
            first.diagnostics.is_empty(),
            "exemplar {} refused: {first:?}",
            e.at
        );
        for (line, key, first) in keys(&e.text) {
            for (kind, other) in [('"', "\"x\""), ('7', "7"), ('[', "[\"x\"]")] {
                let same = (kind == '7' && first.is_ascii_digit()) || kind == first;
                if same {
                    continue;
                }
                let text = replace_line(&e.text, line, Some(&format!("{key} = {other}")));
                let report = check(e, &text);
                let hit = report.diagnostics.iter().any(|d| {
                    d.code == "key.type"
                        && named(d).contains(&key.as_str())
                        && d.at.map(|a| a.line) == Some(line)
                });
                assert!(hit, "{key} = {other}: {report:?}");
                blames_only_present(&text, &report);
            }
            if required(&e.text).contains(&key.as_str()) {
                let text = replace_line(&e.text, line, None);
                let report = check(e, &text);
                let missing: Vec<&Diagnostic> = report
                    .diagnostics
                    .iter()
                    .filter(|d| d.code == "key.missing")
                    .collect();
                assert_eq!(missing.len(), 1, "without {key}: {report:?}");
                assert!(
                    named(missing[0]).contains(&key.as_str()),
                    "{key}: {report:?}"
                );
                assert!(
                    report
                        .diagnostics
                        .iter()
                        .all(|d| d.code != "key.type" && d.code != "key.range"),
                    "{report:?}"
                );
                blames_only_present(&text, &report);
            }
            if key.len() >= 4 {
                let mut wrong = key.clone();
                wrong.remove(key.len() / 2);
                let original = e.text.lines().nth(usize::try_from(line - 1)?).unwrap_or("");
                let text = replace_line(&e.text, line, Some(&original.replacen(&key, &wrong, 1)));
                let report = check(e, &text);
                let unknown = report.diagnostics.iter().find(|d| d.code == "key.unknown");
                let unknown = unknown.ok_or_else(|| format!("{wrong}: {report:?}"))?;
                assert_eq!(unknown.at.map(|a| a.line), Some(line));
                assert_eq!(
                    unknown.help.as_deref(),
                    Some(format!("did you mean `{key}`?").as_str()),
                    "{wrong}"
                );
                let blamed = report
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "key.missing" && named(d).contains(&key.as_str()));
                assert!(!blamed, "{wrong} also missing: {report:?}");
            }
        }
    }
    for e in all {
        let _ = std::fs::remove_dir_all(&e.dir);
    }
    Ok(())
}

#[test]
fn a_wrong_version_and_a_missing_timeout_are_both_reported_version_first() -> TestResult {
    let all = exemplars("two")?;
    let e = all
        .iter()
        .find(|e| e.id == "unit")
        .ok_or("no unit exemplar")?;
    let text = e.text.replace("version = 1", "version = \"one\"");
    let line = keys(&text)
        .into_iter()
        .find(|(_, k, _)| k == "timeout_seconds")
        .ok_or("no timeout")?
        .0;
    let text = replace_line(&text, line, None);
    let report = check(e, &text);
    let codes: Vec<(&str, Option<u32>)> = report
        .diagnostics
        .iter()
        .map(|d| (d.code, d.at.map(|a| a.line)))
        .collect();
    assert_eq!(
        codes,
        [("key.type", Some(3)), ("key.missing", None)],
        "{report:?}"
    );
    assert_eq!(
        report.diagnostics[0].help.as_deref(),
        Some("write a number without quotes: version = 1")
    );
    assert_eq!(
        report.diagnostics[1].message,
        "a mechanical gate needs `timeout_seconds`"
    );
    for e in all {
        let _ = std::fs::remove_dir_all(&e.dir);
    }
    Ok(())
}
