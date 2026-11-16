//! The table test of `defs_table.rs`, over delivery modes, deny lists,
//! the delivery guard and kind declarations: each guide exemplar with
//! each key given the wrong type, removed when required, and misspelt.
use std::error::Error;
use std::path::{Path, PathBuf};
use writfold_core::diag::{Diagnostic, Report};

#[path = "guide_toml/blocks.rs"]
mod blocks;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn value(text: &str, key: &str) -> Option<String> {
    let line = text.lines().find(|l| l.starts_with(&format!("{key} =")))?;
    line.split('"').nth(1).map(str::to_owned)
}

fn list(text: &str, key: &str) -> Vec<String> {
    let line = text
        .lines()
        .find(|l| l.starts_with(&format!("{key} =")))
        .unwrap_or("");
    line.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// A scratch KB root with stub candidate gates for a mode's `gates`.
fn root(tag: &str, gates: &[String]) -> TestResult<PathBuf> {
    let dir = std::env::temp_dir().join(format!("wf-defs-more-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join(".writfold/gates"))?;
    std::fs::create_dir_all(dir.join(".writfold/deliveries"))?;
    for g in gates {
        let body = format!(
            "schema = \"writfold-gate/1\"\nid = \"{g}\"\nversion = 1\nclass = \"attested\"\nsubject = \"candidate\"\n"
        );
        std::fs::write(dir.join(format!(".writfold/gates/{g}.toml")), body)?;
    }
    Ok(dir)
}

/// The exemplar's checker: its file stem, required keys and a checker.
fn kind(text: &str) -> Option<(String, Vec<&'static str>)> {
    let schema = value(text, "schema")?;
    match schema.as_str() {
        "writfold-delivery-mode/1" => Some((value(text, "mode")?, vec!["schema", "mode"])),
        "writfold-deny/1" => Some((
            format!("deny-{}", value(text, "audience")?),
            vec!["schema", "audience"],
        )),
        "writfold-guard/1" => Some(("guard".into(), vec!["schema"])),
        "writfold-source/1" => Some((value(text, "kind")?, vec!["schema", "kind", "freshness"])),
        _ => None,
    }
}

fn check(root: &Path, text: &str, stem: &str) -> Report {
    let result = match value(text, "schema").as_deref() {
        Some("writfold-source/1") => {
            writfold_kb::stale::kind_check::check(text, stem, "f.toml").err()
        }
        _ => writfold_kb::deliver::config::check_file(
            &root.join(".writfold/deliveries"),
            stem,
            "f.toml",
            text,
        )
        .err(),
    };
    result.unwrap_or_default()
}

fn keys(text: &str) -> Vec<(u32, String, char)> {
    text.lines()
        .enumerate()
        .filter_map(|(n, l)| {
            let (k, rest) = l.split_once(" = ")?;
            let first = rest.trim_start().chars().next().unwrap_or(' ');
            Some((u32::try_from(n + 1).unwrap_or(0), k.to_owned(), first))
        })
        .collect()
}

fn replace_line(text: &str, line: u32, with: Option<&str>) -> String {
    let mut out = String::new();
    for (n, l) in text.lines().enumerate() {
        let here = u32::try_from(n + 1).ok() == Some(line);
        match (here, with) {
            (true, Some(w)) => out.push_str(&format!("{w}\n")),
            (true, None) => {}
            (false, _) => out.push_str(&format!("{l}\n")),
        }
    }
    out
}

fn named(d: &Diagnostic) -> Vec<&str> {
    d.message.split('`').skip(1).step_by(2).collect()
}

#[test]
fn delivery_and_kind_files_name_the_wrong_key_at_its_line() -> TestResult {
    let guide = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide");
    let mut seen = 0;
    for page in ["delivery-modes", "checkers", "kb-layout", "validation"] {
        let text = std::fs::read_to_string(guide.join(format!("{page}.md")))?;
        for (n, block) in blocks::blocks(page, &text).into_iter().enumerate() {
            let Some((stem, required)) = kind(&block.text).filter(|_| !block.planned) else {
                continue;
            };
            let root = root(&format!("{page}-{n}"), &list(&block.text, "gates"))?;
            let e = &block.text;
            assert!(
                check(&root, e, &stem).diagnostics.is_empty(),
                "{}: {:?}",
                block.at,
                check(&root, e, &stem)
            );
            seen += 1;
            for (line, key, first) in keys(e) {
                for (kind, other) in [('"', "\"x\""), ('7', "7"), ('[', "[\"x\"]")] {
                    if (kind == '7' && first.is_ascii_digit()) || kind == first {
                        continue;
                    }
                    let report = check(
                        &root,
                        &replace_line(e, line, Some(&format!("{key} = {other}"))),
                        &stem,
                    );
                    let hit = report.diagnostics.iter().any(|d| {
                        d.code == "key.type"
                            && named(d).contains(&key.as_str())
                            && d.at.map(|a| a.line) == Some(line)
                    });
                    assert!(hit, "{}: {key} = {other}: {report:?}", block.at);
                }
                if required.contains(&key.as_str()) {
                    let report = check(&root, &replace_line(e, line, None), &stem);
                    let missing: Vec<&Diagnostic> = report
                        .diagnostics
                        .iter()
                        .filter(|d| d.code == "key.missing")
                        .collect();
                    assert!(
                        missing.len() == 1 && named(missing[0]).contains(&key.as_str()),
                        "{}: without {key}: {report:?}",
                        block.at
                    );
                }
                if key.len() >= 4 {
                    let mut wrong = key.clone();
                    wrong.remove(key.len() / 2);
                    let original = e.lines().nth(usize::try_from(line - 1)?).unwrap_or("");
                    let report = check(
                        &root,
                        &replace_line(e, line, Some(&original.replacen(&key, &wrong, 1))),
                        &stem,
                    );
                    let unknown = report.diagnostics.iter().find(|d| d.code == "key.unknown");
                    let help = unknown.and_then(|d| d.help.clone());
                    assert_eq!(
                        help,
                        Some(format!("did you mean `{key}`?")),
                        "{}: {wrong}: {report:?}",
                        block.at
                    );
                }
            }
            let _ = std::fs::remove_dir_all(&root);
        }
    }
    assert!(seen >= 4, "exemplars found: {seen}");
    Ok(())
}
