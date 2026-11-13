//! Run positions are folded records, not gate judgments or estimates.
mod support;

use serde_json::Value;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::plan::PlanVerb;

const OPEN: &str = "run-20261108T090000Z-a1b2c3";
const COMPLETE: &str = "run-20261109T090000Z-a1b2c3";
const UNPLANNED: &str = "run-20261101T090000Z-a1b2c3";

fn call(w: &World, verb: Verb) -> TestResult<Value> {
    let mut request = w.request(verb);
    request.json = true;
    request.budget = writfold_kb::MAX_BUDGET;
    Ok(serde_json::from_str(&w.run(&request)?.0)?)
}

fn make_run(w: &World, run: &str) -> TestResult {
    let dir = w.kb.join("runs").join(run);
    std::fs::create_dir_all(&dir)?;
    let text = serde_json::json!({
        "run_id": run, "task": "Recorded plan position", "project": "harbor",
        "mode": "authoring", "state": "open", "created": "2026-11-08T09:00:00Z"
    });
    std::fs::write(dir.join("manifest.json"), text.to_string())?;
    call(
        w,
        Verb::Plan(PlanVerb::Set {
            template: Some("position".into()),
            from: None,
            run: Some(run.into()),
            repo: Some(w.code.clone()),
        }),
    )?;
    Ok(())
}

fn close_step(w: &World, run: &str, skip: bool) -> TestResult {
    let (run, repo) = (Some(run.into()), Some(w.code.clone()));
    let verb = if skip {
        PlanVerb::Skip {
            run,
            repo,
            reason: "not needed".into(),
            by: "owner".into(),
        }
    } else {
        PlanVerb::Advance { run, repo }
    };
    call(w, Verb::Plan(verb))?;
    Ok(())
}

fn positions() -> TestResult<World> {
    let w = world(&[])?;
    let plans = w.kb.join(".writfold/plans");
    std::fs::create_dir_all(&plans)?;
    std::fs::write(
        plans.join("position.toml"),
        "schema = \"writfold-plan/1\"\nname = \"position\"\nsteps = [\"first:\", \"second:\", \"third:\"]\n",
    )?;
    for run in [OPEN, COMPLETE] {
        make_run(&w, run)?;
        close_step(&w, run, false)?;
        close_step(&w, run, true)?;
    }
    close_step(&w, COMPLETE, false)?;
    Ok(w)
}

fn row<'a>(answer: &'a Value, id: &str) -> TestResult<&'a Value> {
    answer["rows"]
        .as_array()
        .ok_or("rows")?
        .iter()
        .find(|row| row["id"] == id)
        .ok_or_else(|| format!("missing {id}").into())
}

fn plan_line(row: &Value) -> Option<&Value> {
    row["more"]
        .as_array()?
        .iter()
        .find(|m| m["label"] == "plan")
}

#[test]
fn run_positions_match_plan_counts_for_open_complete_and_unplanned_runs() -> TestResult {
    let w = positions()?;
    let answer = call(
        &w,
        Verb::Runs {
            open: false,
            project: None,
            plan: true,
        },
    )?;
    assert_eq!(answer["trailer"]["counts"]["planned"], 2);
    assert!(plan_line(row(&answer, UNPLANNED)?).is_none());
    for (run, expected) in [
        (OPEN, "step 3/3 done=1 skipped=1 current=position#1/third"),
        (COMPLETE, "complete steps=3 done=2 skipped=1"),
    ] {
        let line = plan_line(row(&answer, run)?).ok_or("plan line")?;
        assert_eq!(line["text"], expected);
        let status = call(
            &w,
            Verb::Plan(PlanVerb::Status {
                run: Some(run.into()),
                repo: Some(w.code.clone()),
                gates: false,
            }),
        )?;
        let counts = &status["trailer"]["counts"];
        for name in ["done", "skipped"] {
            assert!(expected.contains(&format!("{name}={}", counts[name])));
        }
        assert_eq!(counts["steps"], 3);
        if run == OPEN {
            assert!(
                line["path"]
                    .as_str()
                    .is_some_and(|p| p.ends_with("splice-position.1.json"))
            );
            assert_eq!(line["line"], 1);
        } else {
            assert!(line["path"].is_null());
            assert!(line["line"].is_null());
        }
    }
    let default = call(
        &w,
        Verb::Runs {
            open: false,
            project: None,
            plan: false,
        },
    )?;
    assert!(
        default["rows"]
            .as_array()
            .ok_or("rows")?
            .iter()
            .all(|r| plan_line(r).is_none())
    );
    assert!(default["trailer"]["counts"].get("planned").is_none());
    Ok(())
}

#[test]
fn invalid_plan_events_remain_visible_in_the_run_position() -> TestResult {
    let w = positions()?;
    std::fs::write(
        w.kb.join(format!("runs/{OPEN}/plan/broken.json")),
        "not an event",
    )?;
    let answer = call(
        &w,
        Verb::Runs {
            open: true,
            project: Some("harbor".into()),
            plan: true,
        },
    )?;
    let line = plan_line(row(&answer, OPEN)?).ok_or("plan line")?;
    assert_eq!(line["flags"], serde_json::json!(["invalid"]));
    assert_eq!(
        line["text"],
        "step 3/3 done=1 skipped=1 current=position#1/third"
    );
    Ok(())
}
