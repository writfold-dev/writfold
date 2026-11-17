//! Detailed structure is emitted by the fold, including static includes.
mod support;
use serde_json::Value;
use support::{TestResult, World, world};
use writfold_kb::{
    Verb,
    plan::{PlanVerb, fold},
};
const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn template(w: &World, name: &str, steps: &str) -> TestResult {
    let dir = w.kb.join(".writfold/plans");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join(format!("{name}.toml")),
        format!("schema = \"writfold-plan/1\"\nname = \"{name}\"\nsteps = [{steps}]\n"),
    )?;
    Ok(())
}

fn fixture() -> TestResult<World> {
    let w = world(&[])?;
    template(&w, "change", "\"review: use=notes\", \"deliver:\"")?;
    template(&w, "notes", "\"read:\", \"record:\"")?;
    template(&w, "check", "\"first:\", \"second:\"")?;
    for verb in [
        PlanVerb::Set {
            template: Some("change".into()),
            from: None,
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
        PlanVerb::Use {
            template: "check".into(),
            again: false,
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    ] {
        let (text, exit) = w.run(&w.request(Verb::Plan(verb)))?;
        assert_eq!(exit, 0, "{text}");
    }
    Ok(w)
}

#[test]
fn nested_structure_names_exact_parents_and_template_uses() -> TestResult {
    let w = fixture()?;
    let graph = fold::read(&w.kb, &format!("runs/{RUN}"), RUN);
    let mut request = w.request(Verb::Plan(PlanVerb::Status {
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
        gates: true,
    }));
    request.json = true;
    let (text, _) = w.run(&request)?;
    let value: Value = serde_json::from_str(&text)?;
    let rows = value["rows"].as_array().ok_or("rows")?;
    // Six steps, then the plan's summary row (v0.6.0-dev.3).
    assert_eq!(rows.len(), 7);
    for (row, step) in rows.iter().zip(&graph.steps) {
        let usage = &graph.uses[step.usage];
        let more = row["more"].as_array().ok_or("more")?;
        let splice = &more[0];
        assert_eq!(splice["label"], "splice");
        assert_eq!(splice["path"], step.splice);
        assert_eq!(splice["line"], 1);
        assert_eq!(splice["flags"], serde_json::json!([]));
        assert!(splice.get("status").is_none());
        if !usage.parent.is_empty() {
            assert_eq!(more[1]["label"], "under");
            assert_eq!(more[1]["text"], usage.parent);
            assert!(rows.iter().any(|r| r["id"] == more[1]["text"]));
            assert!(more[1]["path"].is_null());
        }
        let grouped: Vec<_> = rows
            .iter()
            .filter(|r| r["more"][0]["text"] == splice["text"])
            .map(|r| r["id"].as_str().ok_or("id"))
            .collect::<Result<_, _>>()?;
        let expected: Vec<_> = graph
            .steps
            .iter()
            .filter(|s| s.usage == step.usage)
            .map(|s| s.path.as_str())
            .collect();
        assert_eq!(grouped, expected);
    }
    assert_eq!(rows[0]["more"][1]["text"], "change#1/review/notes#1/read");
    Ok(())
}
