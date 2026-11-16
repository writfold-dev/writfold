//! `runs --idle AGE`: open runs whose last recorded write (created, the
//! last checkpoint, or a stamped file under the run) is older than AGE.
mod support;

use serde_json::{Value, json};
use support::{TestResult, World, world};
use writfold_kb::Verb;

const FIXTURE: &str = "run-20261101T090000Z-a1b2c3";

fn run(w: &World, id: &str, project: &str, state: &str, created: &str) -> TestResult {
    let dir = w.kb.join("runs").join(id);
    std::fs::create_dir_all(&dir)?;
    let manifest = json!({"run_id": id, "task": "t", "project": project, "mode": "authoring",
        "state": state, "created": created});
    std::fs::write(dir.join("manifest.json"), manifest.to_string())?;
    Ok(())
}

fn idle(w: &World, age: &str, project: Option<&str>) -> TestResult<(Vec<(String, String)>, u8)> {
    let seconds = writfold_kb::verbs::runs::idle_age(age).ok_or("age")?;
    let mut request = w.request(Verb::RunsIdle {
        age: seconds,
        age_text: age.into(),
        project: project.map(str::to_owned),
        plan: false,
    });
    request.json = true;
    request.budget = writfold_kb::MAX_BUDGET;
    let output = writfold_kb::run(&request)?;
    let body: Value = serde_json::from_str(&output.text)?;
    let mut rows = Vec::new();
    for row in body["rows"].as_array().ok_or("rows")? {
        let more = row["more"].as_array().ok_or("more")?;
        let idle = more
            .iter()
            .find(|m| m["label"] == "idle")
            .ok_or("no idle line")?;
        rows.push((
            row["id"].as_str().ok_or("id")?.to_owned(),
            idle["text"].as_str().ok_or("text")?.to_owned(),
        ));
    }
    assert_eq!(
        body["trailer"]["counts"]["idle"].as_u64(),
        Some(rows.len() as u64)
    );
    Ok((rows, output.exit))
}

#[test]
fn the_last_checkpoint_and_the_boundary_second() -> TestResult {
    let w = world(&[])?;
    // The fixture run's last checkpoint is 2026-11-03T16:00:00Z, 7d 8h
    // before the test clock.
    let (rows, exit) = idle(&w, "7d", None)?;
    assert_eq!(exit, 0);
    assert_eq!(
        rows,
        [(
            FIXTURE.to_owned(),
            "7d since 2026-11-03T16:00:00Z (checkpoint)".to_owned()
        )]
    );
    assert_eq!(idle(&w, "633600s", None)?.0.len(), 0);
    assert_eq!(idle(&w, "633599s", None)?.0.len(), 1);
    let (rows, exit) = idle(&w, "8d", None)?;
    assert_eq!((rows.len(), exit), (0, 2));
    Ok(())
}

#[test]
fn stamped_files_closed_runs_projects_and_created() -> TestResult {
    let w = world(&[])?;
    let covered = "run-20261101T000000Z-c0c0c0";
    run(&w, covered, "harbor", "open", "2026-11-01T00:00:00Z")?;
    let covers = w.kb.join("runs").join(covered).join("covers");
    std::fs::create_dir_all(&covers)?;
    std::fs::write(
        covers.join("review.item-20261110T000000Z-a1b2c3.json"),
        "{}\n",
    )?;
    let quiet = "run-20261102T000000Z-d0d0d0";
    run(&w, quiet, "lantern", "open", "2026-11-02T00:00:00Z")?;
    let closed = "run-20261001T000000Z-e0e0e0";
    run(&w, closed, "harbor", "closed", "2026-10-01T00:00:00Z")?;
    let (rows, _) = idle(&w, "2d", None)?;
    let ids: Vec<&str> = rows.iter().map(|(id, _)| id.as_str()).collect();
    // Oldest first; the cover row keeps its run from being idle; a
    // closed run is never listed.
    assert_eq!(ids, [quiet, FIXTURE]);
    assert_eq!(rows[0].1, "9d since 2026-11-02T00:00:00Z (created)");
    let (rows, _) = idle(&w, "2d", Some("lantern"))?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, quiet);
    let (rows, _) = idle(&w, "12h", None)?;
    let covered_row = rows
        .iter()
        .find(|(id, _)| id == covered)
        .ok_or("covered run")?;
    assert_eq!(covered_row.1, "1d since 2026-11-10T00:00:00Z (cover)");
    Ok(())
}
