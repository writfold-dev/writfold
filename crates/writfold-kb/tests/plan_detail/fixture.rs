use super::*;

pub(super) fn plan(w: &World, action: &str) -> TestResult {
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

pub(super) fn status(w: &World, gates: bool, subject: bool) -> TestResult<Value> {
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
    let mut answer: Value = serde_json::from_str(&text)?;
    // Structure has its own nested contract; retain the gate-only comparison.
    for row in answer["rows"].as_array_mut().ok_or("rows")? {
        row["more"]
            .as_array_mut()
            .ok_or("more")?
            .retain(|m| m["label"] != "splice" && m["label"] != "under");
    }
    Ok(answer)
}

pub(super) fn gate(w: &World, id: &str) -> TestResult {
    let (text, exit) = w.run(&w.request(Verb::Gate(GateVerb::Run {
        id: id.into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    })))?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

pub(super) fn template(w: &World, steps: &str) -> TestResult {
    put(
        &w.kb.join(".writfold/plans/detail.toml"),
        &format!("schema = \"writfold-plan/1\"\nname = \"detail\"\nsteps = [{steps}]\n"),
    )
}

pub(super) fn fixture() -> TestResult<World> {
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
