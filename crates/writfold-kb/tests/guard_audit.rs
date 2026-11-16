//! `mode = "audit"`, waivers in the guard's answers, and the guard log:
//! an audit rule is judged and logged but never decides the call; a pass
//! that rested on a waiver says so; `guard log` counts per rule version.
#[path = "guard_status/helpers.rs"]
#[allow(dead_code)]
mod helpers;
mod support;

use helpers::*;
use serde_json::Value;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::guard::GuardVerb;

const AUDIT: &str = "mode = \"audit\"\n";

fn log(w: &World) -> TestResult<Vec<Value>> {
    let text = std::fs::read_to_string(w.kb.join(".git/writfold/guard-log.jsonl"))?;
    Ok(text
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}

fn guard_log(w: &World, rule: Option<&str>) -> TestResult<(Value, u8)> {
    let mut request = w.request(Verb::Guard(GuardVerb::Log {
        rule: rule.map(str::to_owned),
        since: None,
    }));
    request.cwd = w.code.clone();
    request.json = true;
    let output = writfold_kb::run(&request)?;
    Ok((serde_json::from_str(&output.text)?, output.exit))
}

fn audited(w: &World) -> TestResult {
    rule(&w.kb, "commit", &format!("{COMMIT}{AUDIT}"))?;
    current(w)?;
    definition(w, "unit", UNIT)
}

#[test]
fn an_audit_rule_lets_the_call_through_and_logs_it() -> TestResult {
    let w = world(&[])?;
    audited(&w)?;
    let out = hook(&w, false, "s-audit");
    assert!(out.stdout.is_empty(), "{}", out.stdout);
    assert!(
        out.stderr
            .contains("writfold kb hook: guard rule commit (audit) would deny: "),
        "{}",
        out.stderr
    );
    let entries = log(&w)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(
        (
            entries[0]["decided"].as_str(),
            entries[0]["effect"].as_str()
        ),
        (Some("deny"), Some("pass"))
    );
    assert_eq!(entries[0]["mode"], "audit");
    assert_eq!(entries[0]["command"], "git commit -m x");
    assert_eq!(entries[0]["repo"], "demo-service");
    // Without mode, the same rule denies and logs its denial.
    rule(&w.kb, "commit", COMMIT)?;
    assert!(!hook(&w, false, "s-audit").stdout.is_empty());
    let entries = log(&w)?;
    assert_eq!(
        (entries.len(), entries[1]["effect"].as_str()),
        (2, Some("deny"))
    );
    Ok(())
}

#[test]
fn an_enforcing_rule_beside_an_audit_rule_decides_alone() -> TestResult {
    let w = world(&[])?;
    audited(&w)?;
    rule(
        &w.kb,
        "push",
        &COMMIT.replace("id = \"commit\"", "id = \"push\""),
    )?;
    let out = hook(&w, false, "s-both");
    let value: Value = serde_json::from_str(&out.stdout)?;
    let reason = value["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .ok_or("reason")?;
    assert!(
        reason.contains("rule push ") && !reason.contains("rule commit "),
        "{reason}"
    );
    let rules: Vec<String> = log(&w)?
        .iter()
        .filter_map(|e| e["rule"].as_str().map(str::to_owned))
        .collect();
    assert_eq!(rules, ["commit", "push"]);
    Ok(())
}

#[test]
fn guard_status_flags_audit_and_waived() -> TestResult {
    let w = world(&[])?;
    audited(&w)?;
    let (answer, exit) = status(&w)?;
    let guard = row(&answer, "commit")?;
    assert_eq!(guard["status"], "would-deny");
    assert_eq!(guard["flags"], serde_json::json!(["audit"]));
    let counts = &answer["trailer"]["counts"];
    assert_eq!(
        (counts["audit"].as_u64(), counts["deny"].as_u64(), exit),
        (Some(1), Some(0), 2)
    );
    let mode = guard["more"]
        .as_array()
        .ok_or("more")?
        .iter()
        .any(|m| m["label"] == "mode");
    assert!(mode, "{guard}");
    // Enforced, and passing on a waiver.
    rule(&w.kb, "commit", COMMIT)?;
    say(&w, Kind::Waived, "unit")?;
    let (answer, exit) = status(&w)?;
    let guard = row(&answer, "commit")?;
    assert_eq!((guard["status"].as_str(), exit), (Some("would-pass"), 0));
    assert_eq!(guard["flags"], serde_json::json!(["waived"]));
    let waived = &guard["more"][0];
    assert_eq!(waived["label"], "waived");
    assert!(
        waived["text"]
            .as_str()
            .is_some_and(|t| t.contains("looked at it, by owner")),
        "{waived}"
    );
    let out = hook(&w, false, "s-waived");
    assert!(out.stdout.is_empty());
    assert!(
        out.stderr
            .contains("guard rule commit passed on a waiver: gate unit"),
        "{}",
        out.stderr
    );
    let entries = log(&w)?;
    let last = entries.last().ok_or("an entry")?;
    assert_eq!(last["decided"], "pass");
    assert!(
        last["waived"].as_array().is_some_and(|w| w.len() == 1),
        "{last}"
    );
    Ok(())
}

#[test]
fn guard_log_counts_the_current_version_and_suggests_enforcing() -> TestResult {
    let w = world(&[])?;
    audited(&w)?;
    // Never matched: !unmatched, exit 2.
    let (answer, exit) = guard_log(&w, None)?;
    assert_eq!(exit, 2);
    assert!(
        row(&answer, "commit")?["flags"]
            .to_string()
            .contains("unmatched")
    );
    for _ in 0..3 {
        hook(&w, false, "s-log");
    }
    let (answer, exit) = guard_log(&w, None)?;
    let r = row(&answer, "commit")?;
    assert_eq!(exit, 2);
    assert!(
        r["text"]
            .as_str()
            .is_some_and(|t| t.starts_with("matched=3 would-deny=3")),
        "{r}"
    );
    assert_eq!(r["more"][0]["label"], "would-deny");
    // The rule edited: earlier entries are not counted; twelve passes.
    assert_eq!(check(&w, "unit")?, 0);
    rule(
        &w.kb,
        "commit",
        &format!("{COMMIT}{AUDIT}description = \"edited\"\n"),
    )?;
    for _ in 0..12 {
        hook(&w, false, "s-log");
    }
    let (answer, exit) = guard_log(&w, Some("commit"))?;
    let r = row(&answer, "commit")?;
    assert_eq!(exit, 0, "{answer}");
    assert!(
        r["text"]
            .as_str()
            .is_some_and(|t| t.starts_with("matched=12 would-deny=0")),
        "{r}"
    );
    let labels: Vec<&str> = r["more"]
        .as_array()
        .ok_or("more")?
        .iter()
        .filter_map(|m| m["label"].as_str())
        .collect();
    assert_eq!(labels, ["earlier", "next"]);
    Ok(())
}

#[test]
fn rotation_torn_lines_and_an_unwritable_log() -> TestResult {
    let w = world(&[])?;
    audited(&w)?;
    let path = w.kb.join(".git/writfold/guard-log.jsonl");
    std::fs::write(&path, vec![b'x'; 4 * 1024 * 1024])?;
    hook(&w, false, "s-rot");
    assert!(w.kb.join(".git/writfold/guard-log.1.jsonl").is_file());
    assert_eq!(log(&w)?.len(), 1);
    let mut text = std::fs::read_to_string(&path)?;
    text.push_str("{\"schema\":\"writfold-guard-log/1\",\"rule\":\"com");
    std::fs::write(&path, text)?;
    let (answer, _) = guard_log(&w, None)?;
    // The rotated file is one unparsable line, and so is the torn one.
    assert_eq!(answer["trailer"]["counts"]["unparsed"], 2);
    assert_eq!(answer["trailer"]["counts"]["entries"], 1);
    std::fs::remove_file(&path)?;
    std::fs::create_dir(&path)?;
    rule(&w.kb, "commit", COMMIT)?;
    let out = hook(&w, false, "s-rot");
    assert!(!out.stdout.is_empty(), "the decision changed");
    assert!(
        out.stderr.contains("cannot write the guard log"),
        "{}",
        out.stderr
    );
    Ok(())
}

#[test]
fn mode_is_strict() {
    let base = COMMIT.replace("id = \"commit\"", "id = \"r\"");
    let warn = format!("{base}mode = \"warn\"\n");
    assert!(writfold_kb::guard::rules::parse(&warn, "r", "r.toml").is_err());
    let audit = format!("{base}{AUDIT}");
    assert!(writfold_kb::guard::rules::parse(&audit, "r", "r.toml").is_ok());
    let stop = format!("{}{AUDIT}", STOP.replace("id = \"finish\"", "id = \"r\""));
    let why = writfold_kb::guard::rules::parse(&stop, "r", "r.toml").err();
    assert_eq!(
        why.as_deref(),
        Some("mode applies to a pre-tool-use rule only")
    );
}
