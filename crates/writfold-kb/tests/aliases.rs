//! #41: `aliases` in any Markdown record's front matter. `find` answers an
//! alias in its `exact` tier (and through the name tiers), and `kb lint
//! --records` warns when every word of an alias already names another
//! record, the measured way an alias steals a phrase.
mod support;

use serde_json::Value;
use support::{TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::verbs::find::Filter;

fn alias(w: &World, path: &str, aliases: &str) -> TestResult {
    let file = w.kb.join(path);
    let text = std::fs::read_to_string(&file)?;
    let text = text.replacen("---\n", &format!("---\naliases: [{aliases}]\n"), 1);
    std::fs::write(&file, text)?;
    git(&w.kb, &["commit", "-q", "-am", "aliases"])?;
    Ok(())
}

fn find(w: &World, text: &str) -> TestResult<Vec<Value>> {
    let mut request = w.request(Verb::Find {
        text: text.into(),
        filter: Filter::default(),
        signatures: false,
    });
    request.json = true;
    let output = writfold_kb::run(&request)?;
    let body: Value = serde_json::from_str(&output.text)?;
    Ok(body["rows"].as_array().ok_or("rows")?.clone())
}

#[test]
fn an_alias_is_answered_exactly() -> TestResult {
    let w = world(&[])?;
    alias(&w, "runbooks/ops/restart-harbor.md", "bounce the dock, BTD")?;
    for phrase in ["bounce the dock", "BTD"] {
        let rows = find(&w, phrase)?;
        let first = rows.first().ok_or("a row")?;
        assert_eq!(first["id"], "runbook:ops/restart-harbor", "{phrase}");
        assert_eq!(first["matched_on"], "exact", "{phrase}");
    }
    Ok(())
}

#[test]
fn lint_warns_when_an_alias_is_within_another_records_name() -> TestResult {
    let w = world(&[])?;
    alias(&w, "signatures/fsync-stall.md", "harbor api, disk stall")?;
    let mut request = w.request(Verb::Lint {
        paths: Vec::new(),
        repo: None,
        deny_warnings: false,
        records: true,
    });
    request.json = true;
    let (text, exit) = w.run(&request)?;
    assert_eq!(exit, 0, "a warning only: {text}");
    let body: Value = serde_json::from_str(&text)?;
    let found = body["diagnostics"].as_array().ok_or("diagnostics")?;
    assert_eq!(found.len(), 1, "{text}");
    assert_eq!(found[0]["code"], "alias.collides");
    assert_eq!(found[0]["file"], "signatures/fsync-stall.md");
    let message = found[0]["message"].as_str().ok_or("message")?;
    assert!(
        message.starts_with("alias \"harbor api\" is within harbor-api"),
        "{message}"
    );
    Ok(())
}
