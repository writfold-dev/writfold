//! Detailed plan status is an opt-in projection of the same judgments.
mod checklist;
mod support;

use checklist::{RUN, STALL, cover, define, golden, put, run_unit};
use serde_json::{Value, json};
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::{GateVerb, checklist::Outcome};
use writfold_kb::plan::PlanVerb;
use writfold_kb::write::{Write, record::Ticket};

fn plan(w: &World, action: &str) -> TestResult {
    let run = Some(RUN.into());
    let repo = Some(w.code.clone());
    let verb = match action {
        "set" => PlanVerb::Set {
            template: Some("detail".into()),
            from: None,
            run,
            repo,
        },
        "skip" => PlanVerb::Skip {
            reason: "not needed".into(),
            by: "owner".into(),
            run,
            repo,
        },
        _ => PlanVerb::Advance { run, repo },
    };
    let (text, exit) = w.run(&w.request(Verb::Plan(verb)))?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

fn status(w: &World, gates: bool, subject: bool) -> TestResult<Value> {
    let mut request = w.request(Verb::Plan(PlanVerb::Status {
        run: Some(RUN.into()),
        repo: subject.then(|| w.code.clone()),
        gates,
    }));
    if !subject {
        request.cwd = w.base.clone();
    }
    request.json = true;
    request.budget = writfold_kb::MAX_BUDGET;
    let (text, _) = w.run(&request)?;
    Ok(serde_json::from_str(&text)?)
}

fn gate(w: &World, id: &str) -> TestResult {
    let (text, exit) = w.run(&w.request(Verb::Gate(GateVerb::Run {
        id: id.into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    })))?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

fn template(w: &World, steps: &str) -> TestResult {
    put(
        &w.kb.join(".writfold/plans/detail.toml"),
        &format!("schema = \"writfold-plan/1\"\nname = \"detail\"\nsteps = [{steps}]\n"),
    )
}

fn fixture() -> TestResult<World> {
    let w = world(&[])?;
    define(&w.kb)?;
    put(
        &w.kb.join(".writfold/gates/review.md"),
        "Read each checklist item.\n",
    )?;
    put(
        &w.kb.join(".writfold/gates/review.toml"),
        &format!("{}instructions = \"review.md\"\n", checklist::REVIEW),
    )?;
    put(
        &w.kb.join(".writfold/gates/ready.toml"),
        &checklist::UNIT
            .replace("unit", "ready")
            .replace("src/ledger.rs", "src/batch.rs"),
    )?;
    let missing = w.kb.join(".writfold/gates/missing.toml");
    put(&missing, &checklist::UNIT.replace("unit", "missing"))?;
    template(
        &w,
        "\"done: unit\", \"skip: missing\", \"review: ready review missing\", \"later: unit\"",
    )?;
    plan(&w, "set")?;
    run_unit(&w)?;
    plan(&w, "advance")?;
    plan(&w, "skip")?;
    std::fs::remove_file(missing)?;
    gate(&w, "ready")?;
    cover(&w, "errors", Outcome::Supported, &["gate:unit"])?;
    cover(&w, "record", Outcome::Supported, &[STALL])?;
    cover(&w, "perf", Outcome::NotApplicable, &[])?;
    Ok(w)
}

#[test]
fn detailed_rows_preserve_default_and_expose_each_gate() -> TestResult {
    let w = fixture()?;
    let clean = status(&w, true, true)?;
    assert_eq!(clean["rows"][0]["flags"], json!([]));
    for more in clean["rows"][0]["more"].as_array().ok_or("more")? {
        assert_eq!(more["flags"], json!([]));
    }
    put(&w.code.join("src/ledger.rs"), "pub fn changed() {}\n")?;
    let default = status(&w, false, true)?;
    let detailed = status(&w, true, true)?;
    let rows = detailed["rows"].as_array().ok_or("rows")?;
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0]["flags"], json!(["stale"]));
    assert_eq!(rows[0]["more"][0]["label"], "why");
    assert_eq!(rows[0]["more"][1]["text"], "unit [stale] mechanical");
    assert_eq!(rows[0]["more"][1]["flags"], json!(["stale"]));
    assert_eq!(rows[0]["more"][1]["line"], 1);
    assert!(
        rows[0]["more"][1]["path"]
            .as_str()
            .is_some_and(|p| p.contains("/gates/"))
    );
    assert_eq!(rows[0]["more"][2]["label"], "done");
    assert_eq!(rows[1]["more"], json!([]));
    assert_current(&rows[2])?;
    assert_eq!(
        rows[3]["more"],
        json!([{
            "label": "gate", "text": "unit [later]", "status": "later",
            "path": null, "line": null, "flags": []
        }])
    );
    let counts = &detailed["trailer"]["counts"];
    assert_eq!(
        (
            counts["current"].as_u64(),
            counts["current_gates"].as_u64(),
            counts["current_owed"].as_u64()
        ),
        (Some(3), Some(3), Some(2))
    );
    let grammar = regex::Regex::new(r"^[^\s]+ \[([^\]]+)\](?: |$)")?;
    let mut stripped = detailed["rows"].clone();
    for row in stripped.as_array_mut().ok_or("rows")? {
        let more = row["more"].as_array_mut().ok_or("more")?;
        for line in more
            .iter()
            .filter(|m| m["label"] == "gate" || m["label"] == "item")
        {
            let captures = grammar
                .captures(line["text"].as_str().ok_or("text")?)
                .ok_or("grammar")?;
            assert_eq!(line["status"], captures[1]);
        }
        more.retain(|m| m["label"] != "gate" && m["label"] != "item");
    }
    assert_eq!(stripped, default["rows"]);
    assert_ne!(detailed["trailer"]["q"], default["trailer"]["q"]);
    same_items(&w, &rows[2])?;
    goldens(&w)?;
    Ok(())
}

