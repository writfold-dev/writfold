//! Read-only guard projections agree with matching hooks over the same facts.
#[path = "guard_status/helpers.rs"]
mod helpers;
mod support;

use helpers::*;
use serde_json::{Value, json};
use support::{NOW, TestResult, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::guard::GuardVerb;
use writfold_kb::plan::PlanVerb;

#[test]
fn gate_judgements_match_hooks() -> TestResult {
    let w = world(&[])?;
    rule(&w.kb, "commit", COMMIT)?;
    current(&w)?;
    definition(&w, "unit", UNIT)?;
    definition(&w, "review", REVIEW)?;
    let open = parity(&w, "deny")?;
    let owed = row(&open, "commit")?["more"].to_string();
    assert!(owed.contains("unit [open]") && owed.contains("review [open]"));
    assert_eq!(check(&w, "unit")?, 0);
    let partly = parity(&w, "deny")?;
    assert!(
        !row(&partly, "commit")?["more"]
            .to_string()
            .contains("gate unit")
    );
    say(&w, Kind::Attested, "review")?;
    parity(&w, "pass")?;
    put(&w.code.join("src/ledger.rs"), "pub fn write() {}\n")?;
    let stale = parity(&w, "deny")?;
    let owed = row(&stale, "commit")?["more"].to_string();
    assert!(owed.contains("unit [stale]") && owed.contains("review [stale]"));
    say(&w, Kind::Waived, "unit")?;
    say(&w, Kind::Attested, "review")?;
    parity(&w, "pass")?;
    Ok(())
}

#[test]
fn failed_and_unknown_gates_ask() -> TestResult {
    let w = world(&[])?;
    current(&w)?;
    definition(
        &w,
        "fail",
        &UNIT
            .replace("unit", "fail")
            .replace("\"true\"", "\"false\""),
    )?;
    definition(
        &w,
        "crash",
        &UNIT.replace("unit", "crash").replace(
            "[\"true\"]",
            "[\"sh\", \"-c\", \"ulimit -c 0; kill -SEGV $$\"]",
        ),
    )?;
    rule(
        &w.kb,
        "commit",
        &COMMIT
            .replace(
                "\"run.current\", \"gates.met\"",
                "\"gate:fail\", \"gate:crash\"",
            )
            .replace("\"deny\"", "\"ask\""),
    )?;
    assert_eq!(check(&w, "fail")?, 2);
    assert_eq!(check(&w, "crash")?, 2);
    let answer = parity(&w, "ask")?;
    let owed = row(&answer, "commit")?["more"].as_array().ok_or("more")?;
    assert_eq!(owed.len(), 2);
    assert!(
        owed[0]["text"]
            .as_str()
            .ok_or("text")?
            .contains("fail [failed]")
    );
    assert!(
        owed[1]["text"]
            .as_str()
            .ok_or("text")?
            .contains("crash [unknown]")
    );
    // An unknown gate verdict is an unmet requirement, not Fact::Unknown.
    assert_eq!(owed[1]["flags"], json!([]));
    Ok(())
}

#[test]
fn absent_and_closed_current_run_match_hooks() -> TestResult {
    let w = world(&[])?;
    rule(&w.kb, "commit", COMMIT)?;
    let absent = parity(&w, "deny")?;
    let owed = row(&absent, "commit")?["more"].as_array().ok_or("more")?;
    assert_eq!(owed.len(), 2);
    assert_eq!(owed[0]["flags"], json!([]));
    assert_eq!(owed[1]["flags"], json!(["unknown"]));
    assert!(absent["notes"].to_string().contains("current run none"));
    current(&w)?;
    let path = w.kb.join(format!("runs/{RUN}/manifest.json"));
    let mut manifest: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    manifest["state"] = json!("closed");
    put(&path, &serde_json::to_string_pretty(&manifest)?)?;
    let closed = parity(&w, "deny")?;
    assert!(
        row(&closed, "commit")?["more"]
            .to_string()
            .contains("is closed")
    );
    Ok(())
}

#[test]
fn stop_uses_fresh_session_facts_not_nudge_suppression() -> TestResult {
    let w = world(&[])?;
    current(&w)?;
    definition(&w, "unit", UNIT)?;
    put(
        &w.kb.join(".writfold/plans/change.toml"),
        "schema = \"writfold-plan/1\"\nname = \"change\"\nsteps = [\"implement: unit\"]\n",
    )?;
    let set = PlanVerb::Set {
        template: Some("change".into()),
        from: None,
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    assert_eq!(writfold_kb::run(&w.request(Verb::Plan(set)))?.exit, 0);
    rule(&w.kb, "finish", STOP)?;
    let (answer, exit) = status(&w)?;
    assert_eq!(exit, 2);
    let guard = row(&answer, "finish")?;
    assert_eq!(guard["status"], "would-nudge");
    assert_eq!(guard["flags"], json!(["nudge"]));
    assert_eq!(answer["trailer"]["counts"]["nudge"], 1);
    let first = hook(&w, true, "stop-fresh");
    let value: Value = serde_json::from_str(&first.stdout)?;
    let context = value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("context")?;
    for owed in guard["more"].as_array().ok_or("more")? {
        assert!(context.contains(owed["text"].as_str().ok_or("owed text")?));
    }
    assert!(hook(&w, true, "stop-fresh").stdout.is_empty());
    assert_eq!(row(&status(&w)?.0, "finish")?["status"], "would-nudge");
    assert_eq!(check(&w, "unit")?, 0);
    let (met, exit) = status(&w)?;
    assert_eq!(exit, 0);
    assert_eq!(row(&met, "finish")?["status"], "quiet");
    assert_eq!(row(&met, "finish")?["more"], json!([]));
    assert!(hook(&w, true, "another-fresh-session").stdout.is_empty());
    Ok(())
}

#[test]
fn excluded_and_invalid_rules_do_not_deny_hooks() -> TestResult {
    let w = world(&[])?;
    rule(
        &w.code,
        "commit",
        &COMMIT.replace("require =", "repos = [\"*\", \"!demo-service\"]\nrequire ="),
    )?;
    let (answer, exit) = status(&w)?;
    assert_eq!(exit, 0);
    let guard = row(&answer, "commit")?;
    assert_eq!(guard["status"], "not-here");
    assert_eq!(guard["more"], json!([]));
    assert_eq!(guard["flags"], json!([]));
    assert_eq!(
        guard["path"],
        w.code
            .join(".writfold/guards/commit.toml")
            .to_string_lossy()
            .as_ref()
    );
    assert!(hook(&w, false, "excluded").stdout.is_empty());
    rule(&w.kb, "broken", "not valid toml [")?;
    let (invalid, exit) = status(&w)?;
    assert_eq!(exit, 2);
    assert_eq!(invalid["trailer"]["counts"]["invalid"], 1);
    let rows = invalid["rows"].as_array().ok_or("rows")?;
    let broken = rows
        .iter()
        .find(|r| r["kind"] == "guard-rule")
        .ok_or("invalid row")?;
    assert_eq!(broken["status"], "invalid");
    assert_eq!(broken["flags"], json!(["invalid"]));
    assert!(!broken["text"].as_str().ok_or("invalid reason")?.is_empty());
    assert!(hook(&w, false, "invalid").stdout.is_empty());
    Ok(())
}

#[test]
fn no_rules_and_delivery_guard_are_not_evaluated() -> TestResult {
    let w = world(&[])?;
    let (answer, exit) = status(&w)?;
    assert_eq!(exit, 0);
    assert_eq!(answer["trailer"]["answer"], "unknown");
    assert_eq!(answer["trailer"]["reason"], "no-guards");
    assert!(answer["notes"].to_string().contains(".writfold/guards/"));
    assert!(hook(&w, false, "no-rules").stdout.is_empty());
    // Invalid delivery configuration must still only be mentioned, never evaluated.
    put(
        &w.kb.join(".writfold/deliveries/guard.toml"),
        "not valid toml [",
    )?;
    let (delivery, exit) = status(&w)?;
    assert_eq!(exit, 0);
    assert_eq!(delivery["trailer"]["reason"], "no-guards");
    assert_eq!(delivery["rows"], json!([]));
    let notes = delivery["notes"].to_string();
    assert!(
        notes.contains("delivery guard") && notes.contains("not covered"),
        "{notes}"
    );
    Ok(())
}

#[test]
fn guard_status_golden() -> TestResult {
    let w = world(&[])?;
    rule(
        &w.kb,
        "z-last",
        &COMMIT.replace("id = \"commit\"", "id = \"z-last\""),
    )?;
    rule(
        &w.code,
        "a-first",
        &COMMIT
            .replace("id = \"commit\"", "id = \"a-first\"")
            .replace("\"deny\"", "\"ask\""),
    )?;
    rule(&w.kb, "finish", STOP)?;
    let (answer, exit) = status(&w)?;
    assert_eq!(exit, 2);
    let rows = answer["rows"].as_array().ok_or("rows")?;
    assert_eq!(
        rows.iter().map(|r| r["id"].clone()).collect::<Vec<_>>(),
        vec![json!("a-first"), json!("finish"), json!("z-last")]
    );
    assert_eq!(
        answer["trailer"]["counts"],
        json!({"rules": 3, "deny": 1, "ask": 1, "nudge": 1, "invalid": 0})
    );
    for row in rows {
        assert_eq!(row["kind"], "guard");
        assert_eq!(row["line"], 1);
        assert!(row["path"].as_str().ok_or("path")?.ends_with(".toml"));
    }
    let text = rows[0]["text"].as_str().ok_or("rule text")?;
    for piece in [
        "pre-tool-use",
        "git commit",
        "git push",
        "Edit",
        "requires",
        "run.current",
        "gates.met",
    ] {
        assert!(text.contains(piece), "{text}");
    }
    let notes = answer["notes"].to_string();
    assert!(
        notes.contains("as if each rule had matched; a guard, not enforcement; nothing was run")
    );
    let mut request = w.request(Verb::Guard(GuardVerb::Status));
    request.cwd = w.code.clone();
    request.now = Some(NOW + 3600);
    let (text, exit) = w.run(&request)?;
    // The query binds the working directory, including this fixture's temporary root.
    let query = regex::Regex::new(r" q=[0-9a-f]{12}\n")?;
    let text = query.replace(&text, " q=<QUERY>\n");
    let actual = format!("{text}# exit={exit}\n");
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/guard-status.txt");
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(path, actual)?;
    } else {
        assert_eq!(std::fs::read_to_string(path)?, actual);
    }
    Ok(())
}
