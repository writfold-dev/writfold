use super::*;
use writfold_kb::plan::{PlanVerb, event, fold};

pub(super) fn record(w: &World) -> TestResult {
    let dir = w.kb.join(".writfold/plans");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("timeline.toml"),
        "schema = \"writfold-plan/1\"\nname = \"timeline\"\nsteps = [\"first:\", \"second:\", \"later:\"]\n",
    )?;
    for verb in [
        PlanVerb::Set {
            template: Some("timeline".into()),
            from: None,
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
        PlanVerb::Advance {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
        PlanVerb::Skip {
            reason: "not needed".into(),
            by: "owner".into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    ] {
        let mut request = w.request(Verb::Plan(verb));
        request.now = Some(1_793_620_800);
        let (text, exit) = w.run(&request)?;
        assert_eq!(exit, 0, "{text}");
    }
    Ok(())
}

#[test]
fn plan_events_keep_recorded_times_paths_and_filename_ties() -> TestResult {
    let w = run_world()?;
    let mut request = w.request(timeline(RUN));
    request.json = true;
    let (text, _) = w.run(&request)?;
    let body: Value = serde_json::from_str(&text)?;
    let rows: Vec<_> = body["rows"]
        .as_array()
        .ok_or("rows")?
        .iter()
        .filter(|r| r["kind"] == "plan")
        .collect();
    assert_eq!(
        rows.iter()
            .map(|r| r["status"].as_str())
            .collect::<Vec<_>>(),
        [Some("done"), Some("skipped"), Some("spliced")]
    );
    for row in &rows {
        assert!(
            row["text"]
                .as_str()
                .ok_or("text")?
                .starts_with("2026-11-02T12:00:00Z ")
        );
        assert_eq!(row["line"], 1);
        assert!(w.kb.join(row["path"].as_str().ok_or("path")?).is_file());
        assert!(
            row["text"]
                .as_str()
                .ok_or("text")?
                .contains(row["id"].as_str().ok_or("id")?)
        );
    }
    assert_eq!(
        rows[1]["text"],
        "2026-11-02T12:00:00Z skipped timeline#1/second (by owner) not needed"
    );
    Ok(())
}

#[test]
fn events_rejected_by_the_fold_never_enter_timeline() -> TestResult {
    let w = run_world()?;
    let mut request = w.request(timeline(RUN));
    request.json = true;
    let (before, _) = w.run(&request)?;
    let before: Value = serde_json::from_str(&before)?;
    let graph = fold::read(&w.kb, &format!("runs/{RUN}"), RUN);
    let mut splice = graph.splices[0].1.clone();
    splice.parent = "missing#1/parent".into();
    let event = event::Event::Splice(splice);
    let dir = w.kb.join(format!("runs/{RUN}/plan"));
    std::fs::write(dir.join(event::name(&event)), event::bytes(RUN, &event))?;
    std::fs::write(dir.join("done-altered.json"), "{}")?;
    assert_eq!(
        fold::read(&w.kb, &format!("runs/{RUN}"), RUN).invalid.len(),
        2
    );
    let (after, _) = w.run(&request)?;
    let after: Value = serde_json::from_str(&after)?;
    assert_eq!(before["rows"], after["rows"]);
    Ok(())
}