fn assert_current(row: &Value) -> TestResult {
    let lines = row["more"].as_array().ok_or("more")?;
    assert_eq!(lines[0]["text"], "ready [satisfied] mechanical");
    assert_eq!(lines[0]["flags"], json!([]));
    assert_eq!(
        lines[1]["text"],
        "review [stale] checklist 1/2 required covered"
    );
    assert_eq!(lines[1]["flags"], json!(["stale"]));
    assert_eq!(lines[2]["status"], "stale");
    assert_eq!(lines[3]["status"], "supported");
    assert_eq!(lines[4]["status"], "not-applicable");
    assert_eq!(lines[5]["text"], "missing [undefined]");
    assert_eq!(lines[5]["flags"], json!(["undefined"]));
    assert_eq!(lines[5]["path"], Value::Null);
    assert_eq!(lines[6]["label"], "read");
    assert_eq!(lines[7]["label"], "next");
    Ok(())
}

fn same_items(w: &World, current: &Value) -> TestResult {
    let mut request = w.request(Verb::Gate(GateVerb::Status {
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    }));
    request.json = true;
    let (text, _) = w.run(&request)?;
    let gates: Value = serde_json::from_str(&text)?;
    let review = gates["rows"]
        .as_array()
        .ok_or("rows")?
        .iter()
        .find(|r| r["id"] == "review")
        .ok_or("review")?;
    let items = |row: &Value| -> TestResult<Vec<Value>> {
        Ok(row["more"]
            .as_array()
            .ok_or("more")?
            .iter()
            .filter(|m| m["label"] == "item")
            .cloned()
            .collect())
    };
    // Item continuations are `gate status`'s own, `status` key included.
    let actual = items(current)?;
    assert!(actual.iter().all(|i| i["status"].is_string()), "{actual:?}");
    assert_eq!(actual, items(review)?);
    Ok(())
}

fn goldens(w: &World) -> TestResult {
    for (json, name) in [
        (false, "plan-status-gates.txt"),
        (true, "plan-status-gates.json"),
    ] {
        let mut request = w.request(Verb::Plan(PlanVerb::Status {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
            gates: true,
        }));
        request.json = json;
        request.budget = writfold_kb::MAX_BUDGET;
        let (text, exit) = w.run(&request)?;
        golden(name, &text, exit)?;
    }
    Ok(())
}

#[test]
fn no_subject_omits_counts_and_complete_has_current_zero() -> TestResult {
    let w = fixture()?;
    let absent = status(&w, true, false)?;
    for key in ["current", "current_gates", "current_owed"] {
        assert!(absent["trailer"]["counts"].get(key).is_none());
    }
    assert!(absent["notes"].as_array().ok_or("notes")?.iter().any(|n| {
        n.as_str()
            .is_some_and(|s| s.contains("no subject repository"))
    }));
    plan(&w, "skip")?;
    plan(&w, "advance")?;
    let complete = status(&w, true, true)?;
    assert_eq!(complete["trailer"]["counts"]["current"], 0);
    assert!(complete["trailer"]["counts"].get("current_gates").is_none());
    assert!(complete["trailer"]["counts"].get("current_owed").is_none());
    Ok(())
}

#[test]
fn done_run_record_gate_uses_its_own_window() -> TestResult {
    let w = world(&[])?;
    put(
        &w.kb.join(".writfold/gates/backfilled.toml"),
        "schema = \"writfold-gate/1\"\nid = \"backfilled\"\nversion = 1\nclass = \"run-record\"\nrequire = [\"research.backfilled\"]\n",
    )?;
    template(&w, "\"first: backfilled\", \"second: backfilled\"")?;
    plan(&w, "set")?;
    let ticket = Write::Ticket(Ticket {
        ticket: "HBR-301".into(),
        title: "Backfill".into(),
        ..Ticket::default()
    });
    w.run(&w.request(Verb::Write(ticket)))?;
    gate(&w, "backfilled")?;
    plan(&w, "advance")?;
    let detail = status(&w, true, true)?;
    assert_eq!(
        detail["rows"][0]["more"][0]["text"],
        "backfilled [satisfied] run-record"
    );
    assert_eq!(detail["rows"][0]["flags"], json!([]));
    assert_eq!(detail["rows"][1]["more"][0]["status"], "stale");
    Ok(())
}
